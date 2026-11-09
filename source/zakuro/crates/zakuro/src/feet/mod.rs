
mod foot;
mod formats;
mod model;

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::recompile::ModStep;

pub const PROGRAM_ID: u64 = 0x0004_0000_0009_F100;

pub const STEP: ModStep = ModStep {
    name: "the feet mod",
    label: "Feet",
    about: "The characters' feet, built from your copy of the game.",
    folder: "feet",
    program_id: PROGRAM_ID,
    build,
};

fn bodies(romfs: &zakuro_fs::romfs::RomFs) -> Result<Vec<String>, String> {
    let mut dir = romfs.root().map_err(|error| error.to_string())?;
    if let Some((_, nameless)) = romfs.subdirs(&dir).into_iter().find(|(_, d)| d.name.is_empty()) {
        dir = nameless;
    }
    let (_, b) = romfs.subdirs(&dir).into_iter().find(|(_, d)| d.name == "b").ok_or("the game has no b folder")?;
    let mut names: Vec<String> =
        romfs.files(&b).into_iter().map(|(_, file)| file.name).filter(|name| name.starts_with("uBody_") && name.ends_with(".m.lz")).collect();
    names.sort();
    Ok(names)
}

fn build(rom: &Path, data_dir: &Path, progress: &(dyn Fn(usize, usize) + Sync), cancel: &AtomicBool) -> Result<String, String> {
    let title = zakuro_fs::Title::load(rom).map_err(|error| format!("{}: {error}", rom.display()))?;
    if title.program_id() != PROGRAM_ID {
        return Err("the feet are made for the European release".into());
    }
    let romfs = title.romfs.as_ref().ok_or("the game has no RomFS")?;
    let bodies = bodies(romfs)?;
    let out = STEP.folder_in(data_dir).join("romfs").join("b");
    std::fs::create_dir_all(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    let read = |name: &str| -> Option<Vec<u8>> {
        let file = romfs.lookup(&format!("b/{name}")).ok()?;
        crate::lz::unpack(title.read_romfs(&file, 0, file.data_size as usize)?).ok()
    };
    let one = |name: &str| -> Result<&'static str, String> {
        let model = read(name).ok_or("its file can't be read")?;
        let texture = read(&name.replace(".m.lz", "_c0.t.lz")).and_then(|file| foot::colour_texture(&file));
        let (bytes, style) = foot::build(&format!("b/{name}"), &model, texture.as_ref())?;
        let path = out.join(name);
        std::fs::write(&path, crate::lz::lz13(&bytes)).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(style)
    };
    let (next, done, results) = (AtomicUsize::new(0), AtomicUsize::new(0), Mutex::new(Vec::new()));
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).min(bodies.len()).max(1);
    progress(0, bodies.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= bodies.len() || cancel.load(Ordering::Relaxed) {
                    break;
                }
                let result = one(&bodies[i]);
                results.lock().unwrap().push((bodies[i].clone(), result));
                progress(done.fetch_add(1, Ordering::Relaxed) + 1, bodies.len());
            });
        }
    });
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    let mut results = results.into_inner().unwrap();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    let made: BTreeSet<&str> = results.iter().filter(|(_, result)| result.is_ok()).map(|(name, _)| name.as_str()).collect();
    for (name, result) in &results {
        match result {
            Ok(style) => log::info!("feet: {name}, a {style}"),
            Err(error) => log::info!("feet: {name} keeps the game's legs, {error}"),
        }
    }
    for entry in std::fs::read_dir(&out).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("uBody_") && name.ends_with(".m.lz") && !made.contains(name.as_str()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    if made.is_empty() {
        return Err("no body could be given feet".into());
    }
    Ok(format!("feet for {} of the {} bodies", made.len(), bodies.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn the_whole_game() {
        let (Ok(rom), Ok(data)) = (std::env::var("ZAKURO_FEET_ROM"), std::env::var("ZAKURO_FEET_DATA")) else {
            panic!("set ZAKURO_FEET_ROM and ZAKURO_FEET_DATA");
        };
        let started = std::time::Instant::now();
        let said = build(Path::new(&rom), Path::new(&data), &|_, _| {}, &AtomicBool::new(false)).unwrap();
        println!("{said} in {:.1} s", started.elapsed().as_secs_f64());
    }
}
