//! Cleans compression artifacts out of the pictures of the game.
//! It runs the realesrgan-ncnn-vulkan program with a 1x model over the pictures, keeps the result only where it stays faithful, and adds a light grain to the parchment panels.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::ops::{gaussian, label, Mask, Plane, Rgba};
use crate::downloads::{Download, Need, Source};
use crate::zig::windowless;

/// The Windows build of realesrgan-ncnn-vulkan, as its author publishes it on GitHub.
const RUNNER: Source = Source {
    url: "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-windows.zip",
    name: "realesrgan-ncnn-vulkan-20220424-windows.zip",
    size: 45_474_481,
    sha256: "abc02804e17982a3be33675e4d471e91ea374e65b70167abc09e31acb412802d",
};

/// The BCGone Smooth weights, from the link that OpenModelDB lists for the model.
const WEIGHTS: Source = Source {
    url: "https://drive.google.com/uc?export=download&confirm=1&id=1EYAoxC1tE2EhEgI4tiKujHUKjhq68gY-",
    name: "1x_BCGone_Smooth.pth",
    size: 66_665_765,
    sha256: "095e69633e235350817065e6a72c733e38f2900f9271bc537844dd2b4bfe3f43",
};

/// The download that the cleanup needs, with the reason that is shown to the player.
/// The weights are converted by `prepare` once they have arrived.
pub const NEED: Need = Need {
    what: "an AI model, BCGone Smooth, and Real-ESRGAN's program that runs it",
    why: "The 3DS stores the game's pictures compressed, which leaves blocks and stripes in them. Before the words are drawn again, the model looks at each menu picture on your graphics card and smooths those blocks away, keeping the drawing as it is.",
    download: Download { folder: "ui-cleanup-1", sources: &[RUNNER, WEIGHTS], finish: Some(prepare) },
};

/// The model description that the converted model starts from.
/// It ships with the runner, and it describes the same network at four times the size.
const TEMPLATE: &str = "realesrgan-x4plus";

/// The files of the runner download that the cleanup never uses.
const UNUSED: [&str; 3] = ["input.jpg", "input2.jpg", "onepiece_demo.mp4"];

/// Turns the downloaded weights into a model that the runner reads.
/// The runner reads ncnn files, while the weights come as a PyTorch file.
/// The folder then keeps only the program and the converted model.
fn prepare(folder: &Path) -> Result<(), String> {
    let models = folder.join("models");
    let template = std::fs::read_to_string(models.join(format!("{TEMPLATE}.param"))).map_err(|error| format!("{}: {error}", models.display()))?;
    let weights = std::fs::read(folder.join(WEIGHTS.name)).map_err(|error| format!("{}: {error}", WEIGHTS.name))?;
    let (param, bin) = super::esrgan::one_x(&template, &weights)?;
    std::fs::write(models.join(format!("{MODEL}.param")), param).map_err(|error| format!("{}: {error}", models.display()))?;
    std::fs::write(models.join(format!("{MODEL}.bin")), bin).map_err(|error| format!("{}: {error}", models.display()))?;
    // Remove what the cleanup does not use, to keep the folder small.
    let _ = std::fs::remove_file(folder.join(WEIGHTS.name));
    UNUSED.iter().for_each(|name| drop(std::fs::remove_file(folder.join(name))));
    for entry in std::fs::read_dir(&models).into_iter().flatten().flatten() {
        if !entry.file_name().to_string_lossy().starts_with(MODEL) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}

/// The folders of the romfs whose pictures are cleaned.
pub const FOLDERS: [&str; 14] = ["ui", "ui/resident", "gmap", "systems", "room", "jobintro", "bs", "telop", "tut", "tut/F", "tut/G", "tut/I", "tut/S", "tut/U"];
/// The 3DS texture formats that store pictures in compressed blocks: ETC1 and ETC1A4.
/// The model removes the traces of these blocks.
/// The formats with fewer bits per colour, such as RGBA4, hold pixel art without blocks, which the model would only smear.
const BLOCK_COMPRESSED: [u32; 2] = [12, 13];
/// The file name of the program in the download.
const PROGRAM: &str = "realesrgan-ncnn-vulkan.exe";
/// The name of the converted model, without an extension.
const MODEL: &str = "1x-bcgone";
/// The number of passes that spread the colour into the clear pixels.
const BLEED_PASSES: usize = 16;
/// The alpha from which a pixel counts as solid.
const SOLID: f32 = 0.99;
/// The change of a colour channel up to which the colour of the model is fully taken.
/// The traces of the compressed blocks are small changes like these.
const SMALL_CHANGE: f32 = 0.06;
/// The change of a colour channel from which the colour of the model is refused, and the original is kept.
/// Changes this large come from the model blending in a neighbouring colour or smoothing away a fine detail.
const LARGE_CHANGE: f32 = 0.14;
/// The time without a new finished picture after which the program counts as stuck.
const STALL: Duration = Duration::from_secs(60);
/// The size in pixels under which a separate shape counts as a tiny icon, by its largest side.
const TINY_ICON: usize = 24;
/// The largest mean difference that a cleaned picture may have from the original.
const FIDELITY: f32 = 0.025;
/// The number of pictures that the program gets in one run.
const BATCH: usize = 64;
/// The extra options for the second try, which uses small tiles and a single thread per stage to save memory.
const RETRY: [&str; 4] = ["-t", "64", "-j", "1:1:1"];
/// The strength of the grain.
const GRAIN: f32 = 0.01;
/// The blur of the noise of the grain, in pixels.
const GRAIN_SOFTNESS: f32 = 0.6;
/// The slope of the brightness above which the grain fades out completely.
const FLAT: f32 = 0.08;
/// The range of the hue, in degrees, of the parchment colour.
const PARCHMENT_HUE: (f32, f32) = (28.0, 44.0);
/// The range of the saturation of the parchment colour.
const PARCHMENT_SATURATION: (f32, f32) = (0.25, 0.8);
/// The range of the value of the parchment colour.
const PARCHMENT_VALUE: (f32, f32) = (0.55, 0.97);
/// The smallest area in pixels of a patch of parchment colour that counts as a panel.
const PANEL_AREA: usize = 600;

/// Tells whether a 3DS texture format stores compressed blocks, and so is worth cleaning.
pub fn block_compressed(format: u32) -> bool {
    BLOCK_COMPRESSED.contains(&format)
}

/// Runs the cleanup program from its folder.
pub struct Cleaner {
    folder: PathBuf,
}

impl Cleaner {
    /// Creates a cleaner for the folder that holds the program and the model.
    pub fn in_folder(folder: &Path) -> Cleaner {
        Cleaner { folder: folder.to_owned() }
    }

    /// Cleans the pictures, which are given with their names, and returns the cleaned ones by name.
    /// Pictures whose result is not faithful, or that the program fails to produce, are left out.
    /// The progress function gets the number of pictures that are done.
    pub fn clean(&self, pictures: &[(String, Rgba)], progress: &dyn Fn(usize), cancel: &AtomicBool) -> Result<HashMap<String, Rgba>, String> {
        let work = self.folder.join("work");
        let _ = std::fs::remove_dir_all(&work);
        // The program takes opaque pictures, so the colour is first spread into the clear pixels.
        let inputs: Vec<Rgba> = pictures.iter().map(|(_, picture)| bled(picture)).collect();
        // A picture smaller than a tiny icon is left as it is, which also spares the program the very small pictures it can hang on.
        let everything: Vec<usize> = (0..pictures.len()).filter(|&i| pictures[i].1.w.max(pictures[i].1.h) >= TINY_ICON).collect();
        let mut outputs = HashMap::new();
        for (n, batch) in everything.chunks(BATCH).enumerate() {
            outputs.extend(self.run(&work.join(format!("first{n}")), &inputs, batch, &[], &|done| progress(n * BATCH + done), cancel)?);
        }
        // The pictures that came out missing or unfaithful get a second try with the gentler options.
        let missed: Vec<usize> = everything.into_iter().filter(|i| !outputs.contains_key(i)).collect();
        for (n, batch) in missed.chunks(BATCH).enumerate() {
            outputs.extend(self.run(&work.join(format!("again{n}")), &inputs, batch, &RETRY, &|_| {}, cancel)?);
        }
        let _ = std::fs::remove_dir_all(&work);
        let mut cleaned = HashMap::new();
        for (i, (name, picture)) in pictures.iter().enumerate() {
            let Some(doubled) = outputs.get(&i) else { continue };
            // The result is twice the size, so each pixel takes the colour at double its coordinates.
            // Only solid pixels are replaced, and the alpha is kept.
            // The tiny icons keep their own pixels, since the model only smears pixel art that small.
            let tiny = tiny_icons(picture);
            let mut out = picture.clone();
            for (j, p) in out.v.iter_mut().enumerate().filter(|(j, p)| p[3] >= SOLID && !tiny.v[*j]) {
                let q = doubled.get_pixel((j % picture.w * 2) as u32, (j / picture.w * 2) as u32);
                let model = [q[0] as f32 / 255.0, q[1] as f32 / 255.0, q[2] as f32 / 255.0];
                let trust = trust(p, &model);
                for k in 0..3 {
                    p[k] += trust * (model[k] - p[k]);
                }
            }
            grain(&mut out, name);
            cleaned.insert(name.clone(), out);
        }
        Ok(cleaned)
    }

    /// Runs the program once over a selection of the pictures, in a work folder.
    /// Returns the results by index, at twice the size, for the pictures that came out faithful.
    fn run(&self, work: &Path, inputs: &[Rgba], which: &[usize], options: &[&str], progress: &dyn Fn(usize), cancel: &AtomicBool) -> Result<HashMap<usize, image::RgbaImage>, String> {
        let (input, output) = (work.join("in"), work.join("out"));
        for folder in [&input, &output] {
            std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
        }
        for &i in which {
            write_opaque(&input.join(format!("{i}.png")), &inputs[i])?;
        }
        let mut child = windowless(&mut Command::new(self.folder.join(PROGRAM)))
            .arg("-i")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .arg("-m")
            .arg(self.folder.join("models"))
            // The model name, a scale of 2 and PNG output.
            .args(["-n", MODEL, "-s", "2", "-f", "png"])
            .args(options)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("the cleanup could not start, {error}"))?;
        // Wait for the program, and count the finished pictures in the output folder to report the progress.
        let mut last_progress = (0, Instant::now());
        let status = loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("cancelled".to_owned());
            }
            let done = std::fs::read_dir(&output).map_or(0, |entries| entries.count());
            progress(done);
            if done != last_progress.0 {
                last_progress = (done, Instant::now());
            }
            // The program can stop making progress on a picture.
            // It is then stopped, and the pictures it finished are kept, while the others go to the second try.
            if last_progress.1.elapsed() > STALL {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(250)),
                Err(error) => return Err(error.to_string()),
            }
        };
        if status.is_some_and(|status| !status.success()) {
            return Err("the cleanup needs a graphics card with Vulkan".to_owned());
        }
        Ok(which
            .iter()
            .filter_map(|&i| {
                let doubled = image::open(output.join(format!("{i}.png"))).ok()?.to_rgba8();
                faithful(&doubled, &inputs[i]).then_some((i, doubled))
            })
            .collect())
    }
}

/// Marks the pixels of the tiny icons of a picture.
/// A tiny icon is a separate shape, surrounded by clear pixels, whose largest side is under TINY_ICON.
fn tiny_icons(picture: &Rgba) -> Mask {
    let shapes = Mask { w: picture.w, h: picture.h, v: picture.v.iter().map(|p| p[3] > 0.0).collect() };
    let (labels, count) = label(&shapes, true);
    // The bounding box of each shape, as the smallest and the largest x and y.
    let mut boxes = vec![(usize::MAX, usize::MAX, 0, 0); count + 1];
    for (i, &l) in labels.iter().enumerate() {
        let (x, y) = (i % picture.w, i / picture.w);
        let b = &mut boxes[l as usize];
        *b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    let tiny = |l: u32| l != 0 && (boxes[l as usize].2 - boxes[l as usize].0).max(boxes[l as usize].3 - boxes[l as usize].1) < TINY_ICON;
    Mask { w: picture.w, h: picture.h, v: labels.iter().map(|&l| tiny(l)).collect() }
}

/// Tells how much of the colour of the model a pixel takes, from 0 to 1.
/// A small change is taken in full, a large change is refused, and a change in between is blended.
fn trust(original: &[f32; 4], model: &[f32; 3]) -> f32 {
    let change = (0..3).map(|k| (model[k] - original[k]).abs()).fold(0.0, f32::max);
    ((LARGE_CHANGE - change) / (LARGE_CHANGE - SMALL_CHANGE)).clamp(0.0, 1.0)
}

/// Tells whether a cleaned picture is twice the size of the input and stays close to it.
/// The mean difference per channel must not exceed the fidelity limit, which rejects results such as a black picture.
fn faithful(doubled: &image::RgbaImage, input: &Rgba) -> bool {
    if doubled.width() as usize != input.w * 2 || doubled.height() as usize != input.h * 2 {
        return false;
    }
    let difference: f32 = input
        .v
        .iter()
        .enumerate()
        .map(|(j, p)| {
            let q = doubled.get_pixel((j % input.w * 2) as u32, (j / input.w * 2) as u32);
            (0..3).map(|k| (q[k] as f32 / 255.0 - p[k]).abs()).sum::<f32>() / 3.0
        })
        .sum();
    difference / input.v.len().max(1) as f32 <= FIDELITY
}

/// Spreads the colour of the solid pixels into the clear pixels around them.
/// Each pass gives a pixel the mean colour of its known neighbours, and the alpha is not changed.
/// This keeps the colour of the transparent pixels from smearing the edges when the picture is cleaned as an opaque picture.
fn bled(picture: &Rgba) -> Rgba {
    let (w, h) = (picture.w, picture.h);
    let mut out = picture.clone();
    let mut known: Vec<bool> = picture.v.iter().map(|p| p[3] >= SOLID).collect();
    for _ in 0..BLEED_PASSES {
        let reached: Vec<(usize, [f32; 3])> = (0..w * h)
            .filter(|&i| !known[i])
            .filter_map(|i| {
                let (x, y) = (i % w, i / w);
                let around: Vec<[f32; 4]> = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                    .iter()
                    .filter_map(|&(dx, dy)| {
                        let (nx, ny) = (x as isize + dx, y as isize + dy);
                        (nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h && known[ny as usize * w + nx as usize]).then(|| out.v[ny as usize * w + nx as usize])
                    })
                    .collect();
                (!around.is_empty()).then(|| (i, [0, 1, 2].map(|k| around.iter().map(|p| p[k]).sum::<f32>() / around.len() as f32)))
            })
            .collect();
        if reached.is_empty() {
            break;
        }
        for (i, rgb) in reached {
            out.v[i] = [rgb[0], rgb[1], rgb[2], out.v[i][3]];
            known[i] = true;
        }
    }
    out
}

/// Tells whether a pixel has the warm tan colour of the parchment panels.
/// The pixel is converted to hue, saturation and value, and each is tested against its range.
fn parchment(p: [f32; 4]) -> bool {
    let (high, low) = (p[0].max(p[1]).max(p[2]), p[0].min(p[1]).min(p[2]));
    if p[3] < SOLID || high <= 0.0 || high == low {
        return false;
    }
    let chroma = high - low;
    // The position on the colour wheel, in sixths.
    let sector = if high == p[0] {
        ((p[1] - p[2]) / chroma).rem_euclid(6.0)
    } else if high == p[1] {
        (p[2] - p[0]) / chroma + 2.0
    } else {
        (p[0] - p[1]) / chroma + 4.0
    };
    let (hue, saturation) = (sector * 60.0, chroma / high);
    (PARCHMENT_HUE.0..=PARCHMENT_HUE.1).contains(&hue) && (PARCHMENT_SATURATION.0..=PARCHMENT_SATURATION.1).contains(&saturation) && (PARCHMENT_VALUE.0..=PARCHMENT_VALUE.1).contains(&high)
}

/// Marks the pixels that belong to a panel, which is a connected patch of parchment colour that is large enough.
fn panels(picture: &Rgba) -> Mask {
    let warm = Mask { w: picture.w, h: picture.h, v: picture.v.iter().map(|&p| parchment(p)).collect() };
    let (labels, count) = label(&warm, false);
    let mut sizes = vec![0usize; count + 1];
    labels.iter().for_each(|&l| sizes[l as usize] += 1);
    Mask { w: picture.w, h: picture.h, v: labels.iter().map(|&l| l != 0 && sizes[l as usize] >= PANEL_AREA).collect() }
}

/// Adds a faint film grain to the flat parts of the panels, which the cleanup leaves too smooth.
/// The noise is deterministic, as it depends only on the name of the picture, so a rebuild gives the same pictures.
fn grain(picture: &mut Rgba, name: &str) {
    let (w, h) = (picture.w, picture.h);
    let panel = panels(picture);
    if !panel.any() {
        return;
    }
    // The seed is the FNV-1a hash of the name.
    let seed = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| (hash ^ byte as u64).wrapping_mul(0x0100_0000_01b3));
    let mut noise = Plane::new(w, h);
    for (i, v) in noise.v.iter_mut().enumerate() {
        // A SplitMix64 step turns the position and the seed into a random number, which is mapped to the range from -1 to 1.
        let mut z = seed.wrapping_add((i as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        *v = (z >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0;
    }
    // The blurred noise is divided by its root mean square, so that the strength does not depend on the blur.
    let soft = gaussian(&noise, GRAIN_SOFTNESS);
    let spread = (soft.v.iter().map(|v| v * v).sum::<f32>() / soft.v.len().max(1) as f32).sqrt().max(1e-4);
    // The luma uses the Rec. 601 weights.
    let lum: Vec<f32> = picture.v.iter().map(|p| p[0] * 0.299 + p[1] * 0.587 + p[2] * 0.114).collect();
    let at = |x: usize, y: usize| lum[y.min(h - 1) * w + x.min(w - 1)];
    for y in 0..h {
        for x in 0..w {
            if !panel.at(x, y) {
                continue;
            }
            let p = &mut picture.v[y * w + x];
            // The grain fades out where the brightness changes quickly, so that it only shows on flat areas.
            let slope = (at(x + 1, y) - at(x.saturating_sub(1), y)).abs() + (at(x, y + 1) - at(x, y.saturating_sub(1))).abs();
            let amount = GRAIN * (1.0 - slope / FLAT).clamp(0.0, 1.0) * soft.v[y * w + x] / spread;
            for value in p.iter_mut().take(3) {
                *value = (*value + amount).clamp(0.0, 1.0);
            }
        }
    }
}

/// Writes a picture as an opaque PNG, which drops the alpha.
fn write_opaque(path: &Path, picture: &Rgba) -> Result<(), String> {
    let bytes: Vec<u8> = picture.v.iter().flat_map(|p| [p[0], p[1], p[2]].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)).collect();
    image::save_buffer(path, &bytes, picture.w as u32, picture.h as u32, image::ExtendedColorType::Rgb8).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clear pixel gets the mean colour of its solid neighbours, and solid pixels stay as they are.
    #[test]
    fn clear_pixels_take_the_colour_beside_them() {
        let mut picture = Rgba::new(3, 1);
        picture.v = vec![[1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 1.0, 1.0]];
        let out = bled(&picture);
        assert_eq!(out.v[1], [0.5, 0.0, 0.5, 0.0]);
        assert_eq!(out.v[0], picture.v[0]);
    }

    /// Grain changes a parchment panel a little, leaves clear pixels alone, and does not touch other colours.
    #[test]
    fn parchment_gets_a_light_grain_and_other_colours_none() {
        let tan = [0.86, 0.68, 0.42, 1.0];
        let mut picture = Rgba::new(32, 32);
        picture.v.iter_mut().enumerate().for_each(|(i, p)| *p = if i == 0 { [0.86, 0.68, 0.42, 0.0] } else { tan });
        grain(&mut picture, "panel");
        let spread = picture.v[1..].iter().map(|p| (p[0] - tan[0]).abs()).fold(0.0f32, f32::max);
        assert!(spread > 0.003 && spread < 0.1, "{spread}");
        assert_eq!(picture.v[0], [0.86, 0.68, 0.42, 0.0]);
        let mut green = Rgba::new(32, 32);
        green.v.iter_mut().for_each(|p| *p = [0.3, 0.45, 0.1, 1.0]);
        grain(&mut green, "card");
        assert!(green.v.iter().all(|p| *p == [0.3, 0.45, 0.1, 1.0]));
    }

    /// A result of the right size that is close to the input is accepted, and a black result is refused.
    #[test]
    fn a_black_result_is_refused() {
        let mut input = Rgba::new(4, 4);
        input.v.iter_mut().for_each(|p| *p = [0.8, 0.7, 0.5, 1.0]);
        let same = image::RgbaImage::from_pixel(8, 8, image::Rgba([204, 178, 128, 255]));
        let black = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]));
        assert!(faithful(&same, &input) && !faithful(&black, &input));
    }

    /// A small separate shape counts as a tiny icon, while a large one does not.
    #[test]
    fn small_shapes_are_tiny_icons() {
        let mut picture = Rgba::new(64, 64);
        for (i, p) in picture.v.iter_mut().enumerate() {
            let (x, y) = (i % 64, i / 64);
            if (x < 8 && y < 8) || (x >= 20 && y >= 20) {
                *p = [0.5, 0.5, 0.5, 1.0];
            }
        }
        let tiny = tiny_icons(&picture);
        assert!(tiny.v[0] && tiny.v[7 * 64 + 7] && !tiny.v[30 * 64 + 30] && !tiny.v[10 * 64 + 10]);
    }

    /// Small changes of the model are taken, large ones are refused, and the ones in between are blended.
    #[test]
    fn large_changes_are_refused() {
        let original = [0.5, 0.5, 0.5, 1.0];
        assert_eq!(trust(&original, &[0.53, 0.5, 0.5]), 1.0);
        assert_eq!(trust(&original, &[0.5, 0.8, 0.5]), 0.0);
        assert!((trust(&original, &[0.5, 0.5, 0.4]) - 0.5).abs() < 1e-4);
    }

    /// Only the ETC1 formats are cleaned, and the pixel art formats are not.
    #[test]
    fn only_block_compressed_formats_are_cleaned() {
        assert!(block_compressed(12) && block_compressed(13) && !block_compressed(4) && !block_compressed(2) && !block_compressed(0) && !block_compressed(11));
    }
}
