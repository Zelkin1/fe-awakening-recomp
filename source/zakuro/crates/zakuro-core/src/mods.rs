//! mods, kept in the data directory's mods folder under the title's id, laid

use std::path::{Path, PathBuf};

use zakuro_common::memory_map::PAGE_SIZE;
use zakuro_fs::ncch::ExHeader;
use zakuro_fs::{lz77, patch, FsError, Title};

/// the folder a title's mods go in.
pub fn dir(data_dir: &Path, program_id: u64) -> PathBuf {
    let mods = data_dir.join("mods");
    let upper = mods.join(format!("{program_id:016X}"));
    let lower = mods.join(format!("{program_id:016x}"));
    if !upper.exists() && lower.exists() { lower } else { upper }
}

/// the files that change a title's code, each in the mod's folder or in its
/// exefs folder.
const CODE: [&str; 3] = ["code.bin", "code.ips", "code.bps"];

/// the extended header a mod gives in place of the title's, in its folder,
/// which a mod that makes the code longer needs.
const EXHEADER: &str = "exheader.bin";

/// The names of the folders that hold the parts of a mod, which are not mods of their own.
const MOD_PARTS: [&str; 4] = ["romfs", "romfs_ext", "exefs", "textures"];

/// The name of the empty file that marks a mod folder as turned off.
const DISABLED: &str = "disabled";

/// Whether the mod folder is turned on, which it is unless it holds the `disabled` marker.
pub fn enabled(folder: &Path) -> bool {
    !folder.join(DISABLED).exists()
}

/// Turns the mod folder on or off by removing or creating the `disabled` marker.
pub fn set_enabled(folder: &Path, on: bool) -> std::io::Result<()> {
    let marker = folder.join(DISABLED);
    if on {
        // A marker that is already gone is not an error.
        return match std::fs::remove_file(&marker) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(folder)?;
    std::fs::write(marker, b"")
}

/// The folders that hold the mods of the title, in the order they are laid.
/// The folder of the title comes first, followed by the enabled mod folders inside it in name order.
pub fn folders(data_dir: &Path, program_id: u64) -> Vec<PathBuf> {
    let title_folder = dir(data_dir, program_id);
    let mut mod_folders: Vec<PathBuf> = std::fs::read_dir(&title_folder).into_iter().flatten().flatten().map(|entry| entry.path()).filter(|path| is_mod_folder(path)).collect();
    // Sorting makes the order of the layers the same on every run.
    mod_folders.sort();
    std::iter::once(title_folder).chain(mod_folders).collect()
}

/// Whether the path is an enabled folder of a mod, and not one of the part folders of a mod.
fn is_mod_folder(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    path.is_dir() && !MOD_PARTS.iter().any(|part| part.eq_ignore_ascii_case(name)) && enabled(path)
}

/// whether the title's mods folder holds a mod.
pub fn present(data_dir: &Path, program_id: u64) -> bool {
    folders(data_dir, program_id).iter().any(|dir| {
        let files = ["romfs", "romfs_ext"].iter().any(|part| std::fs::read_dir(dir.join(part)).is_ok_and(|mut entries| entries.next().is_some()));
        files || CODE.iter().any(|name| find(dir, name).is_some()) || dir.join(EXHEADER).is_file()
    })
}

/// The `exheader.bin` of the mods of the title when it looks like the one of the title, with its parsed form.
/// The last mod folder that has one wins.
pub fn exheader(title: &Title, data_dir: &Path) -> Option<(Vec<u8>, ExHeader)> {
    let path = folders(data_dir, title.program_id()).into_iter().rev().map(|dir| dir.join(EXHEADER)).find(|path| path.is_file())?;
    let mut bytes = std::fs::read(&path).ok()?;
    let Some(described) = bytes.get(..ExHeader::HASHED_SIZE) else {
        log::warn!("mods: {} is {} bytes, an exheader has {}", path.display(), bytes.len(), ExHeader::HASHED_SIZE);
        return None;
    };
    match ExHeader::parse(described) {
        Ok(mut exheader) if exheader.text.address == title.exheader.text.address => {
            exheader.compress_code = title.exheader.compress_code;
            bytes.truncate(ExHeader::HASHED_SIZE);
            bytes[0x0D] = bytes[0x0D] & !1 | title.exheader.compress_code as u8;
            Some((bytes, exheader))
        }
        Ok(_) => {
            log::warn!("mods: {} is not this game's exheader, or is still encrypted, it is left out", path.display());
            None
        }
        Err(error) => {
            log::warn!("mods: {}: {error}", path.display());
            None
        }
    }
}

fn find(dir: &Path, name: &str) -> Option<PathBuf> {
    [dir.join("exefs").join(name), dir.join(name)].into_iter().find(|path| path.is_file())
}

/// a patch format, applying a patch to data.
type Patch = fn(&[u8], &[u8]) -> Result<Vec<u8>, FsError>;

/// The code of the title with the changes of its mods, and whether there were any.
/// The last `code.bin` wins, and the `code.ips` and `code.bps` patches of every folder are applied over it in order.
pub fn code(title: &Title, data_dir: Option<&Path>) -> Result<(Vec<u8>, bool), FsError> {
    let original = title.code()?;
    let Some(data_dir) = data_dir else {
        return Ok((original, false));
    };
    let mut code = original.clone();
    let folders = folders(data_dir, title.program_id());
    // The reverse order makes the folder laid last the one that is found first.
    if let Some(path) = folders.iter().rev().find_map(|dir| find(dir, "code.bin")) {
        match std::fs::read(&path) {
            Ok(bytes) => {
                code = decompressed(bytes, title);
                log::info!("mods: the game's code is {}", path.display());
            }
            Err(error) => log::warn!("mods: {}: {error}", path.display()),
        }
    }
    let patches: [(&str, Patch); 2] = [("code.ips", patch::ips), ("code.bps", patch::bps)];
    for dir in &folders {
        for (name, apply) in patches {
            let Some(path) = find(dir, name) else { continue };
            match std::fs::read(&path).map_err(FsError::from).and_then(|patch| apply(&patch, &code)) {
                Ok(patched) => {
                    code = patched;
                    log::info!("mods: {} patched the game's code", path.display());
                }
                Err(error) => log::warn!("mods: {}: {error}", path.display()),
            }
        }
    }
    let modded = code != original;
    Ok((code, modded))
}

/// a code.bin as the game runs it. most are kept decompressed, one that is
/// too short for the segments the exheader describes is still compressed.
fn decompressed(bytes: Vec<u8>, title: &Title) -> Vec<u8> {
    let exheader = &title.exheader;
    let needed = (exheader.text.num_pages + exheader.rodata.num_pages) * PAGE_SIZE + exheader.data.size;
    if bytes.len() >= needed as usize {
        return bytes;
    }
    lz77::decompress(&bytes).unwrap_or(bytes)
}

/// Lays the mods of the title over it, saying in the log what they changed.
pub fn lay(title: &mut Title, data_dir: Option<&Path>) {
    let Some(data_dir) = data_dir else {
        return;
    };
    let dir = dir(data_dir, title.program_id());
    if !dir.is_dir() {
        return;
    }
    if let Some((_, exheader)) = exheader(title, data_dir) {
        log::info!("mods: the game's exheader is a mod's");
        title.exheader = exheader;
    }
    let folders = folders(data_dir, title.program_id());
    let from = describe(&folders);
    let folders: Vec<&Path> = folders.iter().map(PathBuf::as_path).collect();
    match title.lay_mods(&folders) {
        Ok(Some(changes)) => log::info!(
            "mods from {from}: {} files replaced, {} added, {} patched, {} removed",
            changes.replaced,
            changes.added,
            changes.patched,
            changes.removed
        ),
        Ok(None) => log::info!("mods: {from} changes nothing in the game's files"),
        Err(error) => log::warn!("mods: {from} can't be laid over the game: {error}"),
    }
    // and over the update's files, which a game reads besides its own
    if let Some(update) = title.update_mut() {
        match update.title.lay_mods(&folders) {
            Ok(Some(changes)) => log::info!(
                "mods over the update's files: {} replaced, {} added, {} patched, {} removed",
                changes.replaced,
                changes.added,
                changes.patched,
                changes.removed
            ),
            Ok(None) => {}
            Err(error) => log::warn!("mods: {from} can't be laid over the update: {error}"),
        }
    }
}

/// Names the folders in one phrase for the log, the folder of the title followed by the names of its mod folders.
fn describe(folders: &[PathBuf]) -> String {
    let [title_folder, mod_folders @ ..] = folders else {
        return String::new();
    };
    let names: Vec<_> = mod_folders.iter().filter_map(|folder| folder.file_name()).map(|name| name.to_string_lossy()).collect();
    match names.is_empty() {
        true => title_folder.display().to_string(),
        false => format!("{} and its {}", title_folder.display(), names.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mod folders inside the folder of a title are found in name order, and a disabled one is skipped.
    #[test]
    fn mods_in_folders_of_their_own_follow_the_titles() {
        let data_dir = std::env::temp_dir().join(format!("zakuro-mod-folders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        let program_id = 0x0004_0000_0009_F100;
        let dir = dir(&data_dir, program_id);
        for folder in ["romfs", "textures", "exefs", "zeta", "feet"] {
            std::fs::create_dir_all(dir.join(folder)).unwrap();
        }
        assert_eq!(folders(&data_dir, program_id), [dir.clone(), dir.join("feet"), dir.join("zeta")]);
        assert!(!present(&data_dir, program_id));
        std::fs::create_dir_all(dir.join("feet/romfs/b")).unwrap();
        std::fs::write(dir.join("feet/romfs/b/body.bin"), b"body").unwrap();
        assert!(present(&data_dir, program_id));
        set_enabled(&dir.join("feet"), false).unwrap();
        assert_eq!(folders(&data_dir, program_id), [dir.clone(), dir.join("zeta")]);
        assert!(!present(&data_dir, program_id));
        set_enabled(&dir.join("feet"), true).unwrap();
        assert!(present(&data_dir, program_id));
        std::fs::remove_dir_all(data_dir).unwrap();
    }
}
