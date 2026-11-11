//! Downloads of the tools and fonts that the launcher needs. Every file is checked against its size and its SHA-256 hash before it is used.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::zig::{fetch, Fetcher};

/// The name of the marker file that is written into a download folder once every source has arrived.
const COMPLETE: &str = "complete";

/// One file to download.
pub struct Source {
    /// The address the file is downloaded from.
    pub url: &'static str,
    /// The file name that is kept in the folder. A name that ends in ".zip" is unpacked instead.
    pub name: &'static str,
    /// The expected size of the file in bytes.
    pub size: u64,
    /// The expected SHA-256 hash of the file as lowercase hexadecimal text.
    pub sha256: &'static str,
}

/// A set of files that are downloaded into one folder.
pub struct Download {
    /// The name of the folder inside the tools folder.
    pub folder: &'static str,
    /// The files to download, in order.
    pub sources: &'static [Source],
    /// An optional step that runs on the folder after every source has arrived.
    pub finish: Option<fn(&Path) -> Result<(), String>>,
}

/// A download together with the texts that explain it to the user.
pub struct Need {
    /// The short name of the optional part of the mod that needs the download, such as "AI cleanup".
    pub name: &'static str,
    /// What is downloaded.
    pub what: &'static str,
    /// Why the download is needed.
    pub why: &'static str,
    /// The download itself.
    pub download: Download,
}

impl Download {
    /// Returns the folder of this download inside the tools folder.
    pub fn path(&self, tools: &Path) -> PathBuf {
        tools.join(self.folder)
    }

    /// Returns true when an earlier download finished completely.
    pub fn present(&self, tools: &Path) -> bool {
        self.path(tools).join(COMPLETE).is_file()
    }

    /// Returns the total size of all sources in bytes.
    fn size(&self) -> u64 {
        self.sources.iter().map(|source| source.size).sum()
    }

    /// Returns the total size as text, in megabytes from one million bytes and in kilobytes below that. The number is rounded up.
    pub fn size_text(&self) -> String {
        let size = self.size();
        if size >= 1_000_000 { format!("{} MB", size.div_ceil(1_000_000)) } else { format!("{} KB", size.div_ceil(1_000)) }
    }

    /// Downloads everything unless it is present already, and returns the folder. The progress callback receives the fraction of the whole download from 0 to 1. A failed download leaves no folder behind.
    pub fn ensure(&self, tools: &Path, progress: &dyn Fn(f32), cancel: &AtomicBool) -> Result<PathBuf, String> {
        let folder = self.path(tools);
        if self.present(tools) {
            return Ok(folder);
        }
        let fetcher = Fetcher::find().ok_or("Zakuro downloads with curl or wget, and this computer has neither")?;
        // A folder without the marker file is the rest of a download that did not finish.
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        let filled = self.fill(&folder, fetcher, progress, cancel).and_then(|()| std::fs::write(folder.join(COMPLETE), b"").map_err(|error| format!("{}: {error}", folder.display())));
        if filled.is_err() {
            let _ = std::fs::remove_dir_all(&folder);
        }
        filled.map(|()| folder)
    }

    /// Downloads, checks and stores every source in order, then runs the finish step.
    fn fill(&self, folder: &Path, fetcher: Fetcher, progress: &dyn Fn(f32), cancel: &AtomicBool) -> Result<(), String> {
        // The maximum with 1 keeps the division below from dividing by zero.
        let total = self.size().max(1) as f32;
        let mut before = 0;
        for source in self.sources {
            // The file arrives under a temporary name and gets its real name only after it passes the check.
            let part = folder.join(format!("{}.part", source.name));
            // The bytes of the sources before this one, which makes the progress count over all sources together.
            let done = before as f32;
            fetch(fetcher, source.url, &part, source.size, &|arrived| progress((done + arrived * source.size as f32) / total), cancel)?;
            source.check(&part)?;
            source.put(&part, folder)?;
            before += source.size;
        }
        self.finish.map_or(Ok(()), |finish| finish(folder))
    }
}

impl Source {
    /// Checks that the file at the path has the expected size and the expected SHA-256 hash.
    fn check(&self, path: &Path) -> Result<(), String> {
        use sha2::{Digest, Sha256};
        let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
        let size = file.metadata().map_err(|error| error.to_string())?.len();
        if size != self.size {
            return Err(format!("{size} bytes of {} arrived, not {}", self.name, self.size));
        }
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher).map_err(|error| error.to_string())?;
        let hash: String = hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect();
        if hash != self.sha256 {
            return Err(format!("{} is not the file Zakuro knows", self.name));
        }
        Ok(())
    }

    /// Moves a downloaded file into the folder under its real name. A zip file is unpacked into the folder and then removed.
    fn put(&self, part: &Path, folder: &Path) -> Result<(), String> {
        if !self.name.ends_with(".zip") {
            return std::fs::rename(part, folder.join(self.name)).map_err(|error| format!("{}: {error}", folder.display()));
        }
        let unpacked = std::fs::read(part).map_err(|error| error.to_string()).and_then(|bytes| unzip(&bytes, folder));
        let _ = std::fs::remove_file(part);
        unpacked.map_err(|error| format!("{} could not be unpacked, {error}", self.name))
    }
}

/// Reads a little-endian number of up to four bytes at an offset of the bytes.
fn number(bytes: &[u8], at: usize, size: usize) -> Result<usize, String> {
    let field = bytes.get(at..at + size).ok_or("the zip file ends too soon")?;
    Ok(field.iter().rev().fold(0, |value, &byte| value << 8 | byte as usize))
}

/// Unpacks the bytes of a zip file into a folder.
/// Zakuro unpacks zip files itself, since not every system has a tool for them.
/// Only stored and deflated entries are read, which are the ones zip files use.
fn unzip(bytes: &[u8], folder: &Path) -> Result<(), String> {
    // The record at the end of the file tells where the central directory starts and how many entries it lists.
    let end = (0..bytes.len().saturating_sub(21)).rev().find(|&i| bytes[i..i + 4] == [0x50, 0x4B, 0x05, 0x06]).ok_or("it is not a zip file")?;
    let mut at = number(bytes, end + 16, 4)?;
    for _ in 0..number(bytes, end + 10, 2)? {
        if number(bytes, at, 4)? != 0x0201_4B50 {
            return Err("its directory is damaged".to_owned());
        }
        let (method, packed, size) = (number(bytes, at + 10, 2)?, number(bytes, at + 20, 4)?, number(bytes, at + 24, 4)?);
        let name_length = number(bytes, at + 28, 2)?;
        let name = String::from_utf8_lossy(bytes.get(at + 46..at + 46 + name_length).ok_or("the zip file ends too soon")?).into_owned();
        let local = number(bytes, at + 42, 4)?;
        at += 46 + name_length + number(bytes, at + 30, 2)? + number(bytes, at + 32, 2)?;
        // A name that climbs out of the folder is refused, so an entry can only land inside it.
        let relative = Path::new(&name);
        if !relative.components().all(|part| matches!(part, Component::Normal(_))) {
            return Err(format!("it holds an unsafe name, {name}"));
        }
        let target = folder.join(relative);
        if name.ends_with('/') {
            std::fs::create_dir_all(&target).map_err(|error| format!("{}: {error}", target.display()))?;
            continue;
        }
        // The data follows the local header, whose name and extra field can differ in length from the directory.
        let start = local + 30 + number(bytes, local + 26, 2)? + number(bytes, local + 28, 2)?;
        let data = bytes.get(start..start + packed).ok_or("the zip file ends too soon")?;
        let content = match method {
            0 => data.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec(data).map_err(|_| format!("{name} is damaged"))?,
            _ => return Err(format!("{name} is packed in a way Zakuro does not read")),
        };
        if content.len() != size {
            return Err(format!("{name} has the wrong size"));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        std::fs::write(&target, content).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(size: u64, sha256: &'static str) -> Source {
        Source { url: "", name: "x.ttf", size, sha256 }
    }

    #[test]
    fn sizes_add_up_in_the_unit_that_fits() {
        const SMALL: [Source; 2] = [Source { url: "", name: "a", size: 266_856, sha256: "" }, Source { url: "", name: "b", size: 267_996, sha256: "" }];
        const LARGE: [Source; 2] = [Source { url: "", name: "c", size: 45_474_481, sha256: "" }, Source { url: "", name: "d", size: 66_665_765, sha256: "" }];
        let small = Download { folder: "a", sources: &SMALL, finish: None };
        let large = Download { folder: "b", sources: &LARGE, finish: None };
        assert_eq!((small.size_text(), large.size_text()), ("535 KB".to_owned(), "113 MB".to_owned()));
    }

    /// Builds a zip file whose entries are stored or deflated, without checksums, which unzip does not read.
    fn zip_of(entries: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let (mut out, mut central) = (Vec::new(), Vec::new());
        for &(name, data, deflated) in entries {
            let packed = if deflated { miniz_oxide::deflate::compress_to_vec(data, 6) } else { data.to_vec() };
            let (method, local) = (if deflated { 8u8 } else { 0 }, out.len() as u32);
            let sizes = [(packed.len() as u32).to_le_bytes(), (data.len() as u32).to_le_bytes()].concat();
            out.extend(0x0403_4B50u32.to_le_bytes());
            out.extend([20, 0, 0, 0, method, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend(&sizes);
            out.extend((name.len() as u16).to_le_bytes());
            out.extend([0, 0]);
            out.extend(name.as_bytes());
            out.extend(&packed);
            central.extend(0x0201_4B50u32.to_le_bytes());
            central.extend([20, 0, 20, 0, 0, 0, method, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            central.extend(&sizes);
            central.extend((name.len() as u16).to_le_bytes());
            central.extend([0; 12]);
            central.extend(local.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let (offset, count) = (out.len() as u32, entries.len() as u16);
        out.extend(&central);
        out.extend(0x0605_4B50u32.to_le_bytes());
        out.extend([0, 0, 0, 0]);
        out.extend([count.to_le_bytes(), count.to_le_bytes()].concat());
        out.extend((central.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        out.extend([0, 0]);
        out
    }

    /// A zip file unpacks its folders, stored files and deflated files, and a name that climbs out of the folder is refused.
    #[test]
    fn zip_files_unpack_safely() {
        let folder = std::env::temp_dir().join(format!("zakuro-unzip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let text = b"the same words again and again, the same words again and again";
        unzip(&zip_of(&[("models/", b"", false), ("models/a.param", b"7767517", false), ("b.txt", text, true)]), &folder).unwrap();
        assert_eq!(std::fs::read(folder.join("models").join("a.param")).unwrap(), b"7767517");
        assert_eq!(std::fs::read(folder.join("b.txt")).unwrap(), text);
        assert!(unzip(&zip_of(&[("../c.txt", b"x", false)]), &folder).is_err() && !folder.parent().unwrap().join("c.txt").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_wrong_file_is_refused() {
        let folder = std::env::temp_dir().join(format!("zakuro-download-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("x.ttf.part");
        std::fs::write(&path, b"abc").unwrap();
        let right = source(3, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert!(source(4, "").check(&path).is_err() && source(3, "00").check(&path).is_err() && right.check(&path).is_ok());
        right.put(&path, &folder).unwrap();
        assert!(folder.join("x.ttf").is_file() && !path.exists());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
