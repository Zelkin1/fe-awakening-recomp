//! recompiling a game with 3dsrecomp, on a thread of its own while the rest
//! carries on, with the code and the modules its mods change.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use recomp3ds::build::{self, Event};
use recomp3ds::compile::Compiler;

/// the compiler a recompile uses: one there is, or Zig, downloaded first
/// into this folder of tools.
#[derive(Debug, Clone, PartialEq)]
pub enum Toolchain {
    Ready(Compiler),
    Download(PathBuf),
}

/// A function that builds the files of a mod step.
/// It receives the game file, the data folder, whether the optional part of the mod is wanted, a progress callback and a cancel flag.
/// The progress callback receives what is being done, the parts done and the parts in total.
/// It returns a short report of what was built.
pub type ModBuilder = fn(rom: &Path, data_dir: &Path, optional: bool, progress: &(dyn Fn(&'static str, usize, usize) + Sync), cancel: &AtomicBool) -> Result<String, String>;

/// One installable part of the mods of a game, such as the HD text.
pub struct ModStep {
    /// The short name of the step.
    pub name: &'static str,
    /// The text that tells the user what is being installed.
    pub label: &'static str,
    /// A longer description of what the step does.
    pub about: &'static str,
    /// The folder of the step inside the mods folder of the game.
    pub folder: &'static str,
    /// The title id of the game the step belongs to.
    pub program_id: u64,
    /// The downloads the step needs, which the user is told about before they start.
    pub needs: &'static [crate::downloads::Need],
    /// The downloads the step needs that are fetched without asking.
    pub quietly: &'static [crate::downloads::Download],
    /// The function that builds the files of the step.
    pub build: ModBuilder,
}

impl ModStep {
    /// Returns the folder of the step inside the mods folder of the game in the data folder.
    pub fn folder_in(&self, data_dir: &Path) -> PathBuf {
        zakuro_core::mods::dir(data_dir, self.program_id).join(self.folder)
    }
}

/// Every mod step that the launcher can install.
pub const MOD_STEPS: &[ModStep] = &[crate::feet::STEP, crate::hdtext::STEP];


impl ModStep {
    /// Returns true when a download that the user is told about is missing.
    /// Returns false when the tools folder is unknown.
    pub fn lacks_downloads(&self, data_dir: &Path) -> bool {
        crate::zig::tools_dir(Some(data_dir)).is_some_and(|tools| self.needs.iter().any(|need| !need.download.present(&tools)))
    }

    /// Returns true when the folder of the step holds anything but the entry named "disabled".
    pub fn installed(&self, data_dir: &Path) -> bool {
        std::fs::read_dir(self.folder_in(data_dir)).is_ok_and(|entries| entries.flatten().any(|entry| entry.file_name() != "disabled"))
    }

    /// Returns every download of the step, the ones the user is told about and the quiet ones.
    fn downloads(&self) -> impl Iterator<Item = &'static crate::downloads::Download> {
        let needs: &'static [crate::downloads::Need] = self.needs;
        needs.iter().map(|need| &need.download).chain(self.quietly)
    }

    /// Removes the folder of the step and the downloads of the step.
    /// A download that another installed step also uses stays.
    pub fn uninstall(&self, data_dir: &Path) -> Result<(), String> {
        let folder = self.folder_in(data_dir);
        std::fs::remove_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        let Some(tools) = crate::zig::tools_dir(Some(data_dir)) else { return Ok(()) };
        for download in self.downloads() {
            // Two steps can share a download folder.
            if MOD_STEPS.iter().any(|other| other.name != self.name && other.installed(data_dir) && other.downloads().any(|theirs| theirs.folder == download.folder)) {
                continue;
            }
            let path = download.path(&tools);
            // A download that is not there already is not an error.
            match std::fs::remove_dir_all(&path) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(format!("{}: {error}", path.display())),
                _ => {}
            }
        }
        Ok(())
    }
}

/// Removes what the launcher built for a game: its recompiled code, the work files of the recompiler and the mods that the launcher built for it.
/// The saves, the save states, the settings and the mods that the user added stay.
/// The function tries every part, and the error lists the parts that it did not remove.
pub fn remove_builds(program_id: u64, data_dir: Option<&Path>) -> Result<(), String> {
    remove_builds_in(program_id, recomp3ds::abi::library_dir().as_deref(), &build::cache_dir(program_id), data_dir)
}

/// Removes the builds of a game as remove_builds does, with the folder of the recompiled libraries and the work folder of the game given.
fn remove_builds_in(program_id: u64, library_dir: Option<&Path>, cache_dir: &Path, data_dir: Option<&Path>) -> Result<(), String> {
    let mut paths = vec![cache_dir.to_owned()];
    // The recompiled code is a library named after the title, with the files beside it that start with the same name.
    let name = format!("{program_id:016X}.");
    if let Some(entries) = library_dir.and_then(|dir| std::fs::read_dir(dir).ok()) {
        paths.extend(entries.flatten().filter(|entry| entry.file_name().to_string_lossy().starts_with(&name)).map(|entry| entry.path()));
    }
    let mut failed: Vec<String> = paths.iter().filter_map(|path| remove(path).err()).collect();
    if let Some(data_dir) = data_dir {
        let built = MOD_STEPS.iter().filter(|step| step.program_id == program_id && step.installed(data_dir));
        failed.extend(built.filter_map(|step| step.uninstall(data_dir).err()));
    }
    match failed.is_empty() {
        true => Ok(()),
        false => Err(failed.join(", ")),
    }
}

/// Removes a file or a folder, and is fine when it is not there.
fn remove(path: &Path) -> Result<(), String> {
    let result = if path.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) };
    match result {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {error}", path.display())),
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    /// Building the mods, with what is being done now and how many parts are done of the total.
    Mods { doing: &'static str, done: usize, total: usize },
    /// fetching the compiler, how much of it has arrived, 0 to 1.
    Downloading(f32),
    /// finding the code and writing it as C.
    Generating,
    Compiling { done: usize, total: usize },
    /// linking the library and putting it where Zakuro finds it.
    Installing,
    Done,
    Failed(String),
}

impl Stage {
    pub fn finished(&self) -> bool {
        matches!(self, Stage::Done | Stage::Failed(_))
    }

    /// What the mods build is doing now, or an empty text in any other stage.
    pub fn doing(&self) -> &'static str {
        match self {
            Stage::Mods { doing, .. } => doing,
            _ => "",
        }
    }

    /// how far along it is, 0 to 1, going by the part that takes longest.
    pub fn fraction(&self) -> f32 {
        match self {
            Stage::Mods { done, total, .. } => *done as f32 / (*total).max(1) as f32,
            Stage::Downloading(arrived) => *arrived,
            Stage::Generating => 0.05,
            Stage::Compiling { done, total } => 0.05 + 0.9 * *done as f32 / (*total).max(1) as f32,
            Stage::Installing => 0.95,
            Stage::Done | Stage::Failed(_) => 1.0,
        }
    }

    /// where an event of the build leaves it, none for one that changes
    /// nothing.
    fn after(event: &Event) -> Option<Stage> {
        match event {
            Event::Generated { .. } => Some(Stage::Compiling { done: 0, total: 1 }),
            Event::Compiled { done, total } => Some(Stage::Compiling { done: *done, total: *total }),
            Event::Built { .. } | Event::Installed(_) => Some(Stage::Installing),
            Event::Note(_) => None,
        }
    }
}

struct State {
    stage: Stage,
    finished_at: Option<Instant>,
}

pub struct Job {
    pub program_id: u64,
    pub name: String,
    /// the user was told how it ended.
    pub announced: bool,
    /// The label of the mod step this job installs, or none when it recompiles a game.
    pub installing: Option<&'static str>,
    started: Instant,
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
}

impl Job {
    /// Starts recompiling the game at the path of the rom on a thread.
    /// The toolchain says whether the compiler is ready or must be downloaded first.
    pub fn start(rom: &Path, update: Option<&Path>, program_id: u64, name: &str, data_dir: Option<&Path>, toolchain: Toolchain) -> Job {
        let first = match &toolchain {
            Toolchain::Download(_) => Stage::Downloading(0.0),
            Toolchain::Ready(_) => Stage::Generating,
        };
        let state = Arc::new(Mutex::new(State { stage: first, finished_at: None }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (progress, stop, rom, update) = (state.clone(), cancel.clone(), rom.to_owned(), update.map(Path::to_owned));
        let (game, data_dir) = (name.to_owned(), data_dir.map(Path::to_owned));
        std::thread::spawn(move || {
            let set = |stage: Stage| {
                if let Ok(mut state) = progress.lock() {
                    state.stage = stage;
                }
            };
            if !matches!(toolchain, Toolchain::Download(_)) {
                set(Stage::Generating);
            }
            let events = |event: Event| {
                if let (Some(stage), Ok(mut state)) = (Stage::after(&event), progress.lock()) {
                    state.stage = stage;
                }
            };
            let compiler = match toolchain {
                Toolchain::Ready(compiler) => Ok(compiler),
                Toolchain::Download(tools) => {
                    let arrived = |fraction: f32| {
                        if let Ok(mut state) = progress.lock() {
                            state.stage = Stage::Downloading(fraction);
                        }
                    };
                    let downloaded = crate::zig::download(&tools, &arrived, &stop);
                    if let Ok(mut state) = progress.lock() {
                        state.stage = Stage::Generating;
                    }
                    downloaded
                }
            };
            let result =
                compiler.and_then(|compiler| recompile(&rom, update.as_deref(), program_id, &game, data_dir.as_deref(), compiler, &stop, &events));
            if let Ok(mut state) = progress.lock() {
                state.stage = match result {
                    Ok(_) => Stage::Done,
                    Err(_) if stop.load(Ordering::Relaxed) => Stage::Failed("cancelled".to_owned()),
                    Err(error) => Stage::Failed(error),
                };
                state.finished_at = Some(Instant::now());
            }
        });
        Job { program_id, name: name.to_owned(), announced: false, installing: None, started: Instant::now(), state, cancel }
    }

    /// Starts building a mod step for the game at the path of the rom on a thread, with or without the optional part of the mod.
    pub fn install(rom: &Path, name: &str, data_dir: &Path, step: &'static ModStep, optional: bool) -> Job {
        let state = Arc::new(Mutex::new(State { stage: Stage::Mods { doing: "", done: 0, total: 0 }, finished_at: None }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (progress, stop, rom, data_dir) = (state.clone(), cancel.clone(), rom.to_owned(), data_dir.to_owned());
        // Whether the step was absent before, which decides if a cancelled build leaves its folder behind.
        let fresh = !step.installed(&data_dir);
        std::thread::spawn(move || {
            let set = |stage: Stage| {
                if let Ok(mut state) = progress.lock() {
                    state.stage = stage;
                }
            };
            let told = |doing: &'static str, done: usize, total: usize| set(Stage::Mods { doing, done, total });
            let outcome = match (step.build)(&rom, &data_dir, optional, &told, &stop) {
                Ok(made) => {
                    log::info!("built {}: {made}", step.name);
                    Stage::Done
                }
                Err(_) if stop.load(Ordering::Relaxed) => {
                    // A half-built new step is removed, but an installed step is kept as it was.
                    if fresh {
                        let _ = std::fs::remove_dir_all(step.folder_in(&data_dir));
                    }
                    Stage::Failed("cancelled".to_owned())
                }
                Err(error) => Stage::Failed(error),
            };
            if let Ok(mut state) = progress.lock() {
                state.stage = outcome;
                state.finished_at = Some(Instant::now());
            }
        });
        Job { program_id: step.program_id, name: name.to_owned(), announced: false, installing: Some(step.label), started: Instant::now(), state, cancel }
    }

    pub fn stage(&self) -> Stage {
        self.state.lock().map(|state| state.stage.clone()).unwrap_or(Stage::Failed("lost track".to_owned()))
    }


    /// how long it has run, or ran.
    pub fn elapsed(&self) -> Duration {
        let finished = self.state.lock().ok().and_then(|state| state.finished_at);
        finished.unwrap_or_else(Instant::now) - self.started
    }

    /// stops it after the files being compiled, the C being written first
    /// if it is at that.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// recompiles the game at rom, its code and its modules the way its update
/// and its mods in data_dir leave them when they change any.
#[allow(clippy::too_many_arguments)]
fn recompile(
    rom: &Path,
    update: Option<&Path>,
    program_id: u64,
    name: &str,
    data_dir: Option<&Path>,
    compiler: Compiler,
    cancel: &AtomicBool,
    events: &(dyn Fn(Event) + Sync),
) -> Result<PathBuf, String> {
    let changes = Changes::read(rom, update, program_id, data_dir);
    let files = |path: &str| changes.as_ref()?.module(path);
    log::info!("recompiling {name} with {}", compiler.describe());
    let mut options = build::Options { cancel: Some(cancel), compiler: Some(compiler), background: true, ..build::Options::default() };
    if let Some(changes) = &changes {
        log::info!(
            "recompiling {name} with its update or mods, which change {}. the library replaces one made without them, and should they go, the functions they changed run in the interpreter",
            changes.describe()
        );
        options.mods = recomp3ds::Mods { code: changes.code.as_deref(), exheader: changes.exheader.as_deref(), romfs: Some(&files) };
    }
    build::build(rom, &options, events)
}

/// what a game's mods change in what gets recompiled.
struct Changes {
    /// the code, decompressed, when they change it.
    code: Option<Vec<u8>>,
    /// the game's modules and static.crs they replace or patch, by their
    /// paths in the RomFS, with their bytes.
    modules: Vec<(String, Vec<u8>)>,
    /// the exheader they give, which says where the code's segments lie.
    exheader: Option<Vec<u8>>,
}

impl Changes {
    /// what the update at update and the mods in data_dir change for the
    /// game at rom, none when they leave its code and its modules alone.
    fn read(rom: &Path, update: Option<&Path>, program_id: u64, data_dir: Option<&Path>) -> Option<Changes> {
        let modded = data_dir.is_some_and(|data_dir| zakuro_core::mods::present(data_dir, program_id));
        if !modded && update.is_none() {
            return None;
        }
        let mut title = zakuro_fs::Title::load(rom).inspect_err(|error| log::warn!("mods: {}: {error}", rom.display())).ok()?;
        let updated = update.is_some_and(|update| zakuro_core::loader::attach_update(&mut title, update));
        // the mods' exheader, or the update's, which says where its code's
        // segments are
        let exheader = data_dir
            .and_then(|data_dir| zakuro_core::mods::exheader(&title, data_dir))
            .map(|(bytes, _)| bytes)
            .or_else(|| updated.then(|| title.exheader_bytes().to_vec()));
        zakuro_core::mods::lay(&mut title, data_dir);
        let code = match zakuro_core::mods::code(&title, data_dir) {
            Ok((code, modded)) => (modded || updated).then_some(code),
            Err(error) => {
                log::warn!("mods: the game's code can't be read: {error}");
                None
            }
        };
        // 3dsrecomp reads the modules the game has, the update's version of
        // one, and the mods' over either. one either adds is left to the
        // interpreter
        let mut modules = match (title.update(), &title.romfs) {
            (Some(update), Some(game)) => update_modules(&update.title, game),
            _ => Vec::new(),
        };
        if let (Some(layered), Some(game)) = (&title.layered, &title.romfs) {
            for (path, bytes) in layered.modded_files(|path| is_module(path) && game.lookup(path).is_ok()) {
                modules.retain(|(at, _)| !at.eq_ignore_ascii_case(&path));
                modules.push((path, bytes));
            }
        }
        (code.is_some() || !modules.is_empty()).then_some(Changes { code, modules, exheader })
    }

    /// the bytes the mods put in place of the module at path.
    fn module(&self, path: &str) -> Option<Vec<u8>> {
        self.modules.iter().find(|(at, _)| at.eq_ignore_ascii_case(path)).map(|(_, bytes)| bytes.clone())
    }

    /// what they change, the code and the modules by path.
    fn describe(&self) -> String {
        let code = self.code.as_ref().map(|_| "the code");
        code.into_iter().chain(self.modules.iter().map(|(path, _)| path.as_str())).collect::<Vec<_>>().join(", ")
    }
}

/// the modules of the game an update has a version of, by their paths, with
/// that version's bytes, with the mods over the update when there are any.
fn update_modules(update: &zakuro_fs::Title, game: &zakuro_fs::RomFs) -> Vec<(String, Vec<u8>)> {
    let Some(romfs) = &update.romfs else { return Vec::new() };
    let modded: Vec<(String, Vec<u8>)> = update.layered.as_ref().map(|layered| layered.modded_files(is_module)).unwrap_or_default();
    let mut modules = Vec::new();
    let Ok(root) = romfs.root() else { return modules };
    let mut pending = vec![(String::new(), root)];
    while let Some((path, dir)) = pending.pop() {
        for (_, file) in romfs.files(&dir) {
            let path = if path.is_empty() { file.name.clone() } else { format!("{path}/{}", file.name) };
            if !is_module(&path) || game.lookup(&path).is_err() {
                continue;
            }
            let bytes = match modded.iter().find(|(at, _)| at.eq_ignore_ascii_case(&path)) {
                Some((_, bytes)) => Some(bytes.clone()),
                None => update.read_romfs(&file, 0, file.data_size as usize).map(<[u8]>::to_vec),
            };
            modules.extend(bytes.map(|bytes| (path, bytes)));
        }
        for (_, child) in romfs.subdirs(&dir) {
            pending.push((if path.is_empty() { child.name.clone() } else { format!("{path}/{}", child.name) }, child));
        }
    }
    modules
}

/// whether the RomFS file at path is code 3dsrecomp reads, a CRO module or
/// the static module that describes the executable.
fn is_module(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.ends_with(".cro") || path == "static.crs"
}

#[cfg(test)]
mod tests {
    use super::*;
    use zakuro_fs::romfs_build::{self, BuildFile};

    #[test]
    fn the_build_moves_the_stage_along() {
        let generated = Event::Generated { files: 279, bytes: 1, overrides: 0 };
        assert_eq!(Stage::after(&generated), Some(Stage::Compiling { done: 0, total: 1 }));
        assert_eq!(Stage::after(&Event::Compiled { done: 140, total: 279 }), Some(Stage::Compiling { done: 140, total: 279 }));
        assert_eq!(Stage::after(&Event::Installed("/x".into())), Some(Stage::Installing));
        assert_eq!(Stage::after(&Event::Note("hm".to_owned())), None);
        assert!(Stage::Compiling { done: 140, total: 279 }.fraction() > 0.5);
    }

    /// the id of the game the tests make, which no real game has.
    const PROGRAM_ID: u64 = 0x0004_0000_0FF3_DE00;

    fn put(out: &mut [u8], at: usize, bytes: &[u8]) {
        out[at..at + bytes.len()].copy_from_slice(bytes);
    }

    /// writes a decrypted .cxi to a file named after test, its code all
    /// text and its RomFS holding files.
    fn rom(test: &str, code: &[u8], files: &[(&str, &[u8])]) -> PathBuf {
        // the NCCH header, never encrypted, the exheader after it, and the
        // ExeFS at 0x600 with .code its only file
        let mut out = vec![0; 0x800];
        put(&mut out, 0x100, b"NCCH");
        put(&mut out, 0x118, &PROGRAM_ID.to_le_bytes());
        out[0x18F] = 0x04;
        put(&mut out, 0x1A0, &3u32.to_le_bytes());
        put(&mut out, 0x210, &[0x0010_0000, 1, code.len() as u32].map(u32::to_le_bytes).concat());
        put(&mut out, 0x600, b".code");
        put(&mut out, 0x60C, &(code.len() as u32).to_le_bytes());
        out.extend_from_slice(code);
        // the RomFS, an IVFC header without hashes and level 3 right after
        let files: Vec<_> = files.iter().map(|(path, data)| BuildFile { path: path.to_string(), data: data.to_vec() }).collect();
        let at = out.len().next_multiple_of(0x200);
        out.resize(at + 0x60, 0);
        put(&mut out, at, b"IVFC");
        put(&mut out, at + 0x04, &0x0001_0000u32.to_le_bytes());
        out[at + 0x4C] = 4;
        out.extend(romfs_build::build(&files));
        let size = (out.len() - at).div_ceil(0x200) as u32;
        put(&mut out, 0x1B0, &((at / 0x200) as u32).to_le_bytes());
        put(&mut out, 0x1B4, &size.to_le_bytes());
        let path = std::env::temp_dir().join(format!("zakuro-recompile-{}-{test}.cxi", std::process::id()));
        std::fs::write(&path, out).unwrap();
        path
    }

    fn write(path: &Path, data: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, data).unwrap();
    }

    /// Removing the builds of a game takes its library, the files beside it and its work folder.
    /// The library of another game and the mods of another game stay, and removing again is fine.
    #[test]
    fn the_builds_of_one_game_go_and_the_others_stay() {
        let root = std::env::temp_dir().join(format!("zakuro-remove-builds-{}", std::process::id()));
        let (libraries, cache, data) = (root.join("libraries"), root.join("cache"), root.join("data"));
        let other = 0x0004_0000_000A_0500_u64;
        for name in ["00040000000A0500.dll", "00040000000A0500.hints", "00040000000A0500.dll.old0", "000400000009F100.dll"] {
            write(&libraries.join(name), b"x");
        }
        write(&cache.join("code.c"), b"x");
        let kept_mod = MOD_STEPS[0].folder_in(&data).join("kept.bin");
        write(&kept_mod, b"x");
        assert_eq!(remove_builds_in(other, Some(&libraries), &cache, Some(&data)), Ok(()));
        let left: Vec<String> = std::fs::read_dir(&libraries).unwrap().flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left, ["000400000009F100.dll"]);
        assert!(!cache.exists());
        assert!(kept_mod.exists(), "the mods of another game stay");
        assert_eq!(remove_builds_in(other, Some(&libraries), &cache, Some(&data)), Ok(()), "nothing left to remove");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// what a mod does to the code and to the game's modules is what gets
    /// recompiled, and a mod of other files changes nothing recompiled.
    #[test]
    fn the_code_and_the_modules_a_mod_changes_are_found() {
        let files: [(&str, &[u8]); 4] = [("static.crs", b"crs"), ("cro/Battle.cro", b"battle"), ("cro/Field.cro", b"field"), ("a.bin", b"a")];
        let rom = rom("found", &[0; 8], &files);
        let data_dir = std::env::temp_dir().join(format!("zakuro-recompile-{}-found", std::process::id()));
        let mods = zakuro_core::mods::dir(&data_dir, PROGRAM_ID);
        assert!(Changes::read(&rom, None, PROGRAM_ID, Some(&data_dir)).is_none());
        write(&mods.join("romfs/a.bin"), b"b");
        write(&mods.join("romfs/cro/Extra.cro"), b"extra");
        assert!(Changes::read(&rom, None, PROGRAM_ID, Some(&data_dir)).is_none(), "neither the code nor the game's modules changed");

        let ips = |data: &[u8]| [&b"PATCH"[..], &[0, 0, 0, 0, data.len() as u8], data, b"EOF"].concat();
        write(&mods.join("exefs/code.ips"), &ips(&[0xAA]));
        write(&mods.join("romfs/cro/battle.cro"), b"modded battle");
        write(&mods.join("romfs_ext/static.crs.ips"), &ips(b"C"));
        let changes = Changes::read(&rom, None, PROGRAM_ID, Some(&data_dir)).unwrap();
        assert_eq!(changes.code.as_deref(), Some(&[0xAA, 0, 0, 0, 0, 0, 0, 0][..]));
        assert_eq!(changes.module("cro/Battle.cro").as_deref(), Some(&b"modded battle"[..]));
        assert_eq!(changes.module("static.crs").as_deref(), Some(&b"Crs"[..]));
        assert_eq!(changes.module("cro/Field.cro"), None);
        assert_eq!(changes.module("cro/Extra.cro"), None);
        let mut described: Vec<String> = changes.describe().split(", ").map(str::to_owned).collect();
        described.sort();
        assert_eq!(described, ["cro/Battle.cro", "static.crs", "the code"]);

        std::fs::remove_dir_all(data_dir).unwrap();
        std::fs::remove_file(rom).unwrap();
    }

    /// a mod's exheader.bin takes the place of the game's when it is the
    /// game's own with its code made longer, and goes to 3dsrecomp, and one
    /// that looks like another game's, or still encrypted, is left out.
    #[test]
    fn a_mods_exheader_is_taken_when_it_is_the_games() {
        let rom = rom("exheader", &[0; 8], &[]);
        let data_dir = std::env::temp_dir().join(format!("zakuro-recompile-{}-exheader", std::process::id()));
        let mods = zakuro_core::mods::dir(&data_dir, PROGRAM_ID);
        let mut exheader = vec![0; 0x800];
        put(&mut exheader, 0x10, &[0x0010_0000, 1, 12].map(u32::to_le_bytes).concat());
        write(&mods.join("exheader.bin"), &exheader);
        write(&mods.join("code.bin"), &[0xAA; 12]);
        let laid = || {
            let mut title = zakuro_fs::Title::load(&rom).unwrap();
            zakuro_core::mods::lay(&mut title, Some(&data_dir));
            title.exheader.text.size
        };
        assert_eq!(laid(), 12);
        let changes = Changes::read(&rom, None, PROGRAM_ID, Some(&data_dir)).unwrap();
        assert_eq!(changes.exheader.as_deref().map(<[u8]>::len), Some(0x400));
        assert_eq!(changes.code.as_deref(), Some(&[0xAA; 12][..]));

        put(&mut exheader, 0x10, &0x0020_0000u32.to_le_bytes());
        write(&mods.join("exheader.bin"), &exheader);
        assert_eq!(laid(), 8, "another game's exheader is left out");
        assert!(Changes::read(&rom, None, PROGRAM_ID, Some(&data_dir)).unwrap().exheader.is_none());

        std::fs::remove_dir_all(data_dir).unwrap();
        std::fs::remove_file(rom).unwrap();
    }

    #[test]
    #[ignore]
    fn a_whole_build() {
        let (Ok(rom), Ok(data)) = (std::env::var("ZAKURO_BUILD_ROM"), std::env::var("ZAKURO_BUILD_DATA")) else {
            panic!("set ZAKURO_BUILD_ROM and ZAKURO_BUILD_DATA");
        };
        let rom = Path::new(&rom);
        let program_id = zakuro_fs::Title::load(rom).unwrap().program_id();
        let compiler = recomp3ds::compile::check(None).unwrap();
        let job = Job::start(rom, None, program_id, "the game", Some(Path::new(&data)), Toolchain::Ready(compiler));
        let mut said = String::new();
        loop {
            let stage = job.stage();
            let now = match &stage {
                Stage::Mods { total, .. } => format!("building the mods, {total} parts"),
                Stage::Compiling { total, .. } => format!("compiling {total}"),
                other => format!("{other:?}"),
            };
            if now != said {
                println!("{:>5} s  {now}", job.elapsed().as_secs());
                said = now;
            }
            if stage.finished() {
                assert_eq!(stage, Stage::Done);
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    #[test]
    fn modules_are_cro_files_and_static_crs() {
        assert!(is_module("cro/Battle.cro") && is_module("Battle.CRO") && is_module("static.crs"));
        assert!(!is_module("cro/static.crs") && !is_module("cro/Battle.crr") && !is_module("a.bin"));
    }
}
