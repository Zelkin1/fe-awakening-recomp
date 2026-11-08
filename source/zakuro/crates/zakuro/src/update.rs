use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use crate::zig::{fetch, windowless, Fetcher};

pub const RELEASE: &str = "v1.0.2";
const LATEST: &str = "https://api.github.com/repos/Zelkin1/fe-awakening-recomp/releases/latest";
const ASSET_SUFFIX: &str = "-windows-x64.zip";
const STAGING: &str = "update";
const LICENSES: &str = "THIRD_PARTY_LICENSES.txt";
const LAUNCHER: &str = "zakuro.exe";

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    tag: String,
    name: String,
    url: String,
    size: u64,
}

pub struct Installing {
    pub progress: Arc<Mutex<f32>>,
    pub done: Receiver<Result<PathBuf, String>>,
}

pub fn installable() -> bool {
    launcher_folder().is_some_and(|folder| folder.join(LICENSES).is_file())
}

pub fn tidy() {
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = std::fs::remove_file(exe.with_extension("exe.old"));
    if let Some(folder) = exe.parent() {
        let _ = std::fs::remove_dir_all(folder.join(STAGING));
    }
}

pub fn check_in_background() -> Receiver<Option<Release>> {
    let (send, receive) = channel();
    std::thread::spawn(move || {
        let _ = send.send(latest().filter(|release| is_newer(&release.tag, RELEASE)));
    });
    receive
}

pub fn install_in_background(release: Release) -> Installing {
    let progress = Arc::new(Mutex::new(0.0));
    let (send, done) = channel();
    let shared = progress.clone();
    std::thread::spawn(move || {
        let told = |fraction: f32| {
            if let Ok(mut progress) = shared.lock() {
                *progress = fraction;
            }
        };
        let _ = send.send(install(&release, &told));
    });
    Installing { progress, done }
}

pub fn restart(exe: &Path) -> std::io::Result<()> {
    Command::new(exe).args(std::env::args_os().skip(1)).spawn()?;
    std::process::exit(0);
}

fn launcher_folder() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(Path::to_owned)
}

fn latest() -> Option<Release> {
    let output = Fetcher::find()?.command(LATEST, None, 30).stderr(Stdio::null()).output().ok()?;
    output.status.success().then(|| parse(&output.stdout)).flatten()
}

fn parse(json: &[u8]) -> Option<Release> {
    let release: serde_json::Value = serde_json::from_slice(json).ok()?;
    let asset = release["assets"].as_array()?.iter().find(|asset| asset["name"].as_str().is_some_and(|name| name.ends_with(ASSET_SUFFIX)))?;
    Some(Release {
        tag: release["tag_name"].as_str()?.to_owned(),
        name: asset["name"].as_str()?.to_owned(),
        url: asset["browser_download_url"].as_str()?.to_owned(),
        size: asset["size"].as_u64()?,
    })
}

fn version(tag: &str) -> Option<(u32, u32, u32)> {
    let mut parts = tag.trim_start_matches('v').split('.').map(|part| part.parse().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

fn is_newer(tag: &str, current: &str) -> bool {
    matches!((version(tag), version(current)), (Some(latest), Some(current)) if latest > current)
}

fn install(release: &Release, progress: &dyn Fn(f32)) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let folder = exe.parent().ok_or("Zakuro can't tell which folder it runs from")?.to_owned();
    let staging = folder.join(STAGING);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|error| format!("{}: {error}", staging.display()))?;
    let fetcher = Fetcher::find().ok_or("Zakuro downloads with curl or wget, and this computer has neither")?;
    let archive = staging.join(&release.name);
    fetch(fetcher, &release.url, &archive, release.size, progress, &AtomicBool::new(false))?;
    let unpacked = windowless(&mut Command::new("tar"))
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&staging)
        .status()
        .map_err(|error| format!("tar could not start, {error}"))?;
    if !unpacked.success() {
        return Err("the download could not be unpacked".to_owned());
    }
    let files = staging.join(release.name.trim_end_matches(ASSET_SUFFIX));
    if !files.join(LAUNCHER).is_file() {
        return Err(format!("the download has no {LAUNCHER}"));
    }
    let old = exe.with_extension("exe.old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|error| format!("{}: {error}", exe.display()))?;
    replace_files(&files, &folder, &exe).inspect_err(|_| {
        let _ = std::fs::rename(&old, &exe);
    })?;
    Ok(exe)
}

fn replace_files(from: &Path, folder: &Path, exe: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(from).map_err(|error| format!("{}: {error}", from.display()))?;
    for entry in entries.flatten() {
        let target = if entry.file_name() == LAUNCHER { exe.to_owned() } else { folder.join(entry.file_name()) };
        std::fs::copy(entry.path(), &target).map_err(|error| format!("{}: {error}", target.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_later_version_is_newer() {
        assert!(is_newer("v0.2.10", "v0.2.9"));
        assert!(is_newer("v0.3.0", "v0.2.9"));
        assert!(!is_newer("v0.2.6", "v0.2.6"));
        assert!(!is_newer("v0.2.5", "v0.2.6"));
        assert!(!is_newer("latest", "v0.2.6"));
    }

    #[test]
    fn the_latest_release_names_its_zip() {
        let json = br#"{"tag_name":"v0.2.7","assets":[{"name":"notes.txt","browser_download_url":"https://x/notes.txt","size":3},{"name":"fe-awakening-launcher-v0.2.7-windows-x64.zip","browser_download_url":"https://x/z.zip","size":9000000}]}"#;
        let release = parse(json).unwrap();
        assert_eq!(release.tag, "v0.2.7");
        assert_eq!((release.url.as_str(), release.size), ("https://x/z.zip", 9_000_000));
        assert_eq!(release.name.trim_end_matches(ASSET_SUFFIX), "fe-awakening-launcher-v0.2.7");
        assert!(parse(br#"{"tag_name":"v0.2.7","assets":[]}"#).is_none());
    }
}
