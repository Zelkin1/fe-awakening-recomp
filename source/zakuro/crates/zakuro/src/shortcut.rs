use std::path::{Path, PathBuf};
use std::process::Command;

use crate::library::ICON_SIZE;

pub fn create(name: &str, icon: Option<&[u8]>) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let folder = exe.parent().ok_or("the launcher's folder is unknown")?;
    let file_name: String = name.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { ' ' } else { c }).collect();
    let file_name = file_name.trim().to_owned();
    let icon_path = match icon {
        Some(rgba) => {
            let path = folder.join(format!("{file_name}.ico"));
            std::fs::write(&path, ico(rgba)).map_err(|error| format!("the icon can't be saved: {error}"))?;
            path
        }
        None => exe.clone(),
    };
    link(&exe, folder, &icon_path, &file_name)
}

fn link(exe: &Path, folder: &Path, icon: &Path, name: &str) -> Result<PathBuf, String> {
    const SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
        $path = Join-Path ([Environment]::GetFolderPath('Desktop')) ($env:ZAKURO_NAME + '.lnk'); \
        $link = (New-Object -ComObject WScript.Shell).CreateShortcut($path); \
        $link.TargetPath = $env:ZAKURO_EXE; \
        $link.WorkingDirectory = $env:ZAKURO_FOLDER; \
        $link.IconLocation = $env:ZAKURO_ICON + ',0'; \
        $link.Save(); \
        [Console]::Out.Write($path)";
    let mut command = Command::new("powershell");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .env("ZAKURO_NAME", name)
        .env("ZAKURO_EXE", exe)
        .env("ZAKURO_FOLDER", folder)
        .env("ZAKURO_ICON", icon);
    let output = crate::zig::windowless(&mut command).output().map_err(|error| format!("PowerShell can't start: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
}

fn ico(rgba: &[u8]) -> Vec<u8> {
    let side = ICON_SIZE;
    let mask_row = (side.div_ceil(8) + 3) / 4 * 4;
    let image_size = 40 + side * side * 4 + mask_row * side;
    let mut out = Vec::with_capacity(22 + image_size);
    out.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    out.extend_from_slice(&[side as u8, side as u8, 0, 0, 1, 0, 32, 0]);
    out.extend_from_slice(&(image_size as u32).to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(side as i32).to_le_bytes());
    out.extend_from_slice(&(side as i32 * 2).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..side).rev() {
        for pixel in rgba[y * side * 4..(y + 1) * side * 4].chunks_exact(4) {
            out.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    out.resize(out.len() + mask_row * side, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_file_holds_the_pixels_bottom_up() {
        let mut rgba = vec![0; ICON_SIZE * ICON_SIZE * 4];
        rgba[..4].copy_from_slice(&[1, 2, 3, 255]);
        let file = ico(&rgba);
        assert_eq!(file.len(), 22 + 40 + ICON_SIZE * ICON_SIZE * 4 + 8 * ICON_SIZE);
        let top_left = 22 + 40 + (ICON_SIZE - 1) * ICON_SIZE * 4;
        assert_eq!(&file[top_left..top_left + 4], &[3, 2, 1, 255]);
    }
}
