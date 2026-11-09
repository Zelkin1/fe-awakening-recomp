mod banners;
mod data;
mod labels;
mod ops;
mod rom;
mod sheets;
mod text;
mod titles;
mod words;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::recompile::ModStep;
use ops::{Plane, Rgba};

pub const PROGRAM_ID: u64 = 0x0004_0000_0009_F100;

pub const STEP: ModStep = ModStep {
    name: "the HD text",
    label: "HD text",
    about: "The game's fonts and the words painted in its pictures, drawn again at four times the resolution from your copy of the game.",
    folder: "hd",
    program_id: PROGRAM_ID,
    build,
};

pub struct Font {
    pub bytes: &'static [u8],
    pub lining_figures: u16,
}

const LANGUAGES: [char; 5] = ['F', 'G', 'I', 'S', 'U'];

struct Game {
    title: zakuro_fs::Title,
}

impl Game {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        let romfs = self.title.romfs.as_ref()?;
        let file = romfs.lookup(path).ok()?;
        crate::lz::unpack(self.title.read_romfs(&file, 0, file.data_size as usize)?).ok()
    }

    fn list(&self, path: &str) -> Vec<String> {
        let Some(romfs) = self.title.romfs.as_ref() else { return Vec::new() };
        let Ok(mut dir) = romfs.root() else { return Vec::new() };
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

pub struct Picture {
    pub name: String,
    pub image: Rgba,
}

fn alpha_picture(name: String, alpha: &Plane) -> Picture {
    let mut image = Rgba::new(alpha.w, alpha.h);
    for (p, &a) in image.v.iter_mut().zip(&alpha.v) {
        *p = [1.0, 1.0, 1.0, a.clamp(0.0, 1.0)];
    }
    Picture { name, image }
}

#[derive(Default)]
pub struct Made {
    pub pictures: Vec<Picture>,
    pub files: Vec<(String, Vec<u8>)>,
}

type Job<'a> = Box<dyn Fn() -> Made + Send + Sync + 'a>;

fn font_job<'a>(game: &'a Game, face: &'a text::Face, name: &'static str) -> Job<'a> {
    Box::new(move || {
        let path = format!("fonts/{name}.bfnt.lz");
        let Some(data) = game.read(&path) else { return Made::default() };
        let Some(font) = sheets::parse(&data) else { return Made::default() };
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

fn title_job<'a>(game: &'a Game, face: &'a text::Face, messages: &'a HashMap<String, String>, lang: char, file: String) -> Job<'a> {
    Box::new(move || {
        let Some(name) = file.strip_suffix(".ctpk.lz") else { return Made::default() };
        let Some(texts) = titles::texts(messages, name, lang, data::PLACES, data::NAMES) else { return Made::default() };
        let Some(bytes) = game.read(&format!("chapter/title/{lang}/{file}")) else { return Made::default() };
        let pictures = rom::textures_in(&bytes).into_iter().filter_map(|(_, texture)| Some(Picture { name: texture.name(), image: titles::redraw(&texture.rgba(), &texts, face)? })).collect();
        Made { pictures, files: Vec::new() }
    })
}

fn banner_job<'a>(game: &'a Game, face: &'a text::Face, lang: char, name: &'static str) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read(&format!("telop/Tlp_{name}_{lang}.r.lz")) else { return Made::default() };
        let mut textures = rom::textures_in(&bytes);
        textures.sort_by_key(|(at, _)| *at);
        let Some((_, words)) = data::BANNER_TEXTS.iter().find(|(l, _)| *l == lang) else { return Made::default() };
        let mut layouts: Vec<((usize, usize), Plane)> = Vec::new();
        let mut pictures = Vec::new();
        for &(_, index, kind) in data::BANNER_SPECS.iter().filter(|(n, _, _)| *n == name) {
            let Some((_, texture)) = textures.get(index) else { continue };
            let key = if name.starts_with("Mission") { format!("{name}.{index}") } else { name.to_string() };
            let Some(text) = data::BANNER_KEYS.iter().position(|k| *k == key).and_then(|i| words.get(i)) else { continue };
            let kind = match kind {
                "glow" => banners::Kind::Glow,
                "stone" => banners::Kind::Stone,
                _ => banners::Kind::Plain,
            };
            let image = texture.rgba();
            let size = (texture.width, texture.height);
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

fn label_job<'a>(face: &'a text::Face, texture: rom::Texture, list: &'static [data::Label]) -> Job<'a> {
    Box::new(move || {
        let image = texture.rgba();
        let mut out = labels::redraw(&image, list, face);
        labels::mirror_copies(&texture.texels, texture.width, texture.height, &mut out, list, &labels::sharp_enlarge(&image, 8.0));
        Made { pictures: vec![Picture { name: texture.name(), image: out }], files: Vec::new() }
    })
}

fn words_job<'a>(game: &'a Game, face: &'a text::Face, path: String, at: Option<usize>, words: Vec<(words::Box4, &'static str)>, keep: &'static [words::Box4]) -> Job<'a> {
    Box::new(move || {
        let Some(bytes) = game.read(&path) else { return Made::default() };
        let pictures = rom::textures_in(&bytes)
            .into_iter()
            .filter(|(offset, _)| at.is_none_or(|at| *offset == at))
            .filter_map(|(_, texture)| Some(Picture { name: texture.name(), image: words::redraw(&texture.rgba(), &words, keep, face)? }))
            .collect();
        Made { pictures, files: Vec::new() }
    })
}

fn write_picture(out: &Path, picture: &Picture) -> Result<(), String> {
    use image::ImageEncoder;
    let path = out.join(&picture.name);
    let file = std::fs::File::create(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let encoder = image::codecs::png::PngEncoder::new_with_quality(std::io::BufWriter::new(file), image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::Adaptive);
    encoder
        .write_image(&picture.image.to_bytes(), picture.image.w as u32, picture.image.h as u32, image::ExtendedColorType::Rgba8)
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn write_file(root: &Path, path: &str, bytes: &[u8]) -> Result<(), String> {
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    std::fs::write(&target, bytes).map_err(|error| format!("{}: {error}", target.display()))
}

pub fn generate(rom: &Path, root: &Path, font: &Font, progress: &(dyn Fn(usize, usize) + Sync), cancel: &AtomicBool) -> Result<String, String> {
    let title = zakuro_fs::Title::load(rom).map_err(|error| format!("{}: {error}", rom.display()))?;
    if title.program_id() != PROGRAM_ID {
        return Err("the HD text is made for the European release".into());
    }
    let game = Game { title };
    let face = text::Face::new(font.bytes, 40.0).with_figures(font.lining_figures);
    let mut names: Vec<&'static str> = data::BANNER_SPECS.iter().map(|(n, _, _)| *n).collect();
    names.dedup();
    let messages: HashMap<char, HashMap<String, String>> = LANGUAGES.iter().map(|&lang| (lang, game.read(&format!("m/{lang}/GameData.bin.lz")).map(|d| rom::messages(&d)).unwrap_or_default())).collect();
    let mut jobs: Vec<Job> = Vec::new();
    for lang in LANGUAGES {
        let atlases = game.read(&format!("ui/status_{lang}.ctpk.lz")).map(|bytes| rom::textures_in(&bytes)).unwrap_or_default();
        for (at, texture) in atlases {
            if let Some((_, _, list)) = data::LABELS.iter().find(|(l, offset, _)| *l == lang && *offset == at) {
                jobs.push(label_job(&face, texture, list));
            }
        }
    }
    jobs.push(words_job(&game, &face, "telop/Tlp_Title_02_U.r.lz".into(), Some(0x44D80), vec![(words::COPYRIGHT_BOX, data::COPYRIGHT)], &[]));
    for (lang, text) in data::PRESS_ANY_BUTTON {
        jobs.push(words_job(&game, &face, format!("bs/press_{lang}.tp.lz"), None, vec![(words::PRESS_ANY_BUTTON_BOX, *text)], &words::PRESS_ANY_BUTTON_KEEP));
    }
    for (lang, texts) in data::MAIN_MENU {
        jobs.push(words_job(&game, &face, format!("ui/filemenu_{lang}.ctpk.lz"), Some(0x20180), words::MENU_BOXES.into_iter().zip(texts.iter().copied()).collect(), &words::MENU_KEEP));
    }
    for name in ["System", "UI_L", "UI_M", "UI_S"] {
        jobs.push(font_job(&game, &face, name));
    }
    for lang in LANGUAGES {
        for &name in &names {
            jobs.push(banner_job(&game, &face, lang, name));
        }
        for file in game.list(&format!("chapter/title/{lang}")) {
            jobs.push(title_job(&game, &face, &messages[&lang], lang, file));
        }
    }
    let out = root.join("textures");
    let romfs = root.join("romfs");
    for folder in [&out, &romfs] {
        if folder.exists() {
            std::fs::remove_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        }
    }
    std::fs::create_dir_all(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    let (next, done, written, failure) = (AtomicUsize::new(0), AtomicUsize::new(0), AtomicUsize::new(0), Mutex::new(None));
    progress(0, jobs.len());
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
                progress(done.fetch_add(1, Ordering::Relaxed) + 1, jobs.len());
            });
        }
    });
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    if let Some(error) = failure.into_inner().unwrap() {
        return Err(error);
    }
    std::fs::write(out.join("pack.json"), "{\n  \"options\": {\n    \"use_new_hash\": true\n  }\n}\n").map_err(|error| error.to_string())?;
    Ok(format!("{} pictures of the game's text, drawn again", written.load(Ordering::Relaxed)))
}

fn build(rom: &Path, data_dir: &Path, progress: &(dyn Fn(usize, usize) + Sync), cancel: &AtomicBool) -> Result<String, String> {
    generate(rom, &STEP.folder_in(data_dir), &BUNDLED, progress, cancel)
}

const BUNDLED: Font = Font { bytes: include_bytes!("fonts/AlegreyaSans-Bold.ttf"), lining_figures: 1686 };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn the_whole_game() {
        let (Ok(rom), Ok(out)) = (std::env::var("ZAKURO_HD_ROM"), std::env::var("ZAKURO_HD_OUT")) else {
            panic!("set ZAKURO_HD_ROM and ZAKURO_HD_OUT");
        };
        let started = std::time::Instant::now();
        let said = generate(Path::new(&rom), Path::new(&out), &BUNDLED, &|_, _| {}, &AtomicBool::new(false)).unwrap();
        println!("{said} in {:.1} s", started.elapsed().as_secs_f64());
    }
}
