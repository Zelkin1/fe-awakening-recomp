//! Self-update of the launcher. This module asks GitHub for the latest release, downloads the asset of the release for this system and replaces the launcher with it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use crate::zig::{fetch, windowless, Fetcher};

/// The tag of the release this build belongs to.
pub const RELEASE: &str = "v1.0.14";
/// The GitHub API address that describes the latest release.
const LATEST: &str = "https://api.github.com/repos/Zelkin1/fe-awakening-recomp/releases/latest";
/// The end of the file name of the release asset that holds the launcher for this system: a zip on Windows, an AppImage on Linux.
const ASSET_SUFFIX: &str = if cfg!(windows) { "-windows-x64.zip" } else { "-x86_64.AppImage" };
/// The folder next to the launcher where an update is downloaded and unpacked.
const STAGING: &str = "update";
/// A file that ships in the folder of a release, used to recognise an installed launcher.
const LICENSES: &str = "THIRD_PARTY_LICENSES.txt";
/// The file name of the launcher program.
const LAUNCHER: &str = "zakuro.exe";

/// A release on GitHub that can be installed.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    /// The version tag, for example "v1.0.11".
    tag: String,
    /// The file name of the zip asset.
    name: String,
    /// The address the zip asset is downloaded from.
    url: String,
    /// The size of the zip asset in bytes.
    size: u64,
}

/// An installation that runs on a background thread.
pub struct Installing {
    /// The fraction of the download that has arrived, from 0 to 1.
    pub progress: Arc<Mutex<f32>>,
    /// Receives the path of the new launcher when the installation succeeds, or the reason it failed.
    pub done: Receiver<Result<PathBuf, String>>,
}

/// Returns true when the launcher can replace itself: on Windows when it runs from the folder of a release, elsewhere when it runs as an AppImage.
pub fn installable() -> bool {
    if cfg!(windows) { launcher_folder().is_some_and(|folder| folder.join(LICENSES).is_file()) } else { running_appimage().is_some() }
}

/// Returns the path of the AppImage the launcher runs from, which the AppImage runtime gives in an environment variable.
fn running_appimage() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE").map(PathBuf::from).filter(|path| path.is_file())
}

/// Removes the leftovers of an earlier update: the old launcher and the staging folder.
pub fn tidy() {
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = std::fs::remove_file(exe.with_extension("exe.old"));
    if let Some(folder) = exe.parent() {
        let _ = std::fs::remove_dir_all(folder.join(STAGING));
    }
}

/// Asks GitHub for the latest release on a thread. The receiver gets the release when it is newer than this build, and nothing otherwise.
pub fn check_in_background() -> Receiver<Option<Release>> {
    let (send, receive) = channel();
    std::thread::spawn(move || {
        let _ = send.send(latest().filter(|release| is_newer(&release.tag, RELEASE)));
    });
    receive
}

/// Starts the installation of a release on a thread and returns the handles to follow it.
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

/// Starts the launcher at the given path with the same arguments as this run, then ends this process.
pub fn restart(exe: &Path) -> std::io::Result<()> {
    Command::new(exe).args(std::env::args_os().skip(1)).spawn()?;
    std::process::exit(0);
}

/// Returns the folder that holds the running launcher.
fn launcher_folder() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(Path::to_owned)
}

/// Downloads the description of the latest release and reads it. Returns nothing when the request or the reading fails.
fn latest() -> Option<Release> {
    let output = Fetcher::find()?.command(LATEST, None, 30).stderr(Stdio::null()).output().ok()?;
    output.status.success().then(|| parse(&output.stdout)).flatten()
}

/// Reads the JSON of a GitHub release and picks the Windows zip asset. Returns nothing when the release has no such asset.
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

/// Splits a tag such as "v1.0.11" into major, minor and patch numbers. Returns nothing when the tag has another shape.
fn version(tag: &str) -> Option<(u32, u32, u32)> {
    let mut parts = tag.trim_start_matches('v').split('.').map(|part| part.parse().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// Returns true when the first tag is a later version than the second. A tag that is not a version is never newer.
fn is_newer(tag: &str, current: &str) -> bool {
    matches!((version(tag), version(current)), (Some(latest), Some(current)) if latest > current)
}

/// Downloads the AppImage of a release next to the running one and puts it in its place. Returns the path of the AppImage.
fn install_appimage(release: &Release, progress: &dyn Fn(f32)) -> Result<PathBuf, String> {
    let current = running_appimage().ok_or("the launcher updates itself only when it runs as an AppImage")?;
    let fetcher = Fetcher::find().ok_or("Zakuro downloads with curl or wget, and this computer has neither")?;
    let part = current.with_extension("AppImage.part");
    fetch(fetcher, &release.url, &part, release.size, progress, &AtomicBool::new(false))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755)).map_err(|error| format!("{}: {error}", part.display()))?;
    }
    // Linux lets a file be renamed over a running program, which keeps running from the old file until it ends.
    std::fs::rename(&part, &current).map_err(|error| format!("{}: {error}", current.display()))?;
    Ok(current)
}

/// Downloads and installs a release. Returns the path of the new launcher.
/// On Windows the zip is unpacked and the files of the launcher are replaced.
/// Elsewhere the AppImage is replaced.
fn install(release: &Release, progress: &dyn Fn(f32)) -> Result<PathBuf, String> {
    if !cfg!(windows) {
        return install_appimage(release, progress);
    }
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
    // The zip unpacks into a folder named like the asset without its suffix.
    let files = staging.join(release.name.trim_end_matches(ASSET_SUFFIX));
    if !files.join(LAUNCHER).is_file() {
        return Err(format!("the download has no {LAUNCHER}"));
    }
    // Windows allows renaming a running program but not overwriting it, so the old launcher moves aside first.
    let old = exe.with_extension("exe.old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|error| format!("{}: {error}", exe.display()))?;
    // When copying fails, the old launcher moves back.
    replace_files(&files, &folder, &exe).inspect_err(|_| {
        let _ = std::fs::rename(&old, &exe);
    })?;
    Ok(exe)
}

/// Copies every file of the unpacked release into the folder of the launcher. The new launcher goes to the path of the running one.
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
    fn the_latest_release_names_the_asset_of_this_system() {
        let json = br#"{"tag_name":"v0.2.7","assets":[{"name":"notes.txt","browser_download_url":"https://x/notes.txt","size":3},{"name":"fe-awakening-launcher-v0.2.7-windows-x64.zip","browser_download_url":"https://x/z.zip","size":9000000},{"name":"fe-awakening-launcher-v0.2.7-x86_64.AppImage","browser_download_url":"https://x/a.AppImage","size":40000000}]}"#;
        let release = parse(json).unwrap();
        assert_eq!(release.tag, "v0.2.7");
        let expected = if cfg!(windows) { ("https://x/z.zip", 9_000_000) } else { ("https://x/a.AppImage", 40_000_000) };
        assert_eq!((release.url.as_str(), release.size), expected);
        assert_eq!(release.name.trim_end_matches(ASSET_SUFFIX), "fe-awakening-launcher-v0.2.7");
        assert!(parse(br#"{"tag_name":"v0.2.7","assets":[]}"#).is_none());
    }
}
