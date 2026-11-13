//! command line parsing.

use zakuro_gpu::RendererKind;

use crate::settings::{Screens, TopShape};

#[derive(Debug, Clone)]
pub struct Options {
    /// the game to start, none to open the library.
    pub rom: Option<String>,
    /// the settings' choice when not given.
    pub renderer: Option<RendererKind>,
    /// window scale relative to the console's 400x480 combined screens, the
    /// settings' when not given.
    pub scale: Option<u32>,
    /// run without a window for this many frames, then report.
    pub headless: Option<u64>,
    pub profile: bool,
    pub new3ds: bool,
    /// skip loading a ROM entirely and just paint both screens solid colors.
    pub test_pattern: bool,
    /// a library of recompiled code for the title, or a directory of them.
    pub recompiled: Option<String>,
    /// draw on the host GPU rather than in software, the settings' choice
    /// when not given.
    pub hardware_rasterizer: Option<bool>,
    /// where saves live, instead of the usual place.
    pub data: Option<String>,
    /// interpret everything, whatever recompiled code there is.
    pub interpreter: bool,
    /// play no sound.
    pub mute: bool,
    /// The arrangement of the screens, none to use the saved setting.
    pub layout: Option<Screens>,
    /// The shape of the top screen, none to use the saved setting.
    pub top_shape: Option<TopShape>,
    /// The names of the enhancements of the game to turn on.
    pub enhance: Vec<String>,
}

const USAGE: &str = "\
zakuro - a high-level-emulation Nintendo 3DS emulator

usage: zakuro [rom.3ds|.cxi] [options]

Without a game it opens the library, the games in a folder you pick. The
options below override the settings for this run.

options:
  --renderer <vulkan|gl|software>  presentation backend (default: vulkan,
                                   or gl where Vulkan does not start)
  --rasterizer <hardware|software> where the 3D is drawn (default: hardware,
                                   the host GPU through Vulkan, or software
                                   where that does not start)
  --scale <n>                      window scale factor (default: 2)
  (Esc opens the menu over a game, F1 pauses, F11 goes fullscreen)
  --headless <frames>              run without a window and print a report
  --new3ds                         emulate a New 3DS
  --profile                        collect a sampling profile and print it
  --recompiled <path>              run code 3dsrecomp built for the title, a
                                   library or a directory holding <title id>.so
                                   (.dll on Windows, default: the one 3dsrecomp
                                   build installed for it, in the system's
                                   place for data, ~/.local/share/3dsrecomp or
                                   %APPDATA%\\3dsrecomp)
  --interpreter                    interpret everything, even with recompiled
                                   code around
  --mute                           play no sound
  --layout <stacked|side-by-side|large-top|top|bottom>
                                   the arrangement of the screens (F9 goes
                                   to the next arrangement, F10 changes to
                                   the other screen when only one screen
                                   shows)
  --top-shape <console|16:9|21:9>  the shape of the top screen (a shape
                                   wider than that of the console needs a
                                   widescreen code)
  --enhance <name>[,<name>...]     turn on the enhancements of the game
                                   with these names (the names that the
                                   Cheats window shows for the game)
  --data <dir>                     where saves live (default: the system's
                                   place for data, ~/.local/share/zakuro or
                                   %APPDATA%\\zakuro)
  -h, --help                       show this message

The system font cannot be generated: put a dump at sysdata/shared_font.bin, in
the working directory or where saves live, for titles that render text with it.
";

pub fn parse() -> Result<Options, String> {
    let mut rom = None;
    let mut options = Options {
        rom: None,
        renderer: None,
        scale: None,
        headless: None,
        profile: false,
        new3ds: false,
        test_pattern: false,
        recompiled: None,
        hardware_rasterizer: None,
        data: None,
        interpreter: false,
        mute: false,
        layout: None,
        top_shape: None,
        enhance: Vec::new(),
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "--renderer" => {
                let value = args.next().ok_or("--renderer needs a value")?;
                options.renderer = Some(
                    RendererKind::parse(&value).ok_or_else(|| format!("unknown renderer '{value}'"))?,
                );
            }
            "--scale" => {
                let value = args.next().ok_or("--scale needs a value")?;
                options.scale = Some(value.parse().map_err(|_| format!("'{value}' is not a scale"))?);
            }
            "--headless" => {
                let value = args.next().ok_or("--headless needs a frame count")?;
                options.headless = Some(
                    value
                        .parse()
                        .map_err(|_| format!("'{value}' is not a frame count"))?,
                );
            }
            "--new3ds" => options.new3ds = true,
            "--rasterizer" => {
                let value = args.next().ok_or("--rasterizer needs a value")?;
                options.hardware_rasterizer = Some(match value.as_str() {
                    "hardware" | "vulkan" | "gpu" => true,
                    "software" | "cpu" => false,
                    other => return Err(format!("unknown rasterizer '{other}'")),
                });
            }
            "--recompiled" => {
                options.recompiled = Some(args.next().ok_or("--recompiled needs a path")?);
            }
            "--data" => {
                options.data = Some(args.next().ok_or("--data needs a directory")?);
            }
            "--interpreter" => options.interpreter = true,
            "--mute" => options.mute = true,
            "--layout" => {
                let value = args.next().ok_or("--layout needs a value")?;
                options.layout = Some(match value.as_str() {
                    "stacked" => Screens::Stacked,
                    "side-by-side" => Screens::SideBySide,
                    "large-top" => Screens::LargeTop,
                    "top" => Screens::TopOnly,
                    "bottom" => Screens::BottomOnly,
                    other => return Err(format!("unknown layout '{other}'")),
                });
            }
            "--top-shape" => {
                let value = args.next().ok_or("--top-shape needs a value")?;
                options.top_shape = Some(match value.as_str() {
                    // The top screen of the console is 400 by 240 pixels, which is 5:3.
                    "console" | "5:3" => TopShape::Console,
                    "16:9" => TopShape::Wide16x9,
                    "21:9" => TopShape::Wide21x9,
                    other => return Err(format!("unknown top screen shape '{other}'")),
                });
            }
            "--enhance" => {
                let value = args.next().ok_or("--enhance needs a name")?;
                // The names are separated by commas, and empty entries are dropped.
                options.enhance.extend(value.split(',').map(str::trim).filter(|name| !name.is_empty()).map(str::to_owned));
            }
            "--profile" => options.profile = true,
            "--test-pattern" => options.test_pattern = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option '{other}'"));
            }
            path => rom = Some(path.to_owned()),
        }
    }

    options.rom = rom;
    if options.headless.is_some() && options.rom.is_none() && !options.test_pattern {
        print!("{USAGE}");
        return Err("--headless needs a game".to_owned());
    }
    Ok(options)
}
