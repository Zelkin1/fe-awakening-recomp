//! Downloads of the tools and fonts that the launcher needs. Every file is checked against its size and its SHA-256 hash before it is used.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use crate::zig::{fetch, windowless, Fetcher};

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

    /// Moves a downloaded file into the folder under its real name. A zip file is unpacked into the folder with tar and then removed.
    fn put(&self, part: &Path, folder: &Path) -> Result<(), String> {
        if !self.name.ends_with(".zip") {
            return std::fs::rename(part, folder.join(self.name)).map_err(|error| format!("{}: {error}", folder.display()));
        }
        let unpacked = windowless(&mut Command::new("tar")).arg("-xf").arg(part).arg("-C").arg(folder).status().map_err(|error| format!("tar could not start, {error}"))?;
        let _ = std::fs::remove_file(part);
        if unpacked.success() { Ok(()) } else { Err(format!("{} could not be unpacked", self.name)) }
    }
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
