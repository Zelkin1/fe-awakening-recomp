//! The HD text mod.
//! It redraws the text in the pictures of the game in a new font at four times the resolution, using the copy of the game of the player.
//! The pictures can first be cleaned of compression artifacts by an AI model.
//! The result is a texture pack plus a few replacement files for the romfs.

mod banners;
mod clean;
mod data;
mod esrgan;
mod labels;
mod ops;
mod rom;
mod sheets;
mod text;
mod titles;
mod words;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::downloads::{Download, Source};
use crate::recompile::ModStep;
use ops::{Plane, Rgba};

/// The program id of the European release of the game, the only one the mod supports.
pub const PROGRAM_ID: u64 = 0x0004_0000_0009_F100;

/// The install step that builds the HD text.
/// On Windows it needs the cleanup tool, and it downloads the fonts quietly.
pub const STEP: ModStep = ModStep {
    name: "the HD text",
    label: "HD text",
    about: "The game's fonts and the words painted in its pictures, drawn again at four times the resolution from your copy of the game.",
    folder: "hd",
    program_id: PROGRAM_ID,
    needs: if cfg!(windows) { &[clean::NEED] } else { &[] },
    quietly: &[FONTS],
    build,
};

/// The progress message while the cleanup model downloads.
const DOWNLOADING: &str = "Downloading the AI model that cleans the game's pictures.";
/// The progress message while the cleanup runs.
pub const CLEANING: &str = "Real-ESRGAN is removing the compression artifacts from the game's pictures. It runs on your graphics card.";
/// The progress message while the text is redrawn.
const REDRAWING: &str = "Drawing the game's text again in the new font, and enlarging the pictures around it.";

/// The two weights of Alegreya Sans that the text is drawn in.
/// They come from the Google Fonts repository at a fixed commit, so they always match their hashes.
/// The install downloads them without asking, since they are small.
const FONTS: Download = Download {
    folder: "hd-text-fonts-1",
    sources: &[
        Source {
            url: "https://raw.githubusercontent.com/google/fonts/88c2351fb58bf5dfedb96934de48be9a606c7d2e/ofl/alegreyasans/AlegreyaSans-Bold.ttf",
            name: "AlegreyaSans-Bold.ttf",
            size: 266_856,
            sha256: "a3055a1893759bdbd7504bb22abc583769e7974c49353176eac0b03792c9fb8e",
        },
        Source {
            url: "https://raw.githubusercontent.com/google/fonts/88c2351fb58bf5dfedb96934de48be9a606c7d2e/ofl/alegreyasans/AlegreyaSans-ExtraBold.ttf",
            name: "AlegreyaSans-ExtraBold.ttf",
            size: 267_996,
            sha256: "3861f08a77d5c1b3b311055913443b07337a4b7abaff21a8cdb51fc0ae980b9e",
        },
    ],
    finish: None,
};

/// The fonts the text is drawn in.
pub struct Font {
    /// The font for words.
    pub text: ab_glyph::FontArc,
    /// The heavier font for the numbers of the game.
    pub numbers: ab_glyph::FontArc,
    /// The glyph id that the faces use for the lining figures of the digits.
    pub lining_figures: u16,
}

impl Font {
    /// Loads both fonts from the folder they were downloaded to.
    fn read(folder: &Path) -> Result<Font, String> {
        let load = |name: &str| {
            let path = folder.join(name);
            let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            ab_glyph::FontArc::try_from_vec(bytes).map_err(|error| format!("{}: {error}", path.display()))
        };
        Ok(Font { text: load("AlegreyaSans-Bold.ttf")?, numbers: load("AlegreyaSans-ExtraBold.ttf")?, lining_figures: 1686 })
    }
}

/// The language letters of the European release, used in the names of the files: French, German, Italian, Spanish and English.
const LANGUAGES: [char; 5] = ['F', 'G', 'I', 'S', 'U'];

/// The game that is read, as a loaded title.
struct Game {
    title: zakuro_fs::Title,
}

impl Game {
    /// Reads a file of the romfs and decompresses it with the LZ format of the game.
    /// Returns `None` when the file is missing or cannot be unpacked.
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        let romfs = self.title.romfs.as_ref()?;
        let file = romfs.lookup(path).ok()?;
        crate::lz::unpack(self.title.read_romfs(&file, 0, file.data_size as usize)?).ok()
    }

    /// Lists the sorted names of the files in a folder of the romfs.
    fn list(&self, path: &str) -> Vec<String> {
        let Some(romfs) = self.title.romfs.as_ref() else { return Vec::new() };
        let Ok(mut dir) = romfs.root() else { return Vec::new() };
        // The files of the game sit in a folder without a name when there is one.
        if let Some((_, nameless)) = romfs.subdirs(&dir).into_iter().find(|(_, d)| d.name.is_empty()) {
            dir = nameless;
        }
        for part in path.split('/') {
            let Some((_, next)) = romfs.subdirs(&dir).into_iter().find(|(_, d)| d.name == part) else { return Vec::new() };
            dir = next;
        }
        let mut names: Vec<String> = romfs.files(&dir).into_iter().map(|(_, file)| file.name).collect();
        names.sort();
        names
    }
}

/// A picture to write into the texture pack.
pub struct Picture {
    /// The file name of the picture, which encodes the hash that the emulator looks textures up by.
    pub name: String,
    /// The pixels of the picture.
    pub image: Rgba,
}

/// Turns a plane of coverage values into a white picture whose alpha is that coverage.
fn alpha_picture(name: String, alpha: &Plane) -> Picture {
    let mut image = Rgba::new(alpha.w, alpha.h);
    for (p, &a) in image.v.iter_mut().zip(&alpha.v) {
        *p = [1.0, 1.0, 1.0, a.clamp(0.0, 1.0)];
    }
    Picture { name, image }
}

/// What a job produces.
#[derive(Default)]
pub struct Made {
    /// The pictures for the texture pack.
    pub pictures: Vec<Picture>,
    /// The replacement files for the romfs, each with its path and its bytes.
    pub files: Vec<(String, Vec<u8>)>,
}

/// A unit of work that the worker threads run.
type Job<'a> = Box<dyn Fn() -> Made + Send + Sync + 'a>;
/// The cleaned pictures by name, set once after the cleanup has run.
type Cleaned = OnceLock<HashMap<String, Rgba>>;

/// Returns the cleaned version of a texture when there is one, and the plain decoded texture otherwise.
fn source(texture: &rom::Texture, cleaned: &Cleaned) -> Rgba {
    cleaned.get().and_then(|pictures| pictures.get(&texture.name())).cloned().unwrap_or_else(|| texture.rgba())
}

/// Collects every distinct texture in the cleanup folders that is stored in compressed blocks, as a name and a picture.
fn cleanup_pictures(game: &Game) -> Vec<(String, Rgba)> {
    let mut seen = HashSet::new();
    let mut pictures = Vec::new();
    for folder in clean::FOLDERS {
        for file in game.list(folder) {
            let Some(bytes) = game.read(&format!("{folder}/{file}")) else { continue };
            for (_, texture) in rom::textures_in(&bytes) {
                if clean::block_compressed(texture.format) && seen.insert(texture.name()) {
                    pictures.push((texture.name(), texture.rgba()));
                }
            }
        }
    }
    pictures
}

/// Builds the job that redraws one bitmap font of the game, together with its sheets.
fn font_job<'a>(game: &'a Game, face: &'a text::Face, name: &'static str) -> Job<'a> {
    Box::new(move || {
        let path = format!("fonts/{name}.bfnt.lz");
        let Some(data) = game.read(&path) else { return Made::default() };
        let Some(font) = sheets::parse(&data) else { return Made::default() };
        // Format 11 is the 3DS alpha-only texture format with four bits per pixel.
        let images: Vec<Plane> = font.sheets.iter().filter_map(|&at| rom::texture(&data, at, font.width, font.height, 11)).map(|t| t.alpha()).collect();
        let style = match name {
            "System" => sheets::Style { measure: sheets::Measure::Capitals, natural_space: true, keep_digit_advance: false },
            _ => sheets::Style { measure: sheets::Measure::Digits, natural_space: false, keep_digit_advance: true },
        };
        let Some(redrawn) = sheets::redraw(face, &data, &font, &images, &style) else {
            log::info!("the HD text keeps the game's {name} font, its letters don't fit its sheets");
            return Made::default();
        };
        let pictures = redrawn.sheets.iter().map(|(hash, p)| alpha_picture(format!("tex1_{}x{}_{hash:016X}_11.png", font.width, font.height), p)).collect();
        Made { pictures, files: vec![(path, crate::lz::lz13(&redrawn.file))] }
    })
}

/// Builds the job that redraws the chapter title pictures of one file in one language.
fn title_job<'a>(game: &'a Game, face: &'a text::Face, messages: &'a HashMap<String, String>, lang: char, file: String) -> Job<'a> {
    Box::new(move || {
        let Some(name) = file.strip_suffix(".ctpk.lz") else { return Made::default() };
        let Some(texts) = titles::texts(messages, name, lang, data::PLACES, data::NAMES) else { return Made::default() };
        let Some(bytes) = game.read(&format!("chapter/title/{lang}/{file}")) else { return Made::default() };
        let pictures = rom::textures_in(&bytes).into_iter().filter_map(|(_, texture)| Some(Picture { name: texture.name(), image: titles::redraw(&texture.rgba(), &texts, face)? })).collect();
        Made { pictures, files: Vec::new() }
    })
}

/// Builds the job that redraws the banners of one telop file in one language.
fn banner_job<'a>(game: &'a Game, face: &'a text::Face, lang: char, name: &'static str) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read(&format!("telop/Tlp_{name}_{lang}.r.lz")) else { return Made::default() };
        let mut textures = rom::textures_in(&bytes);
        // The specs refer to the textures by their order in the file.
        textures.sort_by_key(|(at, _)| *at);
        let Some((_, words)) = data::BANNER_TEXTS.iter().find(|(l, _)| *l == lang) else { return Made::default() };
        let mut layouts: Vec<((usize, usize), Plane)> = Vec::new();
        let mut pictures = Vec::new();
        for &(_, index, kind) in data::BANNER_SPECS.iter().filter(|(n, _, _)| *n == name) {
            let Some((_, texture)) = textures.get(index) else { continue };
            // The mission files hold several banners, so their keys carry the index of the texture.
            let key = if name.starts_with("Mission") { format!("{name}.{index}") } else { name.to_string() };
            let Some(text) = data::BANNER_KEYS.iter().position(|k| *k == key).and_then(|i| words.get(i)) else { continue };
            let kind = match kind {
                "glow" => banners::Kind::Glow,
                "stone" => banners::Kind::Stone,
                _ => banners::Kind::Plain,
            };
            let image = texture.rgba();
            let size = (texture.width, texture.height);
            // Glow banners of the same size reuse the layout of an earlier banner, so that they match each other.
            let reused = (kind == banners::Kind::Glow).then(|| layouts.iter().find(|(s, _)| *s == size).map(|(_, l)| l.clone())).flatten();
            let Some(layout) = reused.or_else(|| banners::layout(face, text, &banners::core(&image), &banners::TRACKINGS)) else { continue };
            if kind != banners::Kind::Glow {
                layouts.push((size, layout.clone()));
            }
            pictures.push(Picture { name: texture.name(), image: banners::redraw(&image, &banners::core(&image), kind, banners::VIVID, &layout) });
        }
        Made { pictures, files: Vec::new() }
    })
}

/// Builds the job that redraws the labels of one status atlas.
fn label_job<'a>(face: &'a text::Face, texture: rom::Texture, list: &'static [data::Label], cleaned: &'a Cleaned) -> Job<'a> {
    Box::new(move || {
        let image = source(&texture, cleaned);
        let mut out = labels::redraw(&image, list, face);
        labels::mirror_copies(&texture.texels, texture.width, texture.height, &mut out, list, &labels::sharp_enlarge(&image, 8.0));
        Made { pictures: vec![Picture { name: texture.name(), image: out }], files: Vec::new() }
    })
}

/// Builds the job that replaces words in the textures of one file.
/// When `at` is set, only the texture at that offset is used.
fn words_job<'a>(game: &'a Game, face: &'a text::Face, path: String, at: Option<usize>, words: Vec<(words::Box4, &'static str)>, keep: &'static [words::Box4]) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read(&path) else { return Made::default() };
        let pictures = rom::textures_in(&bytes)
            .into_iter()
            .filter(|(offset, _)| at.is_none_or(|at| *offset == at))
            .filter_map(|(_, texture)| Some(Picture { name: texture.name(), image: words::redraw(&texture.rgba(), &words, keep, words::SOFT, face)? }))
            .collect();
        Made { pictures, files: Vec::new() }
    })
}

/// The number of class names in each texture of the class name pictures, one per row of 32 pixels.
const CLASS_ROWS: usize = 16;
/// The slant of the class names, as a shear factor.
const CLASS_SLANT: f32 = 0.2;

/// Reads the names of the classes in one language, in the order of the pictures.
/// The spellings of the game are replaced by the full spellings that fit the pictures.
fn class_names(game: &Game, lang: char) -> Vec<String> {
    let Some(bytes) = game.read(&format!("m/{lang}/GameData.bin.lz")) else { return Vec::new() };
    rom::labelled(&bytes)
        .into_iter()
        // The MJID keys are the class names, and the MJID_H keys are left out.
        .filter(|(key, _)| key.starts_with("MJID_") && !key.starts_with("MJID_H_"))
        // The first entry has no row in the pictures.
        .skip(1)
        .map(|(_, name)| data::CLASS_SPELLINGS.iter().filter(|(l, _, _)| *l == lang).fold(name, |n, (_, short, full)| n.replace(short, full)))
        .collect()
}

/// Builds the job that redraws the class name pictures in one language, in a slanted face.
fn class_job<'a>(game: &'a Game, face: &'a text::Face, lang: char) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read(&format!("jobintro/jobname_{lang}.ctpk.lz")) else { return Made::default() };
        let names = class_names(game, lang);
        let slanted = face.slanted(CLASS_SLANT);
        let mut textures = rom::textures_in(&bytes);
        textures.sort_by_key(|(at, _)| *at);
        let mut pictures = Vec::new();
        let mut next = 0;
        for (_, texture) in textures {
            let rows = (texture.height / 32).min(CLASS_ROWS);
            let words: Vec<(words::Box4, &str)> = (0..rows).filter_map(|r| Some(([0, r * 32, texture.width, r * 32 + 32], names.get(next + r)?.as_str()))).collect();
            next += rows;
            if let Some(image) = words::redraw(&texture.rgba(), &words, &[], banners::VIVID, &slanted) {
                pictures.push(Picture { name: texture.name(), image });
            }
        }
        Made { pictures, files: Vec::new() }
    })
}

/// Builds the job that redraws the logo of the title screen.
fn logo_job<'a>(game: &'a Game, face: &'a text::Face) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read("telop/Tlp_Title_01_U.r.lz") else { return Made::default() };
        let pictures = rom::textures_in(&bytes).into_iter().filter(|(at, _)| *at == 0xB6A00).map(|(_, t)| Picture { name: t.name(), image: words::logo(&t.rgba(), face) }).collect();
        Made { pictures, files: Vec::new() }
    })
}

/// Builds the job that redraws the buttons of the world map in one language.
fn gmap_job<'a>(game: &'a Game, face: &'a text::Face, lang: char, cleaned: &'a Cleaned) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read(&format!("gmap/gmap_{lang}.ctpk.lz")) else { return Made::default() };
        let (Some((_, names)), Some((_, pills, at)), Some((_, icons))) = (data::GMAP_WORDS.iter().find(|(l, _)| *l == lang), data::GMAP_PILLS.iter().find(|(l, _, _)| *l == lang), data::GMAP_ICONS.iter().find(|(l, _)| *l == lang)) else { return Made::default() };
        let mut pictures = Vec::new();
        // The two textures of the map that hold words are told apart by their offset in the file.
        for (offset, texture) in rom::textures_in(&bytes) {
            let (image, entries): (_, Vec<words::Word>) = match offset {
                0x180 => (words::calm(&source(&texture, cleaned)), words::GMAP_BUTTONS.iter().map(|&(b, i, ink)| words::Word::flat(b, names[i], ink)).collect()),
                0x10180 => (source(&texture, cleaned), [([at[0], 374, 184, 395], pills[0]), (words::HALT_BOX, pills[1]), ([at[1], 437, at[2], 458], pills[2]), ([at[3], 437, at[4], 458], pills[3]), ([at[5], 437, at[6], 458], pills[4]), ([icons[0] + 2, 444, icons[1] - 2, 453], "X"), ([icons[2], 440, icons[3], 455], "R"), ([icons[4], 440, icons[5], 455], "START"), ([icons[6] + 1, 379, icons[7] - 1, 390], "B"), (words::BUBBLE_A, "A")].into_iter().map(|(b, t)| words::Word::flat(b, t, words::Ink::Light)).collect()),
                _ => continue,
            };
            pictures.push(Picture { name: texture.name(), image: words::buttons(&image, &entries, face, words::sharp_art) });
        }
        Made { pictures, files: Vec::new() }
    })
}

/// Builds the job that redraws the words of the texture at one offset of a file.
fn buttons_job<'a>(game: &'a Game, face: &'a text::Face, path: String, at: usize, entries: Vec<words::Word>, enlarge: fn(&Rgba) -> Rgba, cleaned: &'a Cleaned) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read(&path) else { return Made::default() };
        let pictures = rom::textures_in(&bytes).into_iter().filter(|(offset, _)| *offset == at).map(|(_, t)| Picture { name: t.name(), image: words::buttons(&source(&t, cleaned), &entries, face, enlarge) }).collect();
        Made { pictures, files: Vec::new() }
    })
}

/// Writes a picture as a PNG into the folder of the texture pack.
/// The fast compression keeps the many large pictures quick to write.
fn write_picture(out: &Path, picture: &Picture) -> Result<(), String> {
    use image::ImageEncoder;
    let path = out.join(&picture.name);
    let file = std::fs::File::create(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let encoder = image::codecs::png::PngEncoder::new_with_quality(std::io::BufWriter::new(file), image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive);
    encoder
        .write_image(&picture.image.to_bytes(), picture.image.w as u32, picture.image.h as u32, image::ExtendedColorType::Rgba8)
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// Writes a file below the root, creating the folders it needs.
fn write_file(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(&target, bytes).map_err(|error| format!("{}: {error}", target.display()))
}

/// Builds the whole HD text from the ROM into the folder `root`, with a `textures` pack and a `romfs` folder.
/// It cleans the pictures first when there is a cleaner, then runs all the redraw jobs on worker threads.
/// Returns a short summary, or an error message, and `"cancelled"` when the cancel flag is set.
pub fn generate(rom: &Path, root: &Path, font: &Font, cleaner: Option<&clean::Cleaner>, progress: &(dyn Fn(&'static str, usize, usize) + Sync), cancel: &AtomicBool) -> Result<String, String> {
    let title = zakuro_fs::Title::load(rom).map_err(|error| format!("{}: {error}", rom.display()))?;
    if title.program_id() != PROGRAM_ID {
        return Err("the HD text is made for the European release".into());
    }
    let game = Game { title };
    // The numbers face has the same size, so that numbers and words line up.
    let face = text::Face::new(font.text.clone(), 40.0).with_figures(font.lining_figures);
    let numbers = text::Face::new(font.numbers.clone(), 40.0).with_figures(font.lining_figures);
    let mut names: Vec<&'static str> = data::BANNER_SPECS.iter().map(|(n, _, _)| *n).collect();
    // The specs of one file are next to each other, so this leaves each file name once.
    names.dedup();
    let cleaned: Cleaned = OnceLock::new();
    let messages: HashMap<char, HashMap<String, String>> = LANGUAGES.iter().map(|&lang| (lang, game.read(&format!("m/{lang}/GameData.bin.lz")).map(|d| rom::messages(&d)).unwrap_or_default())).collect();
    let mut jobs: Vec<Job> = Vec::new();
    for lang in LANGUAGES {
        let atlases = game.read(&format!("ui/status_{lang}.ctpk.lz")).map(|bytes| rom::textures_in(&bytes)).unwrap_or_default();
        for (at, texture) in atlases {
            if let Some((_, _, list)) = data::LABELS.iter().find(|(l, offset, _)| *l == lang && *offset == at) {
                jobs.push(label_job(&face, texture, list, &cleaned));
            }
        }
    }
    jobs.push(logo_job(&game, &face));
    jobs.push(words_job(&game, &face, "telop/Tlp_Title_02_U.r.lz".into(), Some(0x44D80), vec![(words::COPYRIGHT_BOX, data::COPYRIGHT)], &[]));
    for (lang, text) in data::PRESS_ANY_BUTTON {
        jobs.push(words_job(&game, &face, format!("bs/press_{lang}.tp.lz"), None, vec![(words::PRESS_ANY_BUTTON_BOX, *text)], &words::PRESS_ANY_BUTTON_KEEP));
    }
    for (lang, texts) in data::MAIN_MENU {
        jobs.push(words_job(&game, &face, format!("ui/filemenu_{lang}.ctpk.lz"), Some(0x20180), words::MENU_BOXES.into_iter().zip(texts.iter().copied()).collect(), &words::MENU_KEEP));
    }
    for name in ["System", "UI_L", "UI_M", "UI_S"] {
        jobs.push(font_job(&game, if name == "System" { &face } else { &numbers }, name));
    }
    for lang in LANGUAGES {
        for &name in &names {
            jobs.push(banner_job(&game, &face, lang, name));
        }
        jobs.push(class_job(&game, &face, lang));
        jobs.push(gmap_job(&game, &face, lang, &cleaned));
        if let Some((_, names)) = data::OVERVIEW_WORDS.iter().find(|(l, _)| *l == lang) {
            jobs.push(buttons_job(&game, &face, format!("ui/status_{lang}.ctpk.lz"), 0x100, words::OVERVIEW.iter().map(|&(b, i, ink)| words::Word::flat(b, names[i], ink)).collect(), words::sharp_art, &cleaned));
        }
        if let Some(&(_, close)) = data::TUTORIAL_WORDS.iter().find(|(l, _)| *l == lang) {
            let names = [close, "START", "L", "R"];
            jobs.push(buttons_job(&game, &face, format!("ui/tutorial_{lang}.ctpk.lz"), 0x100, words::TUTORIAL.iter().map(|&(b, i, ink)| words::Word::flat(b, names[i], ink)).collect(), words::sharp_art, &cleaned));
        }
        if let Some((_, names)) = data::FORECAST_WORDS.iter().find(|(l, _)| *l == lang) {
            let icons = words::SYSTEM_ICONS.iter().map(|&(b, text)| words::Word::flat(b, text, words::Ink::Light));
            let forecast = words::FORECAST.iter().map(|&(at, i, ink, look)| words::Word { at, text: names[i], ink, look, align: words::Align::Centre, slant: 0.0 });
            jobs.push(buttons_job(&game, &face, format!("ui/system_{lang}.ctpk.lz"), 0x46300, icons.chain(forecast).collect(), words::faithful_art, &cleaned));
        }
        if let Some((_, names)) = data::UNIT_WORDS.iter().find(|(l, _)| *l == lang) {
            let entries = words::UNIT.iter().map(|&(at, i, ink, look, align, slant)| words::Word { at, text: names[i], ink, look, align, slant }).collect();
            jobs.push(buttons_job(&game, &face, format!("ui/system_{lang}.ctpk.lz"), 0x26300, entries, words::unit_art, &cleaned));
        }
        if let Some((_, names)) = data::RADAR_WORDS.iter().find(|(l, _)| *l == lang) {
            let entries = words::RADAR.iter().map(|&(at, i, align)| words::Word { at, text: names[i], ink: words::Ink::Light, look: words::Look::Flat, align, slant: 0.0 }).collect();
            jobs.push(buttons_job(&game, &face, format!("bs/intro_{lang}.tp.lz"), 0x900, entries, words::faithful_art, &cleaned));
        }
        if let Some((_, names)) = data::CARD_WORDS.iter().find(|(l, _)| *l == lang) {
            let entries = words::CARDS.iter().map(|&(at, i, align)| words::Word { at, text: names[i], ink: words::Ink::Light, look: words::Look::Shadow, align, slant: 0.0 }).collect();
            jobs.push(buttons_job(&game, &face, format!("ui/filemenu_{lang}.ctpk.lz"), 0x80180, entries, words::faithful_art, &cleaned));
        }
        for file in game.list(&format!("chapter/title/{lang}")) {
            jobs.push(title_job(&game, &face, &messages[&lang], lang, file));
        }
    }
    // Start from empty output folders, so that nothing of an earlier run stays behind.
    let out = root.join("textures");
    let romfs = root.join("romfs");
    for folder in [&out, &romfs] {
        if folder.exists() {
            std::fs::remove_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        }
    }
    std::fs::create_dir_all(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    let pictures = if cleaner.is_some() { cleanup_pictures(&game) } else { Vec::new() };
    let total = pictures.len() + jobs.len();
    progress(if pictures.is_empty() { REDRAWING } else { CLEANING }, 0, total);
    // A failed cleanup is not fatal: the pictures are then used as they are.
    let mut made_clean = HashMap::new();
    if let Some(cleaner) = cleaner.filter(|_| !pictures.is_empty()) {
        match cleaner.clean(&pictures, &|done| progress(CLEANING, done.min(pictures.len()), total), cancel) {
            Ok(pictures) => made_clean = pictures,
            Err(_) if cancel.load(Ordering::Relaxed) => return Err("cancelled".into()),
            Err(error) => log::warn!("the HD text leaves the game's pictures as they are, {error}"),
        }
    }
    // The cleaned pictures are part of the pack too, so that the parts of the game that are not redrawn also get cleaned.
    for (name, image) in &made_clean {
        write_picture(&out, &Picture { name: name.clone(), image: image.clone() })?;
    }
    let base = pictures.len();
    let _ = cleaned.set(made_clean);
    let (next, done, written, failure) = (AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), Mutex::new(None));
    // Each worker takes the next job from a shared counter until none are left or the install is cancelled.
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).min(jobs.len()).max(1);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= jobs.len() || cancel.load(Ordering::Relaxed) {
                    break;
                }
                let made = jobs[i]();
                for picture in &made.pictures {
                    match write_picture(&out, picture) {
                        Ok(()) => {
                            written.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(error) => *failure.lock().unwrap() = Some(error),
                    }
                }
                for (path, bytes) in &made.files {
                    if let Err(error) = write_file(&romfs, path, bytes) {
                        *failure.lock().unwrap() = Some(error);
                    }
                }
                progress(REDRAWING, base + done.fetch_add(1, Ordering::Relaxed) + 1, total);
            });
        }
    });
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    if let Some(error) = failure.into_inner().unwrap() {
        return Err(error);
    }
    // The pack file tells the emulator to look textures up by the new hash.
    std::fs::write(out.join("pack.json"), "{\n  \"options\": {\n    \"use_new_hash\": true\n  }\n}\n").map_err(|error| error.to_string())?;
    let smoothed = cleaned.get().map_or(0, HashMap::len);
    Ok(format!("{} pictures of the game's text, drawn again, {smoothed} of its pictures cleaned", written.load(Ordering::Relaxed)))
}

/// The entry point of the install step.
/// It downloads the fonts and, on Windows, the cleanup tool, then generates the HD text.
fn build(rom: &Path, data_dir: &Path, progress: &(dyn Fn(&'static str, usize, usize) + Sync), cancel: &AtomicBool) -> Result<String, String> {
    let tools = crate::zig::tools_dir(Some(data_dir)).ok_or("the HD text has no folder to download into")?;
    let font = Font::read(&FONTS.ensure(&tools, &|_| {}, cancel)?)?;
    let cleaner = if cfg!(windows) { Some(clean::Cleaner::in_folder(&clean::NEED.download.ensure(&tools, &|arrived| progress(DOWNLOADING, (arrived * 1000.0) as usize, 1000), cancel)?)) } else { None };
    generate(rom, &STEP.folder_in(data_dir), &font, cleaner.as_ref(), progress, cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the HD text for a whole ROM, as set by environment variables.
    /// It is ignored by default, because it needs a copy of the game and takes a long time.
    #[test]
    #[ignore]
    fn the_whole_game() {
        let (Ok(rom), Ok(out), Ok(fonts)) = (std::env::var("ZAKURO_HD_ROM"), std::env::var("ZAKURO_HD_OUT"), std::env::var("ZAKURO_HD_FONTS")) else {
            panic!("set ZAKURO_HD_ROM, ZAKURO_HD_OUT and ZAKURO_HD_FONTS");
        };
        let font = Font::read(Path::new(&fonts)).unwrap();
        let started = std::time::Instant::now();
        let cleaner = std::env::var("ZAKURO_HD_CLEANER").ok().map(|folder| clean::Cleaner::in_folder(Path::new(&folder)));
        let said = generate(Path::new(&rom), Path::new(&out), &font, cleaner.as_ref(), &|_, _, _| {}, &AtomicBool::new(false)).unwrap();
        println!("{said} in {:.1} s", started.elapsed().as_secs_f64());
    }
}
