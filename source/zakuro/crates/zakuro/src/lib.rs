//! Zakuro's frontend, a window showing the library of games or a game with
//! a menu over it, and the loop that drives the emulated console one frame
//! at a time.

mod audio;
mod banners;
mod camera;
mod cli;
mod console;
mod downloads;
mod feet;
mod gamepad;
mod gui;
mod hdtext;
mod input;
mod library;
mod lz;
mod menus;
mod overlays;
mod present;
mod recompile;
mod report;
mod settings;
mod sounds;
mod theme;
mod update;
mod zig;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The time without any use of the mouse after which the cursor is hidden while a game runs.
const CURSOR_HIDES_AFTER: Duration = Duration::from_secs(3);

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use zakuro_common::Screen;
use zakuro_core::{loader, Config, FrameOutcome, System};
use zakuro_gpu::{layout, GpuScreen, Overlay, PresentError, RendererKind, ScreenImage};

use gui::Gui;
use input::Keyboard;
use library::Library;
use menus::{Action, Menus};
use overlays::{Scene, ShownButtons, BOTTOM_OVER_CORNER, BOTTOM_OVER_EDGE, BOTTOM_OVER_WIDTH, PIECE_FADE, PIECE_SHADOW};
use present::Backend;
use recompile::{Job, Stage, Toolchain};
use settings::{CameraSource, Renderer, Screens, Settings};

pub use zakuro_core::memory::Memory;
pub use zakuro_core::recompiled::Linked;

/// what a program built on Zakuro shows over its game, worked out from the
/// game's memory: a line kept in a corner, and a notice shown for a moment.
/// a recompiled game can show what the game keeps hidden this way, its
/// alignment, a timer, without drawing into the game's own screens.
pub trait GameOverlay {
    /// called after each frame the game runs, with its memory.
    fn frame(&mut self, memory: &mut Memory) -> OverlayText;
}

/// what a GameOverlay shows after a frame.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct OverlayText {
    /// a line in the bottom right corner while it is some.
    pub corner: Option<String>,
    /// a notice at the top for a moment, as a saved state's.
    pub notice: Option<String>,
}

/// runs the emulator as the command line says, on recompiled code linked
/// into the program when there is some.
pub fn run(linked: Option<Linked>) {
    run_with(linked, None)
}

/// runs the emulator as run does, with an overlay over the game.
pub fn run_with(linked: Option<Linked>, overlay: Option<Box<dyn GameOverlay>>) {
    console::attach();
    report::init();
    if update::installable() {
        update::tidy();
    }

    let options = match cli::parse() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("zakuro: {message}");
            std::process::exit(2);
        }
    };
    let mut settings = Settings::load();
    console::show(settings.show_console);
    // the console's own number, made the first time and kept, which tells
    // it apart from others over local wireless
    if settings.console_id == 0 {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.as_nanos()));
        settings.console_id = hasher.finish() | 1;
        settings.save();
    }
    if let Some(layout) = options.layout.filter(|_| !settings.original) {
        settings.layout = layout;
    }
    if let Some(shape) = options.top_shape.filter(|_| !settings.original) {
        settings.top_shape = shape;
    }

    let data_dir = options.data.clone().map(PathBuf::from).or_else(default_data_dir);
    if let Some(dir) = &data_dir {
        report::to_file(dir);
        bring_saves(dir);
    }

    let mut app = App {
        keyboard: Keyboard::new(settings.keys.clone()),
        gamepads: gamepad::Gamepads::new(settings.pad.clone()),
        scale: options.scale.unwrap_or(settings.scale).max(1),
        layout: settings.layout,
        bottom_over: false,
        bottom_over_key: false,
        pad_pointer: overlays::PadPointer::default(),
        over_touched_at: None,
        touched_scene: Scene::Plain,
        over_touch_start: None,
        unit_view_from: None,
        scripted_tap: None,
        pending_overview: None,
        restore_inventory: false,
        closing_by_user: false,
        quiet_over: false,
        kept_scene: None,
        bottom_sounds: sounds::BottomScreen::default(),
        scene: Scene::Plain,
        placed: None,
        last_pieces: None,
        last_banners: None,
        banner_textures: overlays::BannerTextures::default(),
        updatable: update::installable(),
        update_check: update::installable().then(update::check_in_background),
        update_offer: None,
        update_install: None,
        buttons: None,
        last_input: (zakuro_core::services::hid::PadState::empty(), false),
        holding_back_cancel: false,
        any_button_pressed: false,
        top_shape: settings.top_shape,
        options,
        settings,
        linked,
        overlay,
        overlay_corner: None,
        library: Library::default(),
        data_dir,
        game: None,
        menus: {
            let mut menus = Menus::default();
            menus.slot = 1;
            menus
        },
        jobs: Vec::new(),
        gui: None,
        backend: None,
        window: None,
        audio: None,
        mouse_down: false,
        mouse_used_at: Instant::now(),
        tilting_from: None,
        cursor: (0.0, 0.0),
        last_title_update: Instant::now(),
        next_frame: Instant::now(),
        skipped: 0,
        shown: 0,
        spent: Spent::default(),
        paused: false,
        fast_forward_key: false,
        fast_forwarding: false,
        next_shown: Instant::now(),
        stop: false,
        camera: None,
        room_server: None,
        room: None,
        dlp_guest: None,
        dlp_own: None,
        wireless_reboot: None,
        maximized_before_full_screen: false,
        resize_after_full_screen: false,
    };

    if let Some(frames) = app.options.headless {
        app.start();
        if let Some(game) = &mut app.game {
            run_headless(&mut game.system, frames);
        }
        return;
    }

    if let Some(source) = app.library_source() {
        app.library.scan(source);
    }
    app.audio = if app.options.mute {
        None
    } else {
        audio::Audio::open(zakuro_core::AUDIO_SAMPLE_RATE).inspect_err(|error| log::warn!("no sound, {error}")).ok()
    };
    app.apply_volume();

    let event_loop = EventLoop::new().expect("create an event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("zakuro: {error}");
    }
}

/// opens a folder in the system's file manager.
fn show_folder(folder: &Path) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    match std::process::Command::new(program).arg(folder).spawn() {
        // waited for so it leaves nothing behind when it closes
        Ok(mut child) => drop(std::thread::spawn(move || child.wait())),
        Err(error) => log::warn!("showing {}: {error}", folder.display()),
    }
}

/// the system's place for a program's data.
fn default_data_dir() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from);
    let dir = if cfg!(windows) {
        var("APPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library/Application Support"))
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|home| home.join(".local/share")))
    };
    dir.map(|dir| dir.join("zakuro"))
}

/// how long ago a state was saved, as a person would say it.
fn ago(time: std::time::SystemTime) -> String {
    let seconds = time.elapsed().map_or(0, |elapsed| elapsed.as_secs());
    match seconds {
        0..60 => "saved just now".into(),
        60..120 => "saved a minute ago".into(),
        120..3600 => format!("saved {} minutes ago", seconds / 60),
        3600..7200 => "saved an hour ago".into(),
        7200..86400 => format!("saved {} hours ago", seconds / 3600),
        86400..172800 => "saved yesterday".into(),
        _ => format!("saved {} days ago", seconds / 86400),
    }
}

/// saves Zakuro kept in the working directory before it had a place for
/// them, copied there the first time. the old ones stay as they were.
fn bring_saves(data: &Path) {
    let (old, new) = (Path::new("user"), data.join("user"));
    if new.exists() || !old.is_dir() {
        return;
    }
    // a copy cut short is left under another name, and tried again
    let partial = data.join("user.copying");
    let copied = std::fs::remove_dir_all(&partial)
        .or_else(|error| if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) })
        .and_then(|()| copy_dir(old, &partial))
        .and_then(|()| std::fs::rename(&partial, &new));
    match copied {
        Ok(()) => log::info!("copied the saves in ./user to {}, the old ones stay as they were", new.display()),
        Err(error) => log::warn!("could not copy the saves in ./user to {}, {error}", new.display()),
    }
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// builds a System with no title loaded and both screens filled with a
/// solid color, to check the presentation path (framebuffer config -> guest
/// memory -> window) in isolation from any title's boot logic.
fn paint_test_pattern(config: Config) -> System {
    use zakuro_core::memory::{MemoryRegion, MemoryState, Permission};

    let mut system = System::new(config);

    // ABGR in memory for format 0 (Rgba8), red = 00 00 00 FF?
    let paint = |system: &mut System, screen: Screen, rgba: [u8; 4]| {
        let width = screen.width();
        let height = screen.height();
        let stride = height * 4;
        let size = stride * width;

        let block = system
            .memory
            .phys
            .allocate(MemoryRegion::Base, size)
            .expect("allocate test pattern framebuffer");

        let vaddr = zakuro_core::services::gsp::physical_to_virtual(system, block.addr);
        system.memory.map(
            vaddr,
            block.addr,
            size,
            Permission::READ | Permission::WRITE,
            MemoryState::Shared,
        );

        let mut pixel = [0u8; 4];
        pixel[3] = rgba[0]; // r
        pixel[2] = rgba[1]; // g
        pixel[1] = rgba[2]; // b
        pixel[0] = rgba[3]; // a
        let mut data = Vec::with_capacity(size as usize);
        for _ in 0..(size / 4) {
            data.extend_from_slice(&pixel);
        }
        system.memory.write_physical(block.addr, &data);

        let index = match screen {
            Screen::Top => 0,
            Screen::Bottom => 1,
        };
        system.set_framebuffer(index, 0, block.addr, block.addr, stride, 0);
    };

    paint(&mut system, Screen::Top, [220, 40, 40, 255]); // red
    paint(&mut system, Screen::Bottom, [40, 90, 220, 255]); // blue
    system
}

fn run_headless(system: &mut System, frames: u64) {
    let start = Instant::now();
    let mut outcome = FrameOutcome::Completed;
    let mut ran = 0;
    for _ in 0..frames {
        outcome = system.run_frame();
        ran += 1;
        if outcome != FrameOutcome::Completed {
            break;
        }
    }
    let elapsed = start.elapsed();
    println!("{outcome:?} after {ran} frames in {elapsed:.2?}");
    println!(
        "{:.1} MIPS, {}",
        system.cpu.cycles as f64 / elapsed.as_secs_f64() / 1e6,
        system.status_line()
    );
    if !system.fatal_errors.is_empty() {
        println!("fatal errors: {}", system.fatal_errors.join("; "));
    }
    for (thread, pc, hits) in system.hot_spots(10) {
        println!("  hot {thread:<8} 0x{pc:08X} {hits}");
    }
}

/// a game being played.
struct Running {
    system: System,
    path: PathBuf,
    name: String,
    /// the inputs written down, ZAKURO_RECORD=file, for playing the run
    /// back.
    recorder: Option<zakuro_core::replay::Recorder>,
    /// the inputs a recording wrote down, ZAKURO_REPLAY=file, played back in
    /// place of the keyboard's and the controllers', and the frames run.
    replay: Option<zakuro_core::replay::Replay>,
    frame: u64,
    frames: u32,
    /// The counts of pictures at the last measure of the frame rate.
    pictures: zakuro_core::Pictures,
    counting_since: Instant,
    fps: f32,
    /// The pictures per second that come from the game and the pictures per second that are interpolated.
    shown: (f32, f32),
}

impl Running {
    fn new(system: System, path: PathBuf, name: String) -> Running {
        Running {
            system,
            path,
            name,
            recorder: None,
            replay: None,
            frame: 0,
            frames: 0,
            pictures: zakuro_core::Pictures::default(),
            counting_since: Instant::now(),
            fps: 0.0,
            shown: (0.0, 0.0),
        }
    }

    fn count_frame(&mut self) {
        self.frames += 1;
        let elapsed = self.counting_since.elapsed();
        if elapsed >= Duration::from_millis(500) {
            let seconds = elapsed.as_secs_f32();
            self.fps = self.frames as f32 / seconds;
            let pictures = self.system.pictures();
            let rate = |now: u64, then: u64| now.saturating_sub(then) as f32 / seconds;
            self.shown = (rate(pictures.from_game, self.pictures.from_game), rate(pictures.interpolated, self.pictures.interpolated));
            self.pictures = pictures;
            self.frames = 0;
            self.counting_since = Instant::now();
        }
    }
}

struct App {
    options: cli::Options,
    settings: Settings,
    linked: Option<Linked>,
    /// what the program built on Zakuro shows over the game, and the line it
    /// keeps in the corner now.
    overlay: Option<Box<dyn GameOverlay>>,
    overlay_corner: Option<String>,
    data_dir: Option<PathBuf>,
    game: Option<Running>,
    library: Library,
    menus: Menus,
    jobs: Vec<Job>,
    gui: Option<Gui>,
    // backend must be declared (and therefore dropped) before window, Rust
    // drops struct fields in declaration order, and the GL surface's Drop
    // calls eglDestroySurface, which on Wayland does a protocol round-trip
    // against the window's wl_surface.
    backend: Option<Backend>,
    window: Option<Window>,
    keyboard: Keyboard,
    gamepads: gamepad::Gamepads,
    audio: Option<audio::Audio>,
    /// the window's size, times the console's.
    scale: u32,
    /// how the screens are arranged.
    layout: Screens,
    /// Whether the bottom screen is laid over the top screen right now.
    bottom_over: bool,
    /// Whether the bottom screen is wanted over the top screen, by the key, the button or a scripted step.
    bottom_over_key: bool,
    /// The pointer on the bottom screen that is steered with the gamepad.
    pad_pointer: overlays::PadPointer,
    /// The moment of the last touch while the bottom screen was laid over the top screen.
    over_touched_at: Option<Instant>,
    /// The scene that was shown at the last touch while the bottom screen was laid over the top screen.
    touched_scene: Scene,
    /// The point where the current touch began while the bottom screen is over the top screen, and whether the inventory was shown at that moment.
    over_touch_start: Option<((u16, u16), bool)>,
    /// The frame where the switch to the view of a unit began, and the last frame in which the game talked.
    unit_view_from: Option<(u64, u64)>,
    /// A touch that the launcher presses on its own, with the point and the number of frames that are left.
    scripted_tap: Option<((u16, u16), u8)>,
    /// The number of frames that are already waited for the overview of the map to show.
    pending_overview: Option<u8>,
    /// Whether the inventory tab is tapped again once the overview of the map closes.
    restore_inventory: bool,
    /// Whether the bottom screen is closing because the user asked for it, which decides if the close sound plays.
    closing_by_user: bool,
    /// Whether the bottom screen opens without the open sound.
    quiet_over: bool,
    /// The scene that keeps the bottom screen over the top screen, see `keeps_bottom_over`.
    kept_scene: Option<Scene>,
    /// The sounds that play when the bottom screen opens and closes.
    bottom_sounds: sounds::BottomScreen,
    /// The scene that the game shows.
    scene: Scene,
    /// The screen layout that was last given to the backend.
    placed: Option<zakuro_gpu::ScreenLayout>,
    /// The pieces of the last layout, kept so that they can fade out after the layout changed.
    last_pieces: Option<&'static [zakuro_gpu::Piece]>,
    /// The banners that were read last from the bottom screen.
    last_banners: Option<banners::Read>,
    /// The textures of the banners.
    banner_textures: overlays::BannerTextures,
    /// Whether this build is able to install updates.
    updatable: bool,
    /// The channel that delivers the result of the check for an update, while the check runs.
    update_check: Option<std::sync::mpsc::Receiver<Option<update::Release>>>,
    /// The release that is on offer as an update.
    update_offer: Option<update::Release>,
    /// The update that is being installed.
    update_install: Option<update::Installing>,
    /// The buttons of a battle that are shown or fading.
    buttons: Option<ShownButtons>,
    /// The buttons that were held, and whether the screen was touched, in the previous frame of the game.
    last_input: (zakuro_core::services::hid::PadState, bool),
    /// Whether the B button is held back from the game until it is released, after it closed the bottom screen.
    holding_back_cancel: bool,
    /// Whether a key or a button of the gamepad was pressed since the last frame of the game.
    any_button_pressed: bool,
    /// how wide the top screen shows.
    top_shape: settings::TopShape,
    /// the left button is down, and where the pointer is, in window pixels.
    mouse_down: bool,
    /// The moment of the last use of the mouse, which hides the cursor once it is long enough ago.
    mouse_used_at: Instant,
    /// where the pointer was when the right button went down, which tilts
    /// the console while held.
    tilting_from: Option<(f32, f32)>,
    cursor: (f32, f32),
    last_title_update: Instant,
    /// when the next frame is due. frames run to a schedule rather than one
    /// after another, so that a slow one is made up by those after it and
    /// the sound, made as the console runs, keeps up.
    next_frame: Instant,
    /// frames in a row not shown while catching up.
    skipped: u32,
    /// frames shown since the title was last updated.
    shown: u32,
    /// how long emulating and showing frames took since the title was last
    /// updated, for the frame rate's log.
    spent: Spent,
    /// stopped with F1, without the menu.
    paused: bool,
    /// Tab is held down, for fast forward.
    fast_forward_key: bool,
    /// the game runs as fast as it can, its sound left out.
    fast_forwarding: bool,
    /// when fast forward shows its next frame, on a grid at the console's
    /// frame rate.
    next_shown: Instant,
    stop: bool,
    /// what the running game's cameras were given to see, to give them
    /// another only when the settings change it.
    camera: Option<(CameraSource, Option<PathBuf>)>,
    /// the room this program holds open, and the one it is in.
    room_server: Option<zakuro_core::room::Server>,
    room: Option<zakuro_core::room::Connection>,
    /// Download Play as a guest, while in a room with no game running.
    dlp_guest: Option<zakuro_core::download_play::Guest>,
    /// the player's own copy of the child joined for, played in place of
    /// one sent.
    dlp_own: Option<PathBuf>,
    /// the network the next game joins as it starts, the host's of a
    /// Download Play child.
    wireless_reboot: Option<[u8; 16]>,
    /// the window was maximized when it went full screen, which leaving
    /// full screen brings back.
    maximized_before_full_screen: bool,
    /// the layout or the scale changed while the window was full screen,
    /// which kept its size, so leaving full screen fits the window to them.
    resize_after_full_screen: bool,
}

/// how long each step of the loop took, its mean and its longest.
#[derive(Default)]
struct Spent {
    emulating: Times,
    showing: Times,
}

#[derive(Default)]
struct Times {
    total: Duration,
    longest: Duration,
    count: u32,
}

impl Times {
    fn add(&mut self, time: Duration) {
        self.total += time;
        self.longest = self.longest.max(time);
        self.count += 1;
    }
}

impl std::fmt::Display for Times {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mean = self.total.as_secs_f64() * 1e3 / self.count.max(1) as f64;
        write!(f, "{mean:.1} ms, at most {:.1}", self.longest.as_secs_f64() * 1e3)
    }
}

/// one 3DS frame, about 16.71 ms.
const FRAME_TIME: Duration =
    Duration::from_nanos(zakuro_core::CYCLES_PER_FRAME * 1_000_000_000 / zakuro_core::kernel::thread::CPU_CLOCK_HZ);
/// how far behind the schedule may fall before it starts over instead of
/// running fast to catch up, after a pause say.
const CATCH_UP_LIMIT: Duration = Duration::from_millis(200);
/// frames that may go unshown in a row while catching up.
const MAX_SKIPPED: u32 = 4;

/// whether a frame goes unshown, the game running behind the schedule with
/// a presenter that waits for the display, and fewer than MAX_SKIPPED
/// unshown before it.
fn skips_showing(playing: bool, behind: bool, waits: bool, skipped: u32) -> bool {
    playing && behind && waits && skipped < MAX_SKIPPED
}

/// what the player is told when the 3D can't be drawn on the GPU.
const GPU_FALLBACK: &str = "The 3D can't be drawn on the GPU, as Vulkan isn't working on this computer, so it's drawn in software. \
Games run much slower that way, and texture packs are left out. Updating the graphics driver usually fixes it; the log says \
what went wrong. To draw in software without this message, turn off \"Draw the 3D on the GPU\" in Settings, Graphics.";

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // Without a game to start, the window is made 30 percent larger for the library.
        let size = match (self.options.rom.is_none(), self.window_size()) {
            (true, winit::dpi::Size::Logical(size)) => winit::dpi::LogicalSize::new(size.width * 1.3, size.height * 1.3).into(),
            (_, size) => size,
        };
        let attributes = Window::default_attributes().with_title("Zakuro").with_inner_size(size);
        let renderer = self.options.renderer.unwrap_or(match self.settings.renderer {
            Renderer::Vulkan => RendererKind::Vulkan,
            Renderer::OpenGl => RendererKind::OpenGl,
        });

        let backend = Backend::create(event_loop, attributes.clone(), renderer).or_else(|error| {
            if renderer != RendererKind::Vulkan {
                return Err(error);
            }
            // a machine without a working Vulkan driver still gets a window
            log::warn!("could not start the Vulkan backend, {error}, presenting with OpenGL instead");
            Backend::create(event_loop, attributes, RendererKind::OpenGl)
        });
        match backend {
            Ok((window, mut backend)) => {
                log::info!("presenting with the {} backend", backend.name());
                backend.set_layout(self.shown_layout());
                backend.set_top_width(self.settings.top_shape.width());
                backend.set_scaling(self.settings.filter.screens(), self.settings.integer_scale);
                backend.set_stereo(self.settings.stereo());
                self.gui = Some(Gui::new(&window));
                self.window = Some(window);
                self.backend = Some(backend);
                // the game the command line names starts on the device the
                // presenter may lend the renderer, once there is one
                self.start();
            }
            Err(error) => {
                eprintln!("zakuro: could not start the {renderer:?} backend: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Tab is fast forward while a game runs, not a key for moving the
        // interface's focus around
        let (tab, repeat) = match &event {
            WindowEvent::KeyboardInput { event, .. } => (event.physical_key == PhysicalKey::Code(KeyCode::Tab), event.repeat),
            _ => (false, false),
        };
        let in_game = self.game.as_ref().is_some_and(|game| game.system.keyboard_request().is_none())
            && !self.menus.menu_open
            && !self.menus.settings_open;
        // a Tab held down keeps repeating over a message too
        let withheld = tab && (in_game || (repeat && self.game.is_some()));
        let consumed = match (&mut self.gui, &self.window) {
            (Some(gui), Some(window)) if !withheld => gui.event(window, &event),
            _ => false,
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            // keys let go while another window has the keyboard never come
            WindowEvent::Focused(false) => {
                self.fast_forward_key = false;
                self.keyboard.release();
            }
            WindowEvent::Resized(size) => {
                if let Some(backend) = &mut self.backend {
                    backend.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.key(event, consumed),
            WindowEvent::MouseWheel { .. } => self.mouse_used_at = Instant::now(),
            WindowEvent::MouseInput { state, button, .. } => {
                self.mouse_used_at = Instant::now();
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        self.pad_pointer.shown = false;
                    }
                    self.mouse_down = state == ElementState::Pressed && !self.pointer_taken(consumed);
                    self.touch();
                }
                if button == MouseButton::Right {
                    let pressed = state == ElementState::Pressed && !self.pointer_taken(consumed);
                    self.tilting_from = pressed.then_some(self.cursor);
                    self.tilt();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_used_at = Instant::now();
                self.cursor = (position.x as f32, position.y as f32);
                self.touch();
                self.tilt();
            }
            WindowEvent::RedrawRequested => self.step(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl App {
    /// the console's settings for a game, from the command line and the
    /// settings.
    fn config(&self) -> Config {
        let interpret = self.options.interpreter || !self.settings.recompiled;
        Config {
            new3ds: self.options.new3ds,
            data_dir: self.data_dir.clone(),
            language: self.settings.language.code(),
            recompiled: self.options.recompiled.clone().filter(|_| !interpret).map(Into::into),
            linked: self.linked.filter(|_| !interpret),
            find_recompiled: !interpret,
            hardware_renderer: self.options.hardware_rasterizer.unwrap_or(self.settings.hardware_rasterizer),
            texture_packs: self.settings.texture_packs,
            original: self.settings.original,
            interpolate: self.settings.frame_interpolation,
            enhancements: self
                .settings
                .enhancements
                .iter()
                .filter_map(|(id, names)| Some((u64::from_str_radix(id, 16).ok()?, names.clone())))
                .chain((!self.options.enhance.is_empty()).then(|| (zakuro_core::ANY_GAME, self.options.enhance.clone())))
                .collect(),
            enhancement_values: self.settings.enhancement_values.iter().filter_map(|(id, values)| Some((u64::from_str_radix(id, 16).ok()?, values.clone()))).collect(),
            resolution: self.settings.resolution,
            device: self.backend.as_ref().and_then(Backend::shared_device),
            username: self.nickname(),
            console_id: self.settings.console_id,
            slider_3d: self.settings.slider_3d(),
            ..Config::default()
        }
    }

    /// starts what the command line asks for, the test pattern or a game.
    fn start(&mut self) {
        if self.options.test_pattern {
            // the test pattern has no CPU program to run, paused keeps just
            // presenting what is there
            let system = paint_test_pattern(self.config());
            self.game = Some(Running::new(system, PathBuf::new(), "test pattern".to_owned()));
            self.paused = true;
        } else if let Some(rom) = self.options.rom.clone() {
            if let Err(error) = self.play(Path::new(&rom)) {
                eprintln!("zakuro: {error}");
                std::process::exit(1);
            }
        }
        if self.options.profile {
            if let Some(game) = &mut self.game {
                game.system.enable_profiler();
            }
        }
    }

    /// starts the game at path, in place of any other.
    fn play(&mut self, path: &Path) -> Result<(), String> {
        // the old game and what it holds on the GPU go first, its sound too,
        // and the overlay's line about it
        self.game = None;
        self.overlay_corner = None;
        if let Some(audio) = &self.audio {
            audio.clear();
        }
        // a Download Play guest goes with the game
        self.dlp_guest = None;
        let mut config = self.config();
        config.wireless_reboot = self.wireless_reboot.take();
        // the update, DLC and Download Play child found with the game in
        // the library
        if let Some(game) = self.library.games.iter().find(|game| game.path == path) {
            config.update = game.update.as_ref().map(|update| update.path.clone());
            config.dlc = game.dlc.iter().map(|dlc| dlc.path.clone()).collect();
            config.dlp_child = game.dlp_child.as_ref().map(|child| child.path.clone());
        } else if let (Some(chosen), true) = (&self.settings.update_file, self.settings.game_file.as_deref() == Some(path)) {
            let (update, dlc) = library::chosen_addon(path, chosen);
            config.update = update.map(|update| update.path);
            config.dlc = dlc.into_iter().map(|dlc| dlc.path).collect();
        }
        // a recording starts the clock at a known time, for playing it back
        let record = std::env::var_os("ZAKURO_RECORD").map(PathBuf::from);
        let clock = zakuro_core::memory::config::host_clock();
        if record.is_some() {
            config.clock = Some(clock);
        }
        // and plays back from the clock it was made at
        let replay = match std::env::var_os("ZAKURO_REPLAY").map(PathBuf::from) {
            Some(file) => {
                let replay = zakuro_core::replay::Replay::open(&file).map_err(|error| format!("could not read {}, {error}", file.display()))?;
                log::info!("playing back the inputs in {}", file.display());
                config.clock = Some(replay.clock());
                Some(replay)
            }
            None => None,
        };
        let system = loader::load(path, config).map_err(|error| format!("could not open {}, {error}", path.display()))?;
        let outdated = system.recompiled.as_ref().is_some_and(|code| code.outdated());
        // the settings ask for the 3D on the GPU, but Vulkan didn't start:
        // only the log said so, while the game ran slowly
        let in_software = system.config.hardware_renderer && !system.gpu.draws_on_gpu();
        let name = self
            .library
            .games
            .iter()
            .find(|game| game.path == path)
            .map(|game| game.name.clone())
            .or_else(|| Some(library::Smdh::parse(system.title.as_ref()?.exefs_file("icon")?)?.name).filter(|name| !name.is_empty()))
            .or_else(|| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
            .unwrap_or_default();
        self.bottom_sounds = system.title.as_ref().map(sounds::bottom_screen).unwrap_or_default();
        let mut game = Running::new(system, path.to_owned(), name);
        game.replay = replay;
        if let Some(record) = record {
            match zakuro_core::replay::Recorder::create(&record, clock) {
                Ok(recorder) => {
                    log::info!("recording the inputs to {}", record.display());
                    game.recorder = Some(recorder);
                }
                Err(error) => log::warn!("could not record to {}, {error}", record.display()),
            }
        }
        if outdated {
            self.menus.message = Some(format!(
                "{} was recompiled by an older version of Zakuro. Recompile it again from the library to get the newest improvements.",
                game.name
            ));
        } else if in_software {
            self.menus.message = Some(GPU_FALLBACK.to_owned());
        }
        self.game = Some(game);
        // the new game's cameras see what the settings say, and it plays
        // in the room the player is in
        self.camera = None;
        self.apply_camera();
        self.link_room();
        self.menus.menu_open = false;
        self.keyboard.release();
        self.paused = false;
        self.next_frame = Instant::now();
        if self.settings.fullscreen && self.window.as_ref().is_some_and(|window| window.fullscreen().is_none()) {
            self.toggle_fullscreen();
        }
        Ok(())
    }

    fn back_to_library(&mut self) {
        self.game = None;
        self.overlay_corner = None;
        if let Some(room) = &self.room {
            room.playing("");
        }
        if let Some(audio) = &self.audio {
            audio.clear();
        }
        self.menus.menu_open = false;
        self.keyboard.release();
    }

    fn key(&mut self, event: KeyEvent, consumed: bool) {
        let pressed = event.state == ElementState::Pressed;
        let PhysicalKey::Code(code) = event.physical_key else { return };
        // Escape leaves a controller binding waiting for a button as it was
        if pressed && code == KeyCode::Escape && self.menus.rebinding_pad.take().is_some() {
            return;
        }
        // a binding waiting for a key takes it, Escape leaves it as it was
        if let (true, Some(index)) = (pressed, self.menus.rebinding) {
            self.menus.rebinding = None;
            if code != KeyCode::Escape {
                if index == menus::BOTTOM_OVER {
                    self.settings.bottom_over_key = code;
                } else if let Some((_, key)) = self.settings.keys.all_mut().into_iter().nth(index) {
                    *key = code;
                }
                self.apply_settings();
            }
            return;
        }
        if code == KeyCode::Tab && !event.repeat && !self.settings.keys.binds(KeyCode::Tab) {
            // fast forward while it is held
            self.fast_forward_key = pressed && self.game.is_some();
        }
        if pressed && !event.repeat {
            self.any_button_pressed = self.game.is_some();
            match code {
                KeyCode::Escape if self.game.is_some() => {
                    self.menus.menu_open = !self.menus.menu_open;
                    self.keyboard.release();
                    return;
                }
                KeyCode::F1 => {
                    self.paused = !self.paused;
                    log::info!("{}", if self.paused { "paused" } else { "resumed" });
                }
                KeyCode::F9 => {
                    self.settings.layout = self.settings.layout.next();
                    self.settings.save();
                    self.apply_layout();
                }
                KeyCode::F10 if self.settings.layout.other_screen() != self.settings.layout => {
                    self.settings.layout = self.settings.layout.other_screen();
                    self.settings.save();
                    self.apply_layout();
                }
                KeyCode::F11 => self.toggle_fullscreen(),
                KeyCode::F5 if self.game.is_some() => self.save_state(),
                KeyCode::F8 if self.game.is_some() => self.load_state(),
                _ if code == self.settings.keys.select && !consumed && !self.gui.as_ref().is_some_and(Gui::wants_keyboard) => {
                    self.map_overview()
                }
                _ if code == self.settings.bottom_over_key && !consumed && !self.gui.as_ref().is_some_and(Gui::wants_keyboard) => {
                    self.toggle_bottom_over()
                }
                _ if code == self.settings.keys.b && self.bottom_over && !consumed && !self.gui.as_ref().is_some_and(Gui::wants_keyboard) => {
                    self.close_bottom_over_with_cancel()
                }
                _ => {}
            }
        }
        let to_game = self.game.is_some()
            && !self.menus.menu_open
            && !self.menus.settings_open
            && !consumed
            && !self.gui.as_ref().is_some_and(Gui::wants_keyboard);
        // letting go always gets through, so that no button stays held
        if to_game || !pressed {
            self.keyboard.key(event.physical_key, pressed);
        }
    }

    /// takes in the controllers' buttons, a binding waiting for one takes
    /// it, and Home opens and closes the menu over the game.
    fn poll_gamepads(&mut self) {
        for button in self.gamepads.poll() {
            self.any_button_pressed = self.game.is_some();
            if let Some(index) = self.menus.rebinding_pad.take() {
                if index == menus::BOTTOM_OVER {
                    self.settings.bottom_over_button = button;
                } else if let Some((_, bound)) = self.settings.pad.all_mut().into_iter().nth(index) {
                    *bound = button;
                }
                self.apply_settings();
            } else if button == gilrs::Button::Mode && self.game.is_some() {
                self.menus.menu_open = !self.menus.menu_open;
                self.keyboard.release();
            } else if button == self.settings.bottom_over_button {
                self.toggle_bottom_over();
                self.pad_pointer.shown = self.bottom_over && !matches!(self.scene, Scene::UnitView | Scene::LevelUp);
            } else if button == self.settings.pad.b && self.bottom_over {
                self.close_bottom_over_with_cancel();
            } else if button == self.settings.pad.select && !self.menus.menu_open {
                self.map_overview();
            }
        }
    }

    /// whether the pointer is busy with something other than the game.
    fn pointer_taken(&self, consumed: bool) -> bool {
        consumed || self.game.is_none() || self.menus.menu_open || self.gui.as_ref().is_some_and(Gui::wants_pointer)
    }

    /// touches the bottom screen while the button is down over it.
    fn touch(&mut self) {
        let Some(window) = &self.window else { return };
        let size = window.inner_size();
        let (top, bottom) = layout(size.width, size.height, self.shown_layout(), self.top_shape.width(), self.settings.integer_scale);
        if let Some(touch) = top.and_then(|top| self.touch_on_pieces(top)) {
            self.keyboard.touch(touch);
            return;
        }
        // with the top screen alone there is nothing to touch
        let Some(bottom) = bottom else {
            self.keyboard.touch(None);
            return;
        };
        let x = self.cursor.0 - bottom.x;
        let y = self.cursor.1 - bottom.y;
        let inside = x >= 0.0 && y >= 0.0 && x < bottom.width && y < bottom.height;
        self.keyboard.touch((self.mouse_down && inside).then(|| {
            ((x / bottom.width * 320.0) as u16, (y / bottom.height * 240.0) as u16)
        }));
    }

    /// Turns a mouse touch on the pieces of the bottom screen into a touch of the game.
    ///
    /// The result is `None` when no pieces are drawn, so the normal handling of touches goes on.
    /// Otherwise it holds the touched point of the bottom screen, or `None` for no touch.
    fn touch_on_pieces(&self, top: zakuro_gpu::Viewport) -> Option<Option<(u16, u16)>> {
        // The buttons of a battle count while they are at least a little visible.
        let buttons = self.buttons.filter(|shown| shown.opacity(Instant::now()) > 0.0).map(|shown| shown.pieces);
        let laid = match self.shown_layout() {
            zakuro_gpu::ScreenLayout::TopWithPieces(pieces) => Some(pieces),
            _ => None,
        };
        if buttons.is_none() && laid.is_none() {
            return None;
        }
        let spans = self.marker_spans();
        let mut places = buttons.into_iter().chain(laid).flat_map(|pieces| zakuro_gpu::piece_places(top, pieces, &spans));
        // Find the first piece under the cursor and map the cursor to a pixel of the bottom screen, which is 320 by 240 pixels.
        let at = places.find_map(|zakuro_gpu::PiecePlace { viewport: place, part: [left, top, width, height], .. }| {
            let (x, y) = ((self.cursor.0 - place.x) / place.width, (self.cursor.1 - place.y) / place.height);
            ((0.0..1.0).contains(&x) && (0.0..1.0).contains(&y)).then(|| (((left + x * width) * 320.0) as u16, ((top + y * height) * 240.0) as u16))
        });
        Some(at.filter(|_| self.mouse_down))
    }

    /// Returns the marker spans that the game reports for the pieces, or none without a game.
    fn marker_spans(&self) -> Vec<(u32, [f32; 4])> {
        self.game.as_ref().map(|game| game.system.marker_spans()).unwrap_or_default()
    }

    /// tilts the console while the right button is held, toward where the
    /// pointer went and by how far, as Citra's motion emulation does,
    /// which titles were tried with. letting go stands it upright again.
    fn tilt(&mut self) {
        // radians a pixel, and the furthest it goes, a quarter turn
        const PER_PIXEL: f32 = 0.01;
        let tilt = match (self.tilting_from, &self.window) {
            (Some((x, y)), Some(window)) => {
                let scale = window.scale_factor() as f32;
                let (dx, dy) = ((self.cursor.0 - x) / scale, (self.cursor.1 - y) / scale);
                let distance = dx.hypot(dy);
                let angle = (distance * PER_PIXEL).min(std::f32::consts::FRAC_PI_2);
                if distance > 0.0 { [dx / distance * angle, dy / distance * angle] } else { [0.0; 2] }
            }
            _ => [0.0; 2],
        };
        self.keyboard.tilt(tilt);
    }

    fn toggle_fullscreen(&mut self) {
        if let Some(window) = &self.window {
            let full = window.fullscreen().is_some();
            if !full {
                self.maximized_before_full_screen = window.is_maximized();
                self.resize_after_full_screen = false;
            }
            window.set_fullscreen((!full).then_some(Fullscreen::Borderless(None)));
            // the layout may have changed meanwhile, which full screen kept
            // the window's size through. a window maximized before comes
            // back maximized, resizing it would take it out of that
            if full && self.resize_after_full_screen && !self.maximized_before_full_screen {
                let _ = window.request_inner_size(self.window_size());
            }
        }
    }

    fn apply_volume(&self) {
        if let Some(audio) = &self.audio {
            audio.set_volume(if self.settings.mute { 0.0 } else { self.settings.volume });
        }
    }

    /// saves the settings and puts to use what can change right away.
    fn apply_settings(&mut self) {
        self.settings.save();
        console::show(self.settings.show_console);
        if let Some(game) = &mut self.game {
            if game.system.config.original != self.settings.original {
                game.system.set_original(self.settings.original);
                log::info!("original mode {}", if self.settings.original { "on" } else { "off" });
            }
            if game.system.gpu.interpolates() != self.settings.frame_interpolation {
                let on = game.system.set_interpolation(self.settings.frame_interpolation);
                log::info!("frame interpolation {}", if on { "on" } else { "off" });
            }
            if game.system.gpu.draws_on_gpu() && game.system.config.resolution != self.settings.resolution {
                let scale = game.system.set_resolution(self.settings.resolution);
                log::info!("drawing at {scale}x");
            }
            // games read the 3D slider every frame
            let slider = self.settings.slider_3d();
            if game.system.config.slider_3d != slider {
                game.system.set_slider_3d(slider);
            }
        }
        self.apply_camera();
        self.keyboard.set_keys(self.settings.keys.clone());
        self.gamepads.set_buttons(self.settings.pad.clone());
        self.apply_volume();
        self.apply_layout();
    }

    /// gives the running game's cameras what the settings say they see,
    /// when it is not what they have.
    fn apply_camera(&mut self) {
        let wanted = (self.settings.camera, self.settings.camera_picture.clone());
        let Some(game) = &mut self.game else { return };
        if self.camera.as_ref() == Some(&wanted) {
            return;
        }
        game.system.camera = match &wanted {
            (CameraSource::Webcam, _) => Some(Box::new(camera::Webcam::default())),
            (CameraSource::Picture, Some(path)) => match camera::picture(path) {
                Ok(picture) => {
                    log::info!("the cameras see {}", path.display());
                    Some(Box::new(picture))
                }
                Err(error) => {
                    self.menus.message = Some(error);
                    None
                }
            },
            _ => None,
        };
        self.camera = Some(wanted);
    }

    /// puts the screens' arrangement and the window scale to use, sizing
    /// the window to them.
    fn apply_layout(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.set_top_width(self.settings.top_shape.width());
            backend.set_scaling(self.settings.filter.screens(), self.settings.integer_scale);
            backend.set_stereo(self.settings.stereo());
        }
        if self.bottom_over {
            let layout = self.shown_layout();
            if let Some(backend) = &mut self.backend {
                backend.set_layout(layout);
            }
        }
        let scale = if self.options.scale.is_none() { self.settings.scale.max(1) } else { self.scale };
        if scale != self.scale || self.settings.layout != self.layout || self.settings.top_shape != self.top_shape {
            self.scale = scale;
            self.layout = self.settings.layout;
            self.top_shape = self.settings.top_shape;
            let layout = self.shown_layout();
            if let Some(backend) = &mut self.backend {
                backend.set_layout(layout);
            }
            // a full screen or maximized window keeps its size, resizing
            // it would take it out of that, full screen fits it on leaving
            match &self.window {
                Some(window) if window.fullscreen().is_some() => self.resize_after_full_screen = true,
                Some(window) if !window.is_maximized() => {
                    let _ = window.request_inner_size(self.window_size());
                }
                _ => {}
            }
        }
    }

    /// Returns where the library finds games: the chosen game file, else the chosen folder, else nowhere.
    fn library_source(&self) -> Option<library::Source> {
        match (&self.settings.game_file, &self.settings.games) {
            (Some(game), _) => Some(library::Source::Files { game: game.clone(), update: self.settings.update_file.clone() }),
            (None, Some(folder)) => Some(library::Source::Folder(folder.clone())),
            (None, None) => None,
        }
    }

    /// Saves the settings and scans the library again.
    fn save_and_rescan(&mut self) {
        self.settings.save();
        if let Some(source) = self.library_source() {
            self.library.scan(source);
        }
    }

    /// Returns the layout of the screens that is actually drawn.
    ///
    /// It follows the chosen layout, except that the top screen alone can gain pieces of the bottom screen, or the bottom screen laid over it, depending on the scene.
    /// Original mode always keeps the chosen layout.
    fn shown_layout(&self) -> zakuro_gpu::ScreenLayout {
        use zakuro_gpu::ScreenLayout;
        match self.layout.screens() {
            ScreenLayout::TopOnly if self.settings.original => ScreenLayout::TopOnly,
            ScreenLayout::TopOnly => match self.scene {
                Scene::Pieces(pieces) => ScreenLayout::TopWithPieces(pieces),
                Scene::Battle(pieces) => match self.settings.battle_ui {
                    settings::BattleUi::Panels => ScreenLayout::TopWithPieces(pieces),
                    settings::BattleUi::Banners | settings::BattleUi::Off => ScreenLayout::TopOnly,
                },
                _ if self.bottom_over => self.bottom_over_layout(),
                Scene::WorldMap => match self.game.as_ref().and_then(|game| game.system.world_pieces()) {
                    Some(pieces) => ScreenLayout::TopWithPieces(pieces),
                    None => ScreenLayout::TopOnly,
                },
                _ => ScreenLayout::TopOnly,
            },
            screens => screens,
        }
    }

    /// Returns the layout that lays the bottom screen over the top screen.
    fn bottom_over_layout(&self) -> zakuro_gpu::ScreenLayout {
        // Leave room for the soft edge, then take a fixed share of what remains.
        let room = 1.0 - BOTTOM_OVER_EDGE / self.top_shape.width();
        let width = BOTTOM_OVER_WIDTH * room;
        // The layout stores the width as a whole number of ten thousandths of the width of the top screen.
        let ten_thousandths = (width * 10_000.0).round().clamp(0.0, 10_000.0) as u16;
        zakuro_gpu::ScreenLayout::BottomOverTop { width: ten_thousandths }
    }

    /// Returns the scene that the game shows, as far as the state of the game tells.
    ///
    /// It is `Plain` unless the top screen is shown alone.
    fn current_scene(&self) -> Scene {
        let Some(game) = self.game.as_ref().filter(|_| self.layout == Screens::TopOnly) else {
            return Scene::Plain;
        };
        match game.system.battle_pieces().map(Scene::Battle).or_else(|| game.system.title_pieces().map(Scene::Pieces)) {
            Some(scene) => scene,
            None if game.system.world_map() => Scene::WorldMap,
            None if game.system.unit_view() => Scene::UnitView,
            None if game.system.forecast() || game.system.brief_forecast() => Scene::Forecast,
            None => Scene::Plain,
        }
    }

    /// Returns whether the bottom screen shows the level up screen right now.
    fn showing_level_up(&mut self) -> bool {
        self.game.as_mut().and_then(|game| game.system.bottom_picture()).is_some_and(|picture| overlays::level_up_screen(&picture))
    }

    /// Returns which screen of the world map the bottom screen shows, if any.
    fn bottom_map_screen(&mut self) -> Option<overlays::MapScreen> {
        self.game.as_mut().and_then(|game| game.system.bottom_picture()).as_deref().and_then(overlays::map_screen)
    }

    /// Decides, once per presented frame, whether the bottom screen is laid over the top screen, and applies the result.
    ///
    /// It works out the scene, plays the open or close sound, runs the scripted taps that restore the screens of the map, and hands a changed layout to the backend.
    fn place_bottom_over(&mut self) {
        // After the map button was tapped, wait until the overview shows, or give up after a number of frames.
        if let Some(waited) = self.pending_overview {
            if waited >= overlays::OVERVIEW_WAIT || self.bottom_map_screen() == Some(overlays::MapScreen::Overview) {
                self.pending_overview = None;
                self.bottom_over_key = true;
            } else {
                self.pending_overview = Some(waited + 1);
            }
        }
        let mut scene = self.current_scene();
        let mut talking = self.game.as_ref().is_some_and(|game| game.system.talking());
        // After a tap on the portrait, the scene counts as the view of the unit for a number of frames from the tap and for a number of frames after the game last talked.
        if let Some((from, talked)) = self.unit_view_from {
            let now = self.game.as_ref().map_or(from, |game| game.system.frames);
            let talked = if talking { now } else { talked };
            let entering = scene == Scene::Plain && (now - from < overlays::UNIT_VIEW_START || now - talked < overlays::UNIT_VIEW_GAP);
            self.unit_view_from = entering.then_some((from, talked));
            if entering {
                scene = Scene::UnitView;
                talking = false;
            }
        }
        if scene == Scene::Plain && self.bottom_over_key && self.showing_level_up() {
            scene = Scene::LevelUp;
        }
        // Only these scenes can show the bottom screen over the top screen.
        let opens = matches!(scene, Scene::Plain | Scene::WorldMap | Scene::UnitView | Scene::LevelUp);
        let into_level_up = scene == Scene::LevelUp && self.scene == Scene::Plain;
        let moved_on = (scene != self.scene && !into_level_up) || (opens && talking && scene != Scene::LevelUp);
        let touched_lately = self.bottom_over && self.touched_scene == Scene::WorldMap && self.over_touched_at.is_some_and(|at| at.elapsed() < overlays::TOUCH_KEEPS_OVER);
        let talk_ended = self.kept_scene.is_some() && !talking;
        if !overlays::keeps_bottom_over(moved_on, opens, talking, touched_lately, scene, &mut self.kept_scene) {
            self.bottom_over_key = false;
        }
        self.scene = scene;
        let shown = self.game.is_some() && self.layout == Screens::TopOnly && !self.settings.original && opens && self.bottom_over_key;
        // Play the open sound when the bottom screen appears. Play the close sound when it goes away by the choice of the user or because the talk ended, not when the scene simply changed.
        let sound = match (self.bottom_over, shown) {
            (false, true) if !self.quiet_over => self.bottom_sounds.open.as_ref(),
            (true, false) if !self.quiet_over && (!moved_on || talk_ended || self.closing_by_user) && self.game.is_some() => self.bottom_sounds.close.as_ref(),
            _ => None,
        };
        if let (Some(audio), Some(sound)) = (&self.audio, sound) {
            audio.play(sound);
        }
        self.closing_by_user = false;
        if !shown && self.pending_overview.is_none() {
            self.quiet_over = false;
        }
        // When the bottom screen closes, tap the game back to the screen that it showed before.
        if self.bottom_over && !shown && std::mem::take(&mut self.restore_inventory) && self.bottom_map_screen() == Some(overlays::MapScreen::Overview) {
            self.scripted_tap = Some((overlays::INVENTORY_TAB, overlays::TAP_FRAMES));
        }
        if self.bottom_over && !shown && self.bottom_map_screen() == Some(overlays::MapScreen::Story) {
            self.scripted_tap = Some((overlays::STORY_CLOSE, overlays::TAP_FRAMES));
        }
        self.bottom_over = shown;
        if !shown {
            self.kept_scene = None;
        }
        let layout = self.shown_layout();
        if self.placed != Some(layout) {
            self.placed = Some(layout);
            if let Some(backend) = &mut self.backend {
                backend.set_layout(layout);
            }
        }
    }

    /// Toggles the bottom screen over the top screen and holds back the B button from the game until it is released.
    fn close_bottom_over_with_cancel(&mut self) {
        self.toggle_bottom_over();
        self.holding_back_cancel = true;
    }

    /// Opens or closes the overview of the world map over the top screen.
    ///
    /// When the bottom screen shows the inventory, the map button is tapped first, and the overview opens once it shows.
    fn map_overview(&mut self) {
        if self.pending_overview.is_some() || self.layout != Screens::TopOnly || self.settings.original {
            return;
        }
        if self.bottom_over && self.quiet_over {
            self.bottom_over_key = false;
            self.closing_by_user = true;
            self.place_bottom_over();
            return;
        }
        if self.scene != Scene::Plain {
            return;
        }
        if self.game.as_ref().is_none_or(|game| game.system.talking()) {
            return;
        }
        match (self.bottom_map_screen(), self.bottom_over) {
            (Some(overlays::MapScreen::Overview), false) => {
                self.quiet_over = true;
                self.bottom_over_key = true;
                self.place_bottom_over();
            }
            (Some(overlays::MapScreen::Overview), true) => {
                self.bottom_over_key = false;
                self.closing_by_user = true;
                self.place_bottom_over();
            }
            (Some(overlays::MapScreen::Inventory), open) => {
                self.scripted_tap = Some((overlays::MAP_BUTTON, overlays::TAP_FRAMES));
                self.restore_inventory = true;
                if !open {
                    self.quiet_over = true;
                    self.pending_overview = Some(0);
                }
            }
            _ => {}
        }
    }

    /// Turns the bottom screen over the top screen on or off.
    ///
    /// It does nothing in original mode or with another layout than the top screen alone.
    fn toggle_bottom_over(&mut self) {
        let Some(game) = &self.game else { return };
        if self.layout != Screens::TopOnly || self.settings.original {
            return;
        }
        // Without a unit under the cursor or selected there is nothing to show, unless the level up screen is up.
        let on_unit = !game.system.follows_cursor() || game.system.cursor_on_unit() || game.system.unit_selected();
        if !self.bottom_over_key && self.scene == Scene::Plain && !on_unit && !self.showing_level_up() {
            return;
        }
        self.closing_by_user = self.bottom_over_key;
        self.bottom_over_key = !self.bottom_over_key;
        self.place_bottom_over();
    }

    /// the window size that fits the screens at the chosen scale.
    fn window_size(&self) -> winit::dpi::Size {
        let (width, height) = self.layout.screens().size(self.top_shape.width());
        // scaled by whole numbers, a desktop scaled by 125% say would leave a
        // border, so the window gets whole multiples of the screens' pixels
        if let (true, Some(window)) = (self.settings.integer_scale, &self.window) {
            let times = (self.scale as f64 * window.scale_factor()).round().max(1.0) as u32;
            return winit::dpi::PhysicalSize::new(width * times, height * times).into();
        }
        winit::dpi::LogicalSize::new(width * self.scale, height * self.scale).into()
    }

    /// recompiles the library's game at index, with the computer's C
    /// compiler, or the Zig Zakuro downloaded, or else, when download is
    /// set, with Zig downloaded first. with none of those it asks whether
    /// to download it.
    fn recompile(&mut self, index: usize, download: bool) {
        let Some(program_id) = self.library.games.get(index).map(|game| game.program_id) else { return };
        self.jobs.retain(|job| job.program_id != program_id || !job.stage().finished());
        if self.jobs.iter().any(|job| job.program_id == program_id) {
            return;
        }
        let tools = zig::tools_dir(self.data_dir.as_deref());
        let toolchain = match recomp3ds::compile::check(None) {
            Ok(compiler) => Toolchain::Ready(compiler),
            Err(error) => match tools.as_deref().and_then(zig::installed) {
                Some(compiler) => Toolchain::Ready(compiler),
                None => match tools {
                    Some(tools) if download && zig::download_size().is_some() && zig::can_download() => Toolchain::Download(tools),
                    Some(_) if zig::download_size().is_some() && zig::can_download() => {
                        self.menus.compiler_offer = Some(index);
                        return;
                    }
                    Some(_) if zig::download_size().is_some() => {
                        self.menus.message =
                            Some(format!("Recompiling needs a C compiler, gcc or clang, or curl or wget for Zakuro to download one: {error}."));
                        return;
                    }
                    _ => {
                        self.menus.message = Some(format!("Recompiling needs a C compiler: {error}."));
                        return;
                    }
                },
            },
        };
        let Some(game) = self.library.games.get(index) else { return };
        let update = game.update.as_ref().map(|update| update.path.as_path());
        self.jobs.push(Job::start(&game.path, update, game.program_id, &game.name, self.data_dir.as_deref(), toolchain));
    }

    /// Uninstalls the mod at the index and tells the result in a message.
    ///
    /// It does nothing while the mod is being installed.
    fn uninstall_mod(&mut self, index: usize) {
        let (Some(step), Some(data_dir)) = (recompile::MOD_STEPS.get(index), self.data_dir.as_deref()) else { return };
        if self.jobs.iter().any(|job| job.installing == Some(step.label) && !job.stage().finished()) {
            return;
        }
        self.menus.message = Some(match step.uninstall(data_dir) {
            Ok(()) => format!("The {} mod is uninstalled.", step.label),
            Err(error) => format!("The {} mod could not be uninstalled, {error}", step.label),
        });
    }

    /// Starts the installation of the mod at the index as a job.
    ///
    /// The mod is built from the game that it belongs to.
    /// When downloads are needed and the user has not agreed yet, an offer is shown instead.
    fn install_mod(&mut self, index: usize, agreed: bool) {
        let Some(step) = recompile::MOD_STEPS.get(index) else { return };
        let Some(data_dir) = self.data_dir.clone() else {
            self.menus.message = Some("The mods need Zakuro's data folder.".to_owned());
            return;
        };
        let Some((path, name)) = self.library.games.iter().find(|game| game.program_id == step.program_id).map(|game| (game.path.clone(), game.name.clone())) else {
            self.menus.message = Some(format!("The {} mod is built from your copy of the game. Choose it first, in Settings, System.", step.label));
            return;
        };
        self.jobs.retain(|job| job.program_id != step.program_id || !job.stage().finished());
        if self.jobs.iter().any(|job| job.program_id == step.program_id) {
            return;
        }
        if !agreed && step.lacks_downloads(&data_dir) {
            self.menus.download_offer = Some(menus::DownloadOffer { step: index });
            return;
        }
        self.jobs.push(Job::install(&path, &name, &data_dir, step));
    }

    /// Collects the results of the check for an update and of the installation, and sets the state of the update button.
    ///
    /// A finished installation restarts the launcher.
    fn poll_update(&mut self) {
        if let Some(found) = self.update_check.as_ref().and_then(|check| check.try_recv().ok()) {
            self.update_check = None;
            self.update_offer = found;
        }
        if let Some(result) = self.update_install.as_ref().and_then(|installing| installing.done.try_recv().ok()) {
            self.update_install = None;
            if let Err(error) = result.and_then(|exe| update::restart(&exe).map_err(|error| error.to_string())) {
                self.menus.message = Some(format!("The update could not be installed, {error}"));
            }
        }
        self.menus.update = match (&self.update_install, &self.update_offer) {
            (Some(installing), _) => Some(menus::UpdateButton::Installing(installing.progress.lock().map_or(0.0, |progress| *progress))),
            (None, Some(_)) => Some(menus::UpdateButton::Available),
            (None, None) if self.updatable => Some(menus::UpdateButton::Idle),
            (None, None) => None,
        };
    }

    /// tells about recompiles as they finish.
    fn poll_jobs(&mut self) {
        let mut finished = Vec::new();
        for job in &mut self.jobs {
            if !job.announced && job.stage().finished() {
                job.announced = true;
                finished.push((job.program_id, job.name.clone(), job.stage(), job.installing));
            }
        }
        for (program_id, name, stage, installing) in finished {
            match (stage, installing) {
                (Stage::Done, Some(label)) => self.menus.message = Some(format!("The {label} mod is installed.")),
                (Stage::Failed(error), Some(label)) if error != "cancelled" => self.menus.message = Some(format!("Installing the {label} mod failed, {error}")),
                (_, Some(_)) => {}
                (Stage::Done, None) => {
                    self.library.refresh_recompiled();
                    let playing = self.game.as_ref().is_some_and(|game| game.system.title.as_ref().is_some_and(|title| title.program_id() == program_id));
                    if !playing {
                        self.menus.message = Some(format!("{name} is recompiled and ready to play."));
                    }
                    let installed = zakuro_core::recompiled::installed(program_id);
                    if let (true, Some(game), Some(path)) = (playing, self.game.as_mut(), installed) {
                        // from here on the game runs on the new code, no reset needed
                        match zakuro_core::loader::swap_recompiled(&mut game.system, &path) {
                            Ok(()) => self.menus.message = Some(format!("{name} is recompiled, and it runs on the new code now.")),
                            Err(error) => {
                                log::warn!("could not switch {name} to its new code, {error}");
                                self.menus.message = Some(format!("{name} is recompiled. Reset it from the menu, Esc, to run it on the new code."));
                            }
                        }
                    }
                }
                (Stage::Failed(error), None) if error != "cancelled" => self.menus.message = Some(format!("Recompiling {name} failed, {error}")),
                _ => {}
            }
        }
    }

    fn act(&mut self, action: Action, event_loop: &ActiveEventLoop) {
        match action {
            Action::Play(path) => {
                if let Err(error) = self.play(&path) {
                    self.menus.message = Some(error);
                }
            }
            Action::Recompile(index) => self.recompile(index, false),
            Action::InstallMod(index, agreed) => self.install_mod(index, agreed),
            Action::UninstallMod(index) => self.uninstall_mod(index),
            Action::ModEnabled(index, on) => {
                if let (Some(data_dir), Some(step)) = (&self.data_dir, recompile::MOD_STEPS.get(index)) {
                    if let Err(error) = zakuro_core::mods::set_enabled(&step.folder_in(data_dir), on) {
                        self.menus.message = Some(format!("{} can't be turned {}, {error}", step.label, if on { "on" } else { "off" }));
                    }
                }
            }
            Action::Update => match (self.update_offer.clone(), &self.update_install) {
                (_, Some(_)) => {}
                (Some(release), None) => self.update_install = Some(update::install_in_background(release)),
                (None, None) if self.update_check.is_some() => self.menus.message = Some("Looking for an update, try again in a moment.".to_owned()),
                (None, None) => self.menus.message = Some("No update available.".to_owned()),
            },
            Action::DownloadCompiler(index) => self.recompile(index, true),
            Action::Mods(program_id) => {
                if let Some(data_dir) = &self.data_dir {
                    let folder = zakuro_core::mods::dir(data_dir, program_id);
                    let made = std::fs::create_dir_all(folder.join("romfs")).and_then(|()| std::fs::create_dir_all(folder.join("textures")));
                    match made {
                        Ok(()) => show_folder(&folder),
                        Err(error) => self.menus.message = Some(format!("The mods folder can't be made: {error}")),
                    }
                }
            }
            Action::CancelRecompile(program_id) => {
                for job in self.jobs.iter().filter(|job| job.program_id == program_id) {
                    job.cancel();
                }
            }
            Action::ChooseGame => {
                if let Some(file) = rfd::FileDialog::new().set_title("The game's file").add_filter("3DS games", &["3ds", "cci", "cxi", "cia"]).pick_file() {
                    self.settings.game_file = Some(file);
                    self.save_and_rescan();
                }
            }
            Action::ChooseUpdate => {
                if let Some(file) = rfd::FileDialog::new().set_title("The game's DLC").add_filter("DLC", &["cia"]).pick_file() {
                    self.settings.update_file = Some(file);
                    self.save_and_rescan();
                }
            }
            Action::ForgetUpdate => {
                self.settings.update_file = None;
                self.save_and_rescan();
            }
            Action::SaveState => self.save_state(),
            Action::LoadState => self.load_state(),
            Action::HostRoom => self.host_room(),
            Action::JoinRoom => {
                let address = self.settings.room_address.clone();
                self.join_room(&address);
            }
            Action::LeaveRoom => self.leave_room(),
            Action::DlpJoin(index) => self.join_download_play(index),
            Action::DlpLeave => {
                if let Some(guest) = &mut self.dlp_guest {
                    guest.leave();
                }
                self.dlp_own = None;
            }
            Action::Resume => {
                self.menus.menu_open = false;
                self.keyboard.release();
            }
            Action::Reset => {
                if let Some(path) = self.game.as_ref().map(|game| game.path.clone()) {
                    if let Err(error) = self.play(&path) {
                        self.menus.message = Some(error);
                    }
                }
            }
            Action::Library => self.back_to_library(),
            Action::Fullscreen => self.toggle_fullscreen(),
            Action::CopyReport => {
                if let (Some(game), Some(gui)) = (&self.game, &self.gui) {
                    gui.ctx.copy_text(report::text(&game.name, &game.system));
                    self.menus.message = Some("The game's details and the end of the log are copied. Paste them in your report and fill in what happens.".into());
                }
            }
            Action::Cheat(index, on) => {
                let Some(game) = &mut self.game else { return };
                let program_id = game.system.title.as_ref().map_or(0, |title| title.program_id());
                let Some(cheat) = game.system.cheats.get_mut(index) else { return };
                cheat.enabled = on;
                log::info!("{} {} {}", if cheat.builtin { "enhancement" } else { "cheat" }, cheat.name, if on { "on" } else { "off" });
                if cheat.builtin {
                    // an enhancement stays on for the game in the settings
                    let names = self.settings.enhancements.entry(format!("{program_id:016X}")).or_default();
                    names.retain(|name| *name != cheat.name);
                    if on {
                        names.push(cheat.name.clone());
                    }
                    self.settings.save();
                } else {
                    self.save_cheats();
                }
            }
            Action::CheatValue(index, value) => {
                let Some(game) = &mut self.game else { return };
                let Some(title) = &game.system.title else { return };
                let program_id = title.program_id();
                let Some(cheat) = game.system.cheats.get_mut(index) else { return };
                let Some(value) = cheat.set_value(value) else { return };
                self.settings.enhancement_values.entry(format!("{program_id:016X}")).or_default().insert(cheat.name.clone(), value);
                self.settings.save();
            }
            Action::AddCheat(name, code) => {
                let lines: Vec<String> = code.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_owned).collect();
                if lines.iter().any(|line| !zakuro_core::cheats::is_code(line)) {
                    self.menus.message = Some("Some of those lines aren't codes like 00000000 00000000, the cheat skips them.".into());
                }
                if let Some(game) = &mut self.game {
                    let name = match name.trim() {
                        "" => format!("Cheat {}", game.system.cheats.len() + 1),
                        name => name.to_owned(),
                    };
                    let mut cheat = zakuro_core::cheats::Cheat::new(&name, lines, Vec::new());
                    cheat.enabled = true;
                    game.system.cheats.push(cheat);
                }
                self.save_cheats();
            }
            Action::ReloadCheats => {
                if let (Some(path), Some(game)) = (self.cheats_path(), &mut self.game) {
                    let enhancements: Vec<_> = game.system.cheats.drain(..).filter(|cheat| cheat.builtin).collect();
                    game.system.cheats = zakuro_core::cheats::load(&path);
                    game.system.cheats.extend(enhancements);
                }
            }
            Action::CheatsFolder => {
                if let Some(folder) = self.cheats_path().and_then(|path| path.parent().map(Path::to_owned)) {
                    match std::fs::create_dir_all(&folder) {
                        Ok(()) => show_folder(&folder),
                        Err(error) => self.menus.message = Some(format!("The cheats folder can't be made: {error}")),
                    }
                }
            }
            Action::Quit => event_loop.exit(),
            Action::Settings => self.apply_settings(),
            Action::Keyboard(text, button) => {
                if let Some(game) = &mut self.game {
                    game.system.answer_keyboard(&text, button);
                }
            }
        }
    }

    fn step(&mut self, event_loop: &ActiveEventLoop) {
        if self.stop {
            event_loop.exit();
            return;
        }
        let now = Instant::now();
        if now > self.next_frame + CATCH_UP_LIMIT {
            self.next_frame = now;
        }
        self.library.poll();
        self.poll_jobs();
        self.poll_update();
        if self.menus.settings_open {
            self.menus.mods = self
                .data_dir
                .as_deref()
                .map(|data_dir| {
                    recompile::MOD_STEPS
                        .iter()
                        .map(|step| {
                            let installing = self.jobs.iter().find(|job| job.installing == Some(step.label) && !job.stage().finished());
                            menus::ModSwitch {
                                label: step.label,
                                program_id: step.program_id,
                                about: step.about,
                                on: zakuro_core::mods::enabled(&step.folder_in(data_dir)),
                                installed: step.installed(data_dir),
                                progress: installing.map(|job| job.stage()).map(|stage| (stage.fraction(), stage.doing())),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
        }
        self.poll_gamepads();
        if self.bottom_over && !matches!(self.scene, Scene::UnitView | Scene::LevelUp) {
            self.pad_pointer.steer(self.gamepads.lean(), now);
        } else {
            self.pad_pointer.rest();
        }
        // a room that went away takes the game's local wireless with it
        if self.room.as_ref().is_some_and(|room| !room.open()) {
            self.leave_room();
            self.menus.message = Some("The room closed, or the connection to it was lost.".into());
        }
        self.download_play();

        // a game waiting on its keyboard waits for the user, not running
        let typing = self.game.as_ref().is_some_and(|game| game.system.keyboard_request().is_some());
        let playing =
            self.game.is_some() && !self.paused && !self.menus.menu_open && !self.menus.settings_open && !typing;
        // Tab or a controller's right trigger, or the right stick's click
        // while ZR has the trigger, whichever no button of the console's is on
        let free = |button: gilrs::Button| self.settings.pad.map().iter().all(|&(_, bound)| bound != button);
        let trigger = [gilrs::Button::RightTrigger2, gilrs::Button::RightThumb].into_iter().find(|&button| free(button));
        let fast = playing && (self.fast_forward_key || trigger.is_some_and(|trigger| self.gamepads.holding(trigger)));
        if fast {
            // the output waits out fast forward rather than run dry
            if let Some(audio) = &self.audio {
                audio.hold();
            }
        }
        if fast != self.fast_forwarding {
            self.fast_forwarding = fast;
            if !fast {
                // back at the console's pace, with no sound piled up
                if let Some(audio) = &self.audio {
                    audio.clear();
                }
                self.next_frame = Instant::now();
            }
        }
        if playing {
            let start = Instant::now();
            self.emulate();
            self.spent.emulating.add(start.elapsed());
        } else if let Some(audio) = &self.audio {
            // the sound stops with the game, which is not falling behind
            audio.hold();
        }

        self.next_frame += FRAME_TIME;
        if fast {
            // nothing to wait for
            self.next_frame = Instant::now();
        }
        // behind the schedule, showing the frame would wait on the display,
        // so it goes unshown, a few at most. a presenter that does not wait,
        // in mailbox mode, shows them all, a game that could not keep up
        // showed a fifth of its frames. fast forward shows them at the
        // console's frame rate, waiting on the display for no more
        let behind = Instant::now() > self.next_frame;
        let waits = self.backend.as_ref().is_some_and(Backend::waits_for_display);
        let unshown = if fast { Instant::now() < self.next_shown } else { skips_showing(playing, behind, waits, self.skipped) };
        if unshown {
            self.skipped += 1;
        } else {
            self.skipped = 0;
            let start = Instant::now();
            self.present(event_loop);
            // the next one a frame later, or right away after falling
            // behind, at the console's rate on average
            self.next_shown = (self.next_shown + FRAME_TIME).max(Instant::now() - FRAME_TIME);
            self.spent.showing.add(start.elapsed());
            self.shown += 1;
        }

        if self.last_title_update.elapsed() >= Duration::from_millis(500) {
            let shown = self.shown as f32 / self.last_title_update.elapsed().as_secs_f32();
            self.shown = 0;
            self.last_title_update = Instant::now();
            if let Some(game) = &self.game {
                let Spent { emulating, showing } = std::mem::take(&mut self.spent);
                log::debug!(
                    target: "zakuro::fps",
                    "{:.1} frames emulated and {shown:.1} shown a second, emulating took {emulating}, showing {showing}",
                    game.fps
                );
            }
            if let Some(window) = &self.window {
                let title = match &self.game {
                    Some(game) => format!("Zakuro - {} - {}", game.name, game.system.status_line()),
                    None => "Zakuro".to_owned(),
                };
                window.set_title(&title);
            }
            let underruns = self.audio.as_ref().map_or(0, |audio| audio.take_underruns());
            if underruns > 0 && playing {
                log::warn!("the sound ran dry {underruns} times, the emulation is falling behind");
            }
        }

        let now = Instant::now();
        if now < self.next_frame {
            std::thread::sleep(self.next_frame - now);
        }
    }

    /// runs one frame of the game.
    fn emulate(&mut self) {
        let Some(game) = &mut self.game else { return };
        let mut input = match &mut game.replay {
            Some(replay) => replay.input(game.frame),
            None if self.bottom_over => {
                let mut input = overlays::bottom_over_input(self.gamepads.apply(self.keyboard.state()), self.scene);
                if self.pad_pointer.shown && self.gamepads.holding(self.settings.pad.a) {
                    input.touch = Some(self.pad_pointer.spot());
                }
                input
            }
            None => self.gamepads.apply(self.keyboard.state()),
        };
        // A scripted tap presses its point for a number of frames, one frame at a time.
        if let (None, Some((spot, frames))) = (&game.replay, self.scripted_tap) {
            input.touch = Some(spot);
            self.scripted_tap = frames.checked_sub(1).filter(|&left| left > 0).map(|left| (spot, left));
        }
        // Keep the B button from the game while it is still held, so that the press that closed the bottom screen does not also cancel in the game.
        if self.holding_back_cancel {
            if input.buttons.contains(zakuro_core::services::hid::PadState::B) {
                input.buttons.remove(zakuro_core::services::hid::PadState::B);
            } else {
                self.holding_back_cancel = false;
            }
        }
        if self.bottom_over && game.replay.is_none() {
            if let Some(spot) = input.touch {
                if self.over_touch_start.is_none() {
                    let inventory = game.system.bottom_picture().as_deref().and_then(overlays::map_screen) == Some(overlays::MapScreen::Inventory);
                    self.over_touch_start = Some((spot, inventory));
                }
            } else if let Some((spot, true)) = self.over_touch_start.take() {
                if overlays::on_portrait(spot) {
                    self.bottom_over_key = false;
                    self.closing_by_user = true;
                    self.unit_view_from = Some((game.system.frames, game.system.frames));
                }
            }
        } else {
            self.over_touch_start = None;
        }
        if self.bottom_over && input.touch.is_some() {
            self.over_touched_at = Some(Instant::now());
            self.touched_scene = self.scene;
        }
        // Only the buttons that went down in this frame, and a touch that began in this frame, can bring the buttons of a battle.
        let (held, touched) = std::mem::replace(&mut self.last_input, (input.buttons, input.touch.is_some()));
        let pieces = match self.scene {
            Scene::Battle(_) if !self.settings.original => game.system.battle_buttons(self.settings.battle_ui == settings::BattleUi::Panels),
            _ => None,
        };
        self.buttons = ShownButtons::next(self.buttons, pieces, input.buttons.difference(held), input.touch.is_some() && !touched, std::mem::take(&mut self.any_button_pressed), Instant::now());
        game.frame += 1;
        if let Some(recorder) = &mut game.recorder {
            recorder.record(input);
        }
        game.system.set_input(input);
        let outcome = game.system.run_frame();
        // the overlay reads the game the program was built on, not another
        // one its library opened
        let ours = self.linked.as_ref().is_none_or(|linked| game.system.title.as_ref().is_some_and(|title| title.program_id() == linked.program_id()));
        if let Some(overlay) = self.overlay.as_mut().filter(|_| ours) {
            let text = overlay.frame(&mut game.system.memory);
            self.overlay_corner = text.corner;
            if let Some(notice) = text.notice {
                self.menus.toast = Some((notice, Instant::now()));
            }
        }
        game.count_frame();
        let sound = game.system.take_audio();
        // fast forward plays no sound, there is too much of it too soon
        if let (Some(audio), false) = (&self.audio, self.fast_forwarding) {
            audio.push(&sound);
        }
        match outcome {
            FrameOutcome::Completed => {}
            FrameOutcome::Exited => {
                log::info!("the title exited");
                self.back_to_library();
            }
            FrameOutcome::Faulted => {
                log::error!("the title stopped on a fault");
                let errors = game.system.fatal_errors.join("; ");
                if !errors.is_empty() {
                    log::error!("{errors}");
                }
                let message = format!("The game stopped on a fault. {errors}");
                self.menus.report = Some((message.clone(), report::text(&game.name, &game.system)));
                self.menus.message = Some(message);
                self.back_to_library();
            }
        }
    }

    /// the name the player goes by, the console's too.
    fn nickname(&self) -> String {
        match self.settings.nickname.trim() {
            "" => "Player".into(),
            name => name.chars().take(10).collect(),
        }
    }

    /// gives the running game the room the player is in, and tells the room
    /// the game.
    fn link_room(&mut self) {
        let Some(game) = &mut self.game else { return };
        game.system.room = self.room.as_ref().map(|room| room.link());
        if let Some(room) = &self.room {
            room.playing(&game.name);
        }
    }

    /// opens a room on the port chosen, and joins it.
    fn host_room(&mut self) {
        let name = match self.settings.room_name.trim() {
            "" => format!("{}'s room", self.nickname()),
            name => name.to_owned(),
        };
        let password = self.menus.room_password.clone();
        match zakuro_core::room::Server::start(self.settings.room_port, &name, &password, 16) {
            Ok(server) => {
                let address = format!("127.0.0.1:{}", server.port());
                self.room_server = Some(server);
                self.join_room(&address);
                if self.room.is_none() {
                    self.room_server = None;
                }
            }
            Err(error) => self.menus.message = Some(format!("The room could not be opened on port {}, {error}", self.settings.room_port)),
        }
    }

    /// joins the room at address.
    fn join_room(&mut self, address: &str) {
        let password = self.menus.room_password.clone();
        match zakuro_core::room::Connection::join(address.trim(), &self.nickname(), &password) {
            Ok(room) => {
                self.room = Some(room);
                self.link_room();
            }
            Err(error) => self.menus.message = Some(format!("Could not join the room, {error}.")),
        }
    }

    /// leaves the room, and closes it when it is this program's.
    fn leave_room(&mut self) {
        self.dlp_guest = None;
        self.room = None;
        self.room_server = None;
        if let Some(game) = &mut self.game {
            game.system.room = None;
        }
    }

    /// Download Play as a guest: hears the offers in the room while no game
    /// runs, and starts the child once the host's game begins.
    fn download_play(&mut self) {
        let Some(room) = self.room.as_ref().filter(|_| self.game.is_none()) else {
            self.dlp_guest = None;
            return;
        };
        let guest = self.dlp_guest.get_or_insert_with(|| zakuro_core::download_play::Guest::new(room.link()));
        guest.poll();
        match guest.end() {
            Some(zakuro_core::download_play::Phase::Begun { child, reboot }) => {
                let own = self.dlp_own.take();
                let path = match child {
                    Some(child) => self.keep_child(&child),
                    None => own.ok_or_else(|| "the game to download went missing".to_owned()),
                };
                let started = path.and_then(|path| {
                    self.wireless_reboot = Some(reboot);
                    self.play(&path)
                });
                if let Err(error) = started {
                    self.wireless_reboot = None;
                    self.menus.message = Some(format!("Download Play could not start the game, {error}."));
                }
            }
            Some(zakuro_core::download_play::Phase::Failed(why)) => {
                self.dlp_own = None;
                self.menus.message = Some(format!("Download Play stopped, {why}."));
            }
            _ => {}
        }
    }

    /// takes part in the Download Play offer at index, with the player's own
    /// copy of the child when the folder has one.
    fn join_download_play(&mut self, index: usize) {
        let node = zakuro_core::download_play::node(self.settings.console_id, &self.nickname());
        let Some(guest) = &mut self.dlp_guest else { return };
        let Some(heard) = guest.offers().get(index) else { return };
        let (host, child) = (heard.host, heard.offer.child);
        self.dlp_own = self.library.children.iter().find(|own| own.title_id == child).map(|own| own.path.clone());
        guest.join(host, node, self.dlp_own.is_some());
    }

    /// keeps a child the host sent, for it to run from.
    fn keep_child(&self, child: &[u8]) -> Result<PathBuf, String> {
        let cia = zakuro_fs::Cia::read(child).map_err(|_| "the host sent something that isn't a game".to_owned())?;
        let folder = self.data_dir.clone().unwrap_or_else(std::env::temp_dir).join("dlp");
        let path = folder.join(format!("{:016X}.cia", cia.title_id));
        std::fs::create_dir_all(&folder).and_then(|()| std::fs::write(&path, child)).map_err(|error| error.to_string())?;
        Ok(path)
    }

    /// the file of a slot of the running game's saved states.
    fn state_path(&self, slot: u8) -> Option<PathBuf> {
        let program_id = self.game.as_ref()?.system.title.as_ref()?.program_id();
        Some(self.data_dir.as_ref()?.join("states").join(format!("{program_id:016X}")).join(format!("{slot}.state")))
    }

    /// saves the running game's state to the slot chosen.
    fn save_state(&mut self) {
        let slot = self.menus.slot;
        let Some(path) = self.state_path(slot) else { return };
        let Some(game) = &mut self.game else { return };
        let started = Instant::now();
        let saved = game.system.save_state().and_then(|bytes| {
            let parent = path.parent().expect("a state's file is in a folder");
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            // written beside it first, a state cut short never takes the
            // place of a whole one
            let partial = path.with_extension("state.new");
            std::fs::write(&partial, &bytes).and_then(|()| std::fs::rename(&partial, &path)).map_err(|error| error.to_string())
        });
        match saved {
            Ok(()) => {
                log::info!("saved the state to {} in {:?}", path.display(), started.elapsed());
                self.menus.toast = Some((format!("Saved to slot {slot}"), Instant::now()));
            }
            Err(error) => self.menus.message = Some(format!("The state could not be saved, {error}")),
        }
        // the time it took is not the game's to make up
        self.next_frame = Instant::now();
    }

    /// loads the running game's state from the slot chosen.
    fn load_state(&mut self) {
        let slot = self.menus.slot;
        let Some(path) = self.state_path(slot) else { return };
        let Some(game) = &mut self.game else { return };
        let loaded = std::fs::read(&path)
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => format!("Slot {slot} has no state saved."),
                _ => format!("The state could not be read, {error}"),
            })
            .and_then(|bytes| game.system.load_state(&bytes).map_err(|error| format!("The state could not be loaded, {error}.")));
        match loaded {
            Ok(()) => {
                log::info!("loaded the state from {}", path.display());
                self.menus.toast = Some((format!("Loaded slot {slot}"), Instant::now()));
                self.menus.menu_open = false;
                // the overlay's line was about the memory before, the next
                // frame gives the one about the state
                self.overlay_corner = None;
                if let Some(audio) = &self.audio {
                    audio.clear();
                }
            }
            Err(error) => self.menus.message = Some(error),
        }
        self.next_frame = Instant::now();
    }

    /// the file of the running game's cheats.
    fn cheats_path(&self) -> Option<PathBuf> {
        let program_id = self.game.as_ref()?.system.title.as_ref()?.program_id();
        Some(zakuro_core::cheats::path(self.data_dir.as_ref()?, program_id))
    }

    /// writes the running game's cheats to their file.
    fn save_cheats(&mut self) {
        let (Some(path), Some(game)) = (self.cheats_path(), &self.game) else { return };
        if let Err(error) = zakuro_core::cheats::save(&path, &game.system.cheats) {
            self.menus.message = Some(format!("The cheats can't be saved: {error}"));
        }
    }

    /// the menus, and the actions they asked for, done.
    fn interface(&mut self, event_loop: &ActiveEventLoop) -> Overlay {
        // when the slot chosen was saved to, while the menu shows it
        let saved: Vec<Option<String>> = (1..=menus::STATE_SLOTS as u8)
            .map(|slot| {
                self.menus
                    .menu_open
                    .then(|| self.state_path(slot))
                    .flatten()
                    .and_then(|path| std::fs::metadata(path).ok()?.modified().ok())
                    .map(ago)
            })
            .collect();
        let over = self.bottom_over_frame();
        let pointer = (over.is_some() && self.pad_pointer.shown).then(|| (self.pad_pointer.at, self.gamepads.holding(self.settings.pad.a)));
        let showing = match self.shown_layout() {
            zakuro_gpu::ScreenLayout::TopWithPieces(pieces) => Some(pieces),
            _ => None,
        };
        if showing.is_some() || self.layout != Screens::TopOnly {
            self.last_pieces = showing;
        }
        let pieces = self.last_pieces.and_then(|pieces| self.placed_pieces(pieces));
        let cuts = self.bottom_screen_cuts(pieces.as_ref().map(|(places, _)| places.as_slice()));
        let banners_on = self.banners_on();
        if banners_on {
            self.read_banners();
        }
        let banner_top = self.window_layout(zakuro_gpu::ScreenLayout::TopOnly).and_then(|(top, _)| top);
        let buttons = self.button_places(banner_top);
        let last_banners = &self.last_banners;
        let (Some(gui), Some(window)) = (&mut self.gui, &self.window) else { return Overlay::default() };
        let show_fps = self.settings.show_fps;
        let game = self.game.as_ref().map(|game| (game.name.clone(), game.shown, game.system.recompiled.is_some()));
        let fast = self.fast_forwarding;
        let keyboard = self.game.as_ref().and_then(|game| game.system.keyboard_request().cloned());
        let room = self.room.as_ref().filter(|_| self.menus.multiplayer_open).map(|connection| {
            let members = connection.members();
            menus::RoomView {
                name: connection.room().to_owned(),
                download_play: self.dlp_guest.as_ref().map(|guest| download_play_view(guest, &members, &self.library)),
                members: members.into_iter().map(|member| (member.nickname, member.game)).collect(),
                port: self.room_server.as_ref().map(|server| server.port()),
            }
        });
        let cheats = self.game.as_ref().filter(|_| self.menus.cheats_open).map(|game| {
            game.system
                .cheats
                .iter()
                .map(|cheat| (cheat.name.clone(), cheat.enabled, cheat.notes.join("\n"), cheat.builtin, cheat.setting.as_ref().map(mod_setting)))
                .collect::<Vec<_>>()
        });
        let corner = self.overlay_corner.clone().filter(|_| game.is_some());
        let (menus, library, settings, jobs) = (&mut self.menus, &self.library, &mut self.settings, &self.jobs);
        let banner_textures = &mut self.banner_textures;
        let cursor_idle = self.game.is_some() && self.mouse_used_at.elapsed() >= CURSOR_HIDES_AFTER;
        let mut actions = Vec::new();
        let overlay = gui.frame(window, |ui| {
            match &game {
                Some((name, fps, recompiled)) => {
                    actions.extend(menus.game(ui, name, show_fps.then_some(*fps), *recompiled, fast, &saved));
                    if let Some(text) = &corner {
                        menus::corner(ui.ctx(), text);
                    }
                    let fading = ui.ctx().animate_bool_with_time(egui::Id::new("banners"), banners_on, PIECE_FADE);
                    if let (Some(read), Some(top), true) = (last_banners, banner_top, fading > 0.0) {
                        let ppp = ui.ctx().pixels_per_point();
                        let rect = egui::Rect::from_min_size(egui::pos2(top.x, top.y) / ppp, egui::vec2(top.width, top.height) / ppp);
                        overlays::banners(ui.ctx(), banner_textures, read, rect, fading);
                    }
                    let opacity = ui.ctx().animate_bool_with_time(egui::Id::new("pieces"), showing.is_some(), PIECE_FADE);
                    if let (Some((pieces, edge)), true) = (&pieces, opacity > 0.0) {
                        overlays::pieces(ui.ctx(), pieces, &cuts, edge / ui.ctx().pixels_per_point(), opacity);
                    }
                    if let Some((places, edge, opacity)) = &buttons {
                        overlays::pieces(ui.ctx(), places, &[], edge / ui.ctx().pixels_per_point(), *opacity);
                    }
                    if let Some((bottom, edge, corner)) = over {
                        let ppp = ui.ctx().pixels_per_point();
                        let rect = egui::Rect::from_min_size(egui::pos2(bottom.x, bottom.y) / ppp, egui::vec2(bottom.width, bottom.height) / ppp);
                        overlays::bottom_over(ui.ctx(), rect, edge / ppp, corner / ppp);
                        if let Some((at, pressed)) = pointer {
                            overlays::pad_pointer(ui.ctx(), rect, at, pressed);
                        }
                    }
                }
                None => actions.extend(menus.library(ui, library, settings, jobs)),
            }
            actions.extend(menus.settings(ui.ctx(), settings));
            if menus.multiplayer_open {
                actions.extend(menus.multiplayer(ui.ctx(), settings, room.as_ref()));
            }
            if let Some(cheats) = &cheats {
                actions.extend(menus.cheats(ui.ctx(), cheats, settings.original));
            }
            if let Some(request) = &keyboard {
                actions.extend(menus.keyboard(ui.ctx(), request));
            }
            actions.extend(menus.compiler_offer(ui.ctx()));
            actions.extend(menus.download_offer(ui.ctx()));
            menus.message(ui.ctx());
            if cursor_idle {
                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            }
        });
        self.library.changed = false;
        for action in actions {
            self.act(action, event_loop);
        }
        overlay
    }

    /// Returns where the top and the bottom screen lie in the window for the given layout, or `None` without a window.
    fn window_layout(&self, arrangement: zakuro_gpu::ScreenLayout) -> Option<(Option<zakuro_gpu::Viewport>, Option<zakuro_gpu::Viewport>)> {
        let size = self.window.as_ref()?.inner_size();
        Some(layout(size.width, size.height, arrangement, self.top_shape.width(), self.settings.integer_scale))
    }

    /// Returns the place of the bottom screen over the top screen, with the size of its edge and of its corners in pixels of the window.
    ///
    /// The result is `None` when the bottom screen is not laid over the top screen.
    fn bottom_over_frame(&self) -> Option<(zakuro_gpu::Viewport, f32, f32)> {
        let arrangement = self.shown_layout();
        if !matches!(arrangement, zakuro_gpu::ScreenLayout::BottomOverTop { .. }) {
            return None;
        }
        let (top, bottom) = self.window_layout(arrangement)?;
        let bottom = bottom?;
        Some((bottom, BOTTOM_OVER_EDGE * top?.width / self.top_shape.width(), BOTTOM_OVER_CORNER * bottom.width / 320.0))
    }

    /// Returns where the pieces of the bottom screen lie in the window, with the size of their shadow in pixels of the window.
    fn placed_pieces(&self, pieces: &'static [zakuro_gpu::Piece]) -> Option<(Vec<zakuro_gpu::PiecePlace>, f32)> {
        let top = self.window_layout(zakuro_gpu::ScreenLayout::TopWithPieces(pieces))?.0?;
        Some((zakuro_gpu::piece_places(top, pieces, &self.marker_spans()), PIECE_SHADOW * top.height / 240.0))
    }

    /// Returns, for each place, the runs of pixels that are cut out of the picture of the bottom screen.
    ///
    /// A place without a cut gets an empty list.
    fn bottom_screen_cuts(&mut self, places: Option<&[zakuro_gpu::PiecePlace]>) -> Vec<Vec<[u16; 3]>> {
        let (Some(places), Some(game)) = (places, self.game.as_mut()) else {
            return Vec::new();
        };
        if !places.iter().any(|place| place.cut.is_some()) {
            return Vec::new();
        }
        let picture = game.system.bottom_picture();
        places
            .iter()
            .map(|place| match (place.cut, &picture) {
                (Some(ground), Some(picture)) => {
                    let [left, top, width, height] = place.part;
                    let rect = [left * 320.0, top * 240.0, width * 320.0, height * 240.0].map(|value| value.round().max(0.0) as u16);
                    zakuro_gpu::cut_out(picture, rect, ground)
                }
                _ => Vec::new(),
            })
            .collect()
    }

    /// Returns whether the HD banners are drawn: in a battle, with the banner interface chosen and not in original mode.
    fn banners_on(&self) -> bool {
        matches!(self.shown_layout(), zakuro_gpu::ScreenLayout::TopWithPieces(_) | zakuro_gpu::ScreenLayout::TopOnly)
            && matches!(self.scene, Scene::Battle(_))
            && self.settings.battle_ui == settings::BattleUi::Banners
            && !self.settings.original
    }

    /// Reads the banners from the picture of the bottom screen and keeps the result when the read succeeds.
    fn read_banners(&mut self) {
        let Some(game) = self.game.as_mut() else { return };
        if let (Some(banners), Some(picture)) = (game.system.battle_banners(), game.system.bottom_picture()) {
            let spans = game.system.marker_spans();
            if let Some(read) = banners::read(&picture, banners, &spans) {
                self.last_banners = Some(read);
            }
        }
    }

    /// Returns where the buttons of a battle lie in the window, with the size of their shadow and their opacity.
    ///
    /// The result is `None` while the buttons are invisible.
    fn button_places(&self, top: Option<zakuro_gpu::Viewport>) -> Option<(Vec<zakuro_gpu::PiecePlace>, f32, f32)> {
        let (shown, opacity) = self.buttons.map(|shown| (shown, shown.opacity(Instant::now()))).filter(|&(_, opacity)| opacity > 0.0)?;
        let top = top?;
        Some((zakuro_gpu::piece_places(top, shown.pieces, &self.marker_spans()), PIECE_SHADOW * top.height / 240.0, opacity))
    }

    fn present(&mut self, event_loop: &ActiveEventLoop) {
        self.place_bottom_over();
        let overlay = self.interface(event_loop);
        let blank = |screen: Screen| ((std::sync::Arc::new(Vec::new()), screen.width(), screen.height()), None);
        // the right eye's picture only while the 3D shows
        let stereo = self.settings.stereo != settings::Stereo3d::Off;
        let (top, bottom, right) = match &mut self.game {
            Some(game) => (
                shown(&mut game.system, Screen::Top),
                shown(&mut game.system, Screen::Bottom),
                stereo.then(|| shown_right(&mut game.system)).flatten(),
            ),
            None => (blank(Screen::Top), blank(Screen::Bottom), None),
        };
        let right = right.unwrap_or_else(|| blank(Screen::Top));
        let Some(backend) = &mut self.backend else { return };
        let (((top, top_width, top_height), top_gpu), ((bottom, bottom_width, bottom_height), bottom_gpu)) = (top, bottom);
        let ((right, right_width, right_height), right_gpu) = right;
        let result = backend.present(
            ScreenImage { width: top_width, height: top_height, pixels: &top, gpu: top_gpu },
            ScreenImage { width: bottom_width, height: bottom_height, pixels: &bottom, gpu: bottom_gpu },
            ScreenImage { width: right_width, height: right_height, pixels: &right, gpu: right_gpu },
            &overlay,
        );
        match result {
            Ok(()) | Err(PresentError::OutOfDate) => {}
            Err(error) => {
                log::error!("presentation failed: {error}");
                event_loop.exit();
            }
        }
    }
}

/// what the multiplayer window shows of Download Play: the offers in the
/// room, and where the guest is.
fn download_play_view(guest: &zakuro_core::download_play::Guest, members: &[zakuro_core::room::Member], library: &Library) -> menus::DlpView {
    use zakuro_core::download_play::Phase;
    let offers = guest
        .offers()
        .iter()
        .map(|heard| {
            let offer = &heard.offer;
            let smdh = library::Smdh::parse(&offer.smdh);
            let own = library.children.iter().any(|own| own.title_id == offer.child);
            let blocked = if offer.guests >= offer.most {
                Some("The host's game takes no more guests.".to_owned())
            } else if !offer.sends_child && !own {
                Some(
                    "The game to download is encrypted on the card, so either player needs a decrypted copy of it: \
                     decrypt the .cia in the card's Download Play partition with GodMode9 and put it in the games folder."
                        .to_owned(),
                )
            } else {
                None
            };
            menus::OfferView {
                child: offer.child,
                title: smdh.as_ref().map(|smdh| smdh.name.clone()).filter(|name| !name.is_empty()).unwrap_or_else(|| format!("{:016X}", offer.child)),
                host: members.iter().find(|member| member.mac == heard.host).map(|member| member.nickname.clone()).unwrap_or_default(),
                icon: smdh.map(|smdh| smdh.icon),
                guests: offer.guests,
                most: offer.most,
                blocked,
            }
        })
        .collect();
    let doing = guest.phase().map(|phase| match phase {
        Phase::Asking => ("Asking the host to join.".to_owned(), None),
        Phase::Waiting => ("Joined. The game downloads once the host starts it.".to_owned(), None),
        Phase::Receiving { got, total } => ("Downloading the game.".to_owned(), Some(*got as f32 / (*total).max(1) as f32)),
        Phase::Ready => ("Ready. Waiting for the host to start the game.".to_owned(), None),
        Phase::Begun { .. } => ("Starting the game.".to_owned(), None),
        Phase::Failed(why) => (why.clone(), None),
    });
    menus::DlpView { offers, doing }
}

/// a screen as the presenter takes it, straight from the GPU when it shares
/// the renderer's device and the GPU drew what the screen shows, else its
/// pixels.
fn shown(system: &mut System, screen: Screen) -> (zakuro_core::Screen, Option<GpuScreen>) {
    if let Some(gpu) = system.gpu_screen(screen) {
        let scale = system.gpu.scale();
        return ((std::sync::Arc::new(Vec::new()), screen.width() * scale, screen.height() * scale), Some(gpu));
    }
    (system.read_screen_scaled(screen), None)
}

/// the top screen's picture for the right eye, as shown gives the screens,
/// when the game draws in 3D.
fn shown_right(system: &mut System) -> Option<(zakuro_core::Screen, Option<GpuScreen>)> {
    if let Some(gpu) = system.gpu_right_eye() {
        let scale = system.gpu.scale();
        return Some(((std::sync::Arc::new(Vec::new()), Screen::Top.width() * scale, Screen::Top.height() * scale), Some(gpu)));
    }
    system.read_right_eye_scaled().map(|screen| (screen, None))
}

/// keeps the renderer kind referenced even when a backend feature is off.
const _: RendererKind = RendererKind::Software;

/// Converts the setting of a cheat into the setting that the menus show.
fn mod_setting(cheat_setting: &zakuro_core::cheats::CheatSetting) -> menus::ModSetting {
    let setting = &cheat_setting.setting;
    menus::ModSetting { name: setting.name.clone(), min: setting.min, max: setting.max, value: cheat_setting.value, unit: setting.unit.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a frame goes unshown only while the game runs behind with a
    /// presenter that waits for the display, a few in a row at most.
    #[test]
    fn frames_go_unshown_only_when_showing_would_wait() {
        assert!(skips_showing(true, true, true, 0));
        assert!(skips_showing(true, true, true, MAX_SKIPPED - 1));
        assert!(!skips_showing(true, true, true, MAX_SKIPPED), "a few in a row at most");
        assert!(!skips_showing(true, true, false, 0), "mailbox shows every frame");
        assert!(!skips_showing(true, false, true, 0), "on time");
        assert!(!skips_showing(false, true, true, 0), "paused");
    }
}
