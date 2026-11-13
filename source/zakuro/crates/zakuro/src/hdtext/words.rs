//! Redraws words in the pictures of the menus, the buttons and the title screen.
//! This module finds a word in a box of a picture.
//! It erases the letters of the word.
//! It draws the same text again in the new font at four times the size.
//! The tables hold the boxes of the words in the pictures of the game, in pixels of the original pictures.

use super::banners;
use super::ops::{dilate, gaussian, resize, Filter, Mask, Plane, Rgba};
use super::text::Face;
use super::titles::{place, search, SCALE, SS, TITLE};

/// A box in a picture, as left, top, right and bottom in pixels.
pub type Box4 = [usize; 4];

/// The box of the text of the "press any button" picture.
pub const PRESS_ANY_BUTTON_BOX: Box4 = [44, 0, 212, 32];
/// The boxes of the "press any button" picture whose strokes are kept as they are, which are the two ornaments at the ends.
pub const PRESS_ANY_BUTTON_KEEP: [Box4; 2] = [[0, 0, 47, 32], [211, 0, 256, 32]];
/// The boxes of the eight texts of the main menu.
pub const MENU_BOXES: [Box4; 8] = [[0, 0, 170, 24], [0, 24, 170, 50], [0, 50, 170, 73], [0, 73, 170, 97], [0, 97, 170, 126], [170, 26, 256, 40], [170, 67, 256, 84], [170, 88, 256, 124]];
/// The boxes of the main menu picture whose strokes are kept as they are.
pub const MENU_KEEP: [Box4; 2] = [[170, 0, 256, 26], [170, 40, 256, 67]];
/// The box of the copyright line.
pub const COPYRIGHT_BOX: Box4 = [0, 0, 512, 32];
/// The offset of the texture of the battle pictures that holds the numbers and the words shown over a fight.
pub const BATTLE_TEXT_AT: usize = 0x100;
/// The slant of the italic numbers and words of a fight.
const BATTLE_SLANT: f32 = 0.18;
/// The share of the height of the old digits of the damage that the new digits take.
const BATTLE_DIGIT_SIZE: f32 = 0.92;
/// The letters of the battle picture that every language shares, as a box and a text.
/// The digits of the damage come first, in cells of 24 pixels that end above the gauges on row 26.
/// The letters of "NO DAMAGE!" and "MISS!" come next, in cells of 16 pixels.
/// The bonus of a skill comes last.
const BATTLE_SHARED: [(Box4, &str); 26] = [
    ([0, 0, 24, 26], "0"),
    ([24, 0, 48, 26], "1"),
    ([48, 0, 72, 26], "2"),
    ([72, 0, 96, 26], "3"),
    ([96, 0, 120, 26], "4"),
    ([120, 0, 144, 26], "5"),
    ([144, 0, 168, 26], "6"),
    ([168, 0, 192, 26], "7"),
    ([192, 0, 216, 26], "8"),
    ([216, 0, 240, 26], "9"),
    ([0, 32, 16, 48], "N"),
    ([16, 32, 32, 48], "O"),
    ([32, 32, 48, 48], "D"),
    ([48, 32, 64, 48], "A"),
    ([64, 32, 80, 48], "M"),
    ([80, 32, 96, 48], "G"),
    ([96, 32, 112, 48], "E"),
    ([112, 32, 128, 48], "!"),
    ([0, 48, 16, 64], "M"),
    ([16, 48, 32, 64], "I"),
    ([32, 48, 48, 64], "S"),
    ([48, 48, 64, 64], "!"),
    ([64, 48, 72, 64], "+"),
    ([72, 48, 96, 64], "10"),
    ([96, 48, 120, 64], "15"),
    ([120, 48, 144, 64], "20"),
];
/// The boxes of the four names of the stats that a skill raises: hit, critical, avoid and dodge.
const BATTLE_LABELS: [Box4; 4] = [[168, 32, 200, 48], [224, 32, 256, 48], [168, 48, 200, 64], [224, 48, 256, 64]];
/// The box of the word of a critical hit.
const BATTLE_CRITICAL: Box4 = [0, 68, 128, 100];
/// The rows of the words of no damage and of a miss, which are spaced out with one letter per cell of 16 pixels.
/// The letters of the first row end on row 118, and the accents of the second start on row 119.
const BATTLE_SPACED_ROWS: [(usize, usize); 2] = [(100, 119), (119, 137)];
/// The width of a cell of a spaced out word.
const BATTLE_CELL: usize = 16;

/// The words of the battle picture of one language, from the names of the four stats, the word of a critical hit, and the words of no damage and of a miss.
pub fn battle_text(labels: &[&'static str; 4], critical: &'static str, spaced: [&'static str; 2]) -> Vec<Word> {
    let painted = |at: Box4, text: &'static str, slant: f32| Word { at, text, ink: Ink::Opaque, look: Look::GradedOutline, align: Align::Centre, slant, size: 1.0 };
    // The word of a critical hit is read from the left edge of the picture, so it stays there.
    let critical = Word { align: Align::Left, ..painted(BATTLE_CRITICAL, critical, BATTLE_SLANT) };
    // The digits of the damage are smaller than the old ones, whose thick outline adds to their height.
    let shared = BATTLE_SHARED.iter().map(|&(at, text)| Word { size: if at[1] == 0 { BATTLE_DIGIT_SIZE } else { 1.0 }, ..painted(at, text, BATTLE_SLANT) });
    let names = BATTLE_LABELS.iter().zip(labels).map(|(&at, &text)| painted(at, text, 0.0));
    // Each letter of a spaced out word sits in a cell of its own, so it is redrawn there alone.
    let letters = spaced.into_iter().zip(BATTLE_SPACED_ROWS).flat_map(|(word, (top, bottom))| {
        word.char_indices().enumerate().filter(|(_, (_, c))| *c != ' ').map(move |(cell, (at, c))| {
            painted([cell * BATTLE_CELL, top, (cell + 1) * BATTLE_CELL, bottom], &word[at..at + c.len_utf8()], BATTLE_SLANT)
        })
    });
    shared.chain(names).chain(std::iter::once(critical)).chain(letters).collect()
}

/// The offset of the texture of the system pictures that holds the words of the big buttons.
pub const BUTTON_WORDS_AT: usize = 0x1300;
/// The part of that texture that holds only the words of the big buttons.
pub const BUTTON_WORD_AREA: Box4 = [104, 331, 220, 475];
/// The boxes of the six words of the big buttons, one per row of 24 pixels from the top.
pub const BUTTON_WORD_BOXES: [Box4; 6] = [[104, 331, 220, 353], [104, 355, 220, 377], [104, 379, 220, 401], [104, 403, 220, 425], [104, 427, 220, 449], [104, 451, 220, 473]];
/// The buttons of the world map that hold words, as a box, an index into the words of the map and the ink.
pub const GMAP_BUTTONS: [(Box4, usize, Ink); 8] = [
    ([147, 11, 181, 30], 0, Ink::Dark),
    ([186, 21, 221, 34], 1, Ink::Light),
    ([147, 61, 181, 75], 2, Ink::Dark),
    ([187, 61, 221, 74], 1, Ink::Dark),
    ([66, 91, 101, 110], 0, Ink::Light),
    ([107, 91, 141, 110], 0, Ink::Dark),
    ([146, 100, 181, 116], 2, Ink::Light),
    ([187, 91, 221, 110], 0, Ink::Dark),
];
/// The words of the chapter overview, as a box, an index into the overview words and the ink.
pub const OVERVIEW: [(Box4, usize, Ink); 9] = [
    ([192, 23, 270, 39], 0, Ink::Light),
    ([191, 81, 238, 98], 1, Ink::Dark),
    ([180, 202, 202, 215], 2, Ink::Light),
    ([228, 202, 251, 216], 3, Ink::Light),
    ([182, 220, 201, 233], 4, Ink::Light),
    ([318, 186, 406, 202], 5, Ink::Opaque),
    ([318, 202, 406, 219], 6, Ink::Opaque),
    ([318, 222, 406, 239], 7, Ink::Opaque),
    ([318, 239, 406, 256], 8, Ink::Opaque),
];
/// The box of the halt pill of the world map.
pub const HALT_BOX: Box4 = [168, 406, 210, 427];
/// The box of the small "TM" mark of the logo.
pub const TM_BOX: Box4 = [450, 63, 462, 70];
/// The box of the letter in the round A button symbol of the world map.
pub const BUBBLE_A: Box4 = [139, 490, 151, 500];
/// The words of the tutorial buttons, as a box, an index into the tutorial words and the ink.
pub const TUTORIAL: [(Box4, usize, Ink); 11] = [
    ([394, 8, 428, 28], 0, Ink::Light),
    ([394, 49, 430, 69], 0, Ink::Dark),
    ([394, 88, 428, 108], 0, Ink::Light),
    ([4, 195, 76, 207], 0, Ink::Light),
    ([4, 235, 76, 248], 0, Ink::Dark),
    ([4, 183, 76, 195], 1, Ink::Dark),
    ([4, 224, 76, 236], 1, Ink::Dark),
    ([312, 102, 340, 117], 2, Ink::Dark),
    ([352, 102, 380, 117], 3, Ink::Dark),
    ([312, 174, 340, 189], 2, Ink::Dark),
    ([352, 174, 380, 189], 3, Ink::Dark),
];
/// The button symbols of the system picture, as a box and the letter in the symbol.
pub const SYSTEM_ICONS: [(Box4, &str); 7] = [
    ([139, 2, 149, 14], "A"),
    ([155, 2, 165, 14], "B"),
    ([171, 2, 181, 14], "X"),
    ([187, 2, 197, 14], "Y"),
    ([91, 378, 102, 391], "L"),
    ([107, 378, 118, 391], "R"),
    ([125, 378, 146, 391], "START"),
];
/// The words of the battle forecast, as a box, an index into the forecast words, the ink and the look.
pub const FORECAST: [(Box4, usize, Ink, Look); 23] = [
    ([66, 58, 119, 72], 0, Ink::Light, Look::Flat),
    ([193, 114, 245, 128], 0, Ink::Light, Look::Flat),
    ([124, 77, 150, 95], 1, Ink::Dark, Look::Flat),
    ([57, 133, 86, 152], 1, Ink::Dark, Look::Flat),
    ([64, 166, 92, 184], 1, Ink::Dark, Look::Flat),
    ([101, 166, 129, 184], 1, Ink::Dark, Look::Flat),
    ([63, 186, 93, 201], 2, Ink::Light, Look::Flat),
    ([101, 186, 131, 201], 2, Ink::Light, Look::Flat),
    ([63, 202, 93, 217], 3, Ink::Light, Look::Flat),
    ([101, 202, 131, 217], 3, Ink::Light, Look::Flat),
    ([63, 218, 93, 233], 4, Ink::Light, Look::Flat),
    ([101, 218, 128, 233], 4, Ink::Light, Look::Flat),
    ([219, 314, 246, 329], 2, Ink::Light, Look::Flat),
    ([219, 330, 246, 345], 3, Ink::Light, Look::Flat),
    ([219, 346, 246, 361], 4, Ink::Light, Look::Flat),
    ([63, 495, 92, 511], 5, Ink::Light, Look::Flat),
    ([24, 458, 56, 470], 6, Ink::Opaque, Look::GradedOutline),
    ([22, 473, 56, 486], 7, Ink::Opaque, Look::GradedOutline),
    ([0, 491, 40, 503], 8, Ink::Opaque, Look::GradedOutline),
    ([41, 297, 72, 312], 9, Ink::Opaque, Look::GradedOutline),
    ([39, 489, 57, 503], 10, Ink::Opaque, Look::GradedOutline),
    ([52, 278, 71, 289], 11, Ink::Light, Look::Flat),
    ([92, 278, 111, 289], 12, Ink::Light, Look::Flat),
];
/// The words of the unit screen, as a box, an index into the unit words, the ink, the look, the alignment and the slant.
pub const UNIT: [(Box4, usize, Ink, Look, Align, f32); 10] = [
    ([0, 192, 63, 208], 0, Ink::Opaque, Look::Flat, Align::Centre, 0.0),
    ([0, 208, 63, 224], 1, Ink::Opaque, Look::Flat, Align::Centre, 0.0),
    ([0, 224, 63, 240], 2, Ink::Opaque, Look::Flat, Align::Centre, 0.0),
    ([0, 240, 63, 256], 3, Ink::Opaque, Look::Flat, Align::Centre, 0.0),
    ([63, 192, 129, 208], 4, Ink::Opaque, Look::Flat, Align::Centre, 0.0),
    ([129, 193, 158, 208], 5, Ink::Opaque, Look::GradedShadow, Align::Left, 0.0),
    ([176, 194, 210, 206], 6, Ink::Light, Look::Flat, Align::Centre, 0.0),
    ([96, 209, 121, 220], 7, Ink::Opaque, Look::Flat, Align::Left, 0.0),
    ([96, 220, 121, 231], 8, Ink::Opaque, Look::Flat, Align::Left, 0.0),
    ([150, 233, 222, 255], 9, Ink::Light, Look::Graded, Align::Centre, 0.2),
];
/// The stat names around the radar chart, as a box, an index into the radar words and the alignment.
pub const RADAR: [(Box4, usize, Align); 6] = [
    ([394, 40, 436, 56], 0, Align::Centre),
    ([330, 70, 368, 89], 1, Align::Right),
    ([463, 70, 500, 88], 2, Align::Left),
    ([330, 133, 369, 151], 3, Align::Right),
    ([462, 133, 500, 151], 4, Align::Left),
    ([394, 169, 436, 188], 5, Align::Centre),
];
/// The words on the cards of the file menu, as a box, an index into the card words and the alignment.
pub const CARDS: [(Box4, usize, Align); 14] = [
    ([9, 104, 98, 121], 0, Align::Left),
    ([9, 136, 98, 153], 1, Align::Left),
    ([9, 168, 98, 185], 2, Align::Left),
    ([9, 200, 98, 217], 3, Align::Left),
    ([136, 8, 246, 26], 0, Align::Centre),
    ([136, 40, 246, 58], 1, Align::Centre),
    ([136, 72, 246, 90], 2, Align::Centre),
    ([136, 104, 246, 122], 3, Align::Centre),
    ([136, 136, 246, 154], 0, Align::Centre),
    ([136, 168, 246, 186], 1, Align::Centre),
    ([136, 200, 246, 218], 2, Align::Centre),
    ([136, 232, 246, 250], 3, Align::Centre),
    ([9, 73, 33, 87], 4, Align::Centre),
    ([49, 73, 74, 87], 5, Align::Centre),
];
/// The tone of the soft text of the plain words, as the share and the grey level that the banners module takes.
pub const SOFT: banners::Tone = banners::Tone { share: 1.0, grey: 0.92 };
/// The smallest difference between the lowest and the highest value around a pixel that counts as an edge when art is sharpened.
const EDGE_CONTRAST: f32 = 0.12;
/// The margin in enlarged pixels that is kept free on each side of a word in its box.
const FIT_MARGIN: f32 = 4.0;
/// The smallest difference of luminance between the letters and the rest of a box for the letters to be found.
const SPLIT_CONTRAST: f32 = 0.15;
/// The fraction of the pixels of a box, as one in this number, whose colors are taken as the color of the letters.
const LETTER_SHARE: usize = 10;
/// The fraction of a row or column that must be covered by letters for it to count as a line of the frame.
const LINE_SHARE: f32 = 0.8;
/// The blur of the logo before its edges are sharpened, in pixels of the original picture.
const LOGO_SMOOTH: f32 = 0.4;
/// The levels of the blurred logo that map to black and to white.
const LOGO_EDGE: (f32, f32) = (0.22, 0.62);
/// The first and the last row of the "TM" mark in the logo.
const TM_ROWS: (usize, usize) = (64, 69);
/// The narrowest that a word can be squeezed horizontally, as a fraction of its natural width.
const CONDENSED: f32 = 0.7;
/// The number of passes of the smoothing of the calm filter.
const CALM_PASSES: usize = 2;
/// The reach of the calm filter in pixels.
const CALM_REACH: usize = 2;
/// The spatial spread of the calm filter in pixels.
const CALM_SPACE: f32 = 1.5;
/// The color spread of the calm filter, which keeps real edges from being blurred.
const CALM_RANGE: f32 = 0.1;
/// The spread in pixels of the weights that fill an erased word from its surroundings.
const FILL_SIGMA: f32 = 3.0;
/// The shadow of a word: the offset, the blur and the opacity, with the offset and the blur in pixels of the original picture.
const SHADOW: (f32, f32, f32) = (0.5, 0.15, 0.85);
/// The weakest letter coverage that counts as a trace of a letter when a word is erased.
const FAINT: f32 = 0.05;
/// The width of the outline of a word in pixels of the original picture.
const OUTLINE: f32 = 0.75;
/// The fraction of the pixels near a word, as one in this number, that gives the dark color of its shadow or outline.
const DARK_SHARE: usize = 6;
/// The number of samples per side of an enlarged pixel in the faithful enlargement.
const FAITHFUL_SAMPLES: usize = 2;
/// The largest difference per color channel for two pixels to count as the same color.
const ALIKE: f32 = 0.1;
/// The alpha below which a pixel counts as clear.
const CLEAR: f32 = 0.02;
/// The reach in pixels of the original picture that is used to find the peak around a stroke.
const STROKE_REACH: f32 = 1.0;
/// The lowest peak value that is used when a stroke is normalized.
const STROKE_FLOOR: f32 = 0.05;
/// The normalized levels of a stroke that map to nothing and to full strength.
const STROKE_EDGE: (f32, f32) = (0.3, 0.65);

/// Marks the pixels of the given boxes that have ink, with a margin of one pixel around the ink.
fn kept(core: &Plane, keep: &[Box4]) -> Mask {
    let inked = dilate(&Mask::from(core, |v| v > 0.1), 1);
    let mut mask = Mask::new(core.w, core.h);
    for k in keep {
        for y in k[1]..k[3].min(core.h) {
            for x in k[0]..k[2].min(core.w) {
                mask.set(x, y, inked.at(x, y));
            }
        }
    }
    mask
}

/// Enlarges a picture to four times the size and keeps its edges crisp.
/// Each channel is enlarged smoothly.
/// Then the values near an edge are pushed toward the lowest or the highest value of the pixels around.
pub fn sharp_art(image: &Rgba) -> Rgba {
    let (w, h) = (image.w, image.h);
    let alpha = image.channel(3);
    // The color channels are multiplied by the alpha, so that clear pixels do not bleed their color into the edges.
    let channels = [0, 1, 2, 3].map(|k| if k == 3 { alpha.clone() } else { image.channel(k).zip(&alpha, |v, a| v * a) });
    let shaped = channels.map(|p| {
        let up = resize(&p, w * SCALE, h * SCALE, Filter::Bicubic);
        // This is the lowest or the highest value in the three by three pixels around each pixel, enlarged.
        let around = |pick: fn(f32, f32) -> f32, start: f32| {
            let mut out = Plane::new(w, h);
            for y in 0..h {
                for x in 0..w {
                    let mut v = start;
                    for yy in y.saturating_sub(1)..(y + 2).min(h) {
                        for xx in x.saturating_sub(1)..(x + 2).min(w) {
                            v = pick(v, p.at(xx, yy));
                        }
                    }
                    out.set(x, y, v);
                }
            }
            resize(&out, w * SCALE, h * SCALE, Filter::Bilinear)
        };
        let (lo, hi) = (around(f32::min, 1.0), around(f32::max, 0.0));
        Plane {
            w: up.w,
            h: up.h,
            v: (0..up.v.len())
                .map(|i| {
                    let (u, l, r) = (up.v[i], lo.v[i], hi.v[i] - lo.v[i]);
                    if r <= EDGE_CONTRAST {
                        return u.clamp(0.0, 1.0);
                    }
                    // Where there is an edge, the position between the low and the high value is steepened with two smoothstep curves.
                    let mut t = ((u - l) / r).clamp(0.0, 1.0);
                    for _ in 0..2 {
                        t = t * t * (3.0 - 2.0 * t);
                    }
                    l + r * t
                })
                .collect(),
        }
    });
    // Undo the multiplication by the alpha.
    let mut out = Rgba::new(w * SCALE, h * SCALE);
    for (i, p) in out.v.iter_mut().enumerate() {
        let a = shaped[3].v[i].clamp(0.0, 1.0);
        *p = if a > 1e-4 { [(shaped[0].v[i] / a).clamp(0.0, 1.0), (shaped[1].v[i] / a).clamp(0.0, 1.0), (shaped[2].v[i] / a).clamp(0.0, 1.0), a] } else { [0.0; 4] };
    }
    out
}

/// Multiplies the color of a pixel by its alpha.
fn premultiplied(p: [f32; 4]) -> [f32; 4] {
    [p[0] * p[3], p[1] * p[3], p[2] * p[3], p[3]]
}

/// Divides the color of a pixel by its alpha again, and gives a fully clear pixel for an alpha that is nearly zero.
fn unpremultiplied(p: [f32; 4]) -> [f32; 4] {
    let a = p[3].clamp(0.0, 1.0);
    if a > 1e-4 {
        [p[0] / a, p[1] / a, p[2] / a, a]
    } else {
        [0.0; 4]
    }
}

/// Smooths the noise in a picture without blurring its edges.
/// It is a bilateral filter, where the weight of a neighbor falls with both its distance and its difference in color.
pub fn calm(image: &Rgba) -> Rgba {
    let (w, h) = (image.w, image.h);
    let mut pixels: Vec<[f32; 4]> = image.v.iter().map(|&p| premultiplied(p)).collect();
    for _ in 0..CALM_PASSES {
        pixels = (0..w * h)
            .map(|i| {
                let (x, y, p) = (i % w, i / w, pixels[i]);
                let (mut sum, mut weight) = ([0.0; 4], 0.0);
                for yy in y.saturating_sub(CALM_REACH)..(y + CALM_REACH + 1).min(h) {
                    for xx in x.saturating_sub(CALM_REACH)..(x + CALM_REACH + 1).min(w) {
                        let q = pixels[yy * w + xx];
                        let (dx, dy) = (xx as f32 - x as f32, yy as f32 - y as f32);
                        let near: f32 = (0..4).map(|c| (q[c] - p[c]).powi(2)).sum();
                        let k = (-(dx * dx + dy * dy) / (2.0 * CALM_SPACE * CALM_SPACE) - near / (2.0 * CALM_RANGE * CALM_RANGE)).exp();
                        (0..4).for_each(|c| sum[c] += k * q[c]);
                        weight += k;
                    }
                }
                sum.map(|v| v / weight)
            })
            .collect();
    }
    Rgba { w, h, v: pixels.into_iter().map(unpremultiplied).collect() }
}

/// Returns the median of each of the four channels of the samples.
fn median4(mut samples: Vec<[f32; 4]>) -> [f32; 4] {
    [0, 1, 2, 3].map(|k| {
        samples.sort_by(|a, b| a[k].total_cmp(&b[k]));
        samples[samples.len() / 2][k]
    })
}

/// The sum of the differences of the four channels of two pixels.
fn gap(a: [f32; 4], b: [f32; 4]) -> f32 {
    (0..4).map(|k| (a[k] - b[k]).abs()).sum()
}

/// The letters of a box, told apart from its background.
struct Split {
    /// The color of the letters, with the color multiplied by the alpha.
    letter: [f32; 4],
    /// The color of the background in each row of the box.
    rows: Vec<[f32; 4]>,
    /// How much each pixel is covered by letters, from 0 to 1.
    mask: Plane,
    /// The pixels that show at least a trace of the letters.
    faint: Mask,
}

/// What the letters of a word look like against their background.
#[derive(Clone, Copy, PartialEq)]
pub enum Ink {
    /// The letters are lighter than the background.
    Light,
    /// The letters are darker than the background.
    Dark,
    /// The letters are the opaque part of the picture, and the rest is clear.
    Opaque,
}

/// How a word is painted.
#[derive(Clone, Copy, PartialEq)]
pub enum Look {
    /// One color.
    Flat,
    /// A color that changes from the top row of the word to the bottom row.
    Graded,
    /// One color with a drop shadow.
    Shadow,
    /// A changing color with a drop shadow.
    GradedShadow,
    /// A changing color with an outline.
    GradedOutline,
}

impl Look {
    /// Tells whether the color changes from row to row.
    fn graded(self) -> bool {
        matches!(self, Look::Graded | Look::GradedShadow | Look::GradedOutline)
    }
}

/// Where a word sits in its box.
#[derive(Clone, Copy, PartialEq)]
pub enum Align {
    Centre,
    /// The word starts where the old letters started.
    Left,
    /// The word ends where the old letters ended.
    Right,
}

/// A word to draw in a box of a picture.
#[derive(Clone, Copy)]
pub struct Word {
    /// The box that holds the old word.
    pub at: Box4,
    /// The new text.
    pub text: &'static str,
    pub ink: Ink,
    pub look: Look,
    pub align: Align,
    /// The slant of the letters, where 0 is upright.
    pub slant: f32,
    /// The share of the height of the old word that the new one takes, 1 for all of it.
    pub size: f32,
}

impl Word {
    /// Creates a flat, centered and upright word.
    pub fn flat(at: Box4, text: &'static str, ink: Ink) -> Word {
        Word { at, text, ink, look: Look::Flat, align: Align::Centre, slant: 0.0, size: 1.0 }
    }
}

/// The colors that a word is painted with.
struct Paint {
    look: Look,
    /// The color of the letters for each row of the word, from top to bottom.
    rows: Vec<[f32; 3]>,
    /// The color of the shadow or the outline.
    dark: [f32; 3],
}

/// Finds the letters in a box.
/// The pixels are ranked by luminance, or by alpha for opaque ink.
/// The most extreme tenth gives the color of the letters.
/// The background is estimated for each row.
/// The coverage of each pixel is its position between the background and the letter color.
/// Returns `None` when the box has too little contrast.
fn split(region: &Rgba, ink: Ink) -> Option<Split> {
    let (w, h) = (region.w, region.h);
    let pixels: Vec<[f32; 4]> = region.v.iter().map(|&p| premultiplied(p)).collect();
    // The luminance uses the Rec. 601 weights.
    let lum: Vec<f32> = pixels.iter().map(|p| if ink == Ink::Opaque { p[3] } else { p[0] * 0.299 + p[1] * 0.587 + p[2] * 0.114 }).collect();
    let mut order: Vec<usize> = (0..pixels.len()).collect();
    order.sort_by(|&a, &b| lum[a].total_cmp(&lum[b]));
    // After this, the pixels that look most like letters come first.
    if ink != Ink::Dark {
        order.reverse();
    }
    let extreme = order.len() / LETTER_SHARE;
    if extreme < 5 || (lum[order[0]] - lum[order[order.len() - 1]]).abs() < SPLIT_CONTRAST {
        return None;
    }
    let letter = median4(order[..extreme].iter().map(|&i| pixels[i]).collect());
    // For opaque ink, the alpha is the coverage, with a soft ramp between 0.15 and 0.65.
    if ink == Ink::Opaque {
        let mask = Plane { w, h, v: pixels.iter().map(|p| ((p[3] - 0.15) / 0.5).clamp(0.0, 1.0)).collect() };
        let faint = Mask { w, h, v: pixels.iter().map(|p| p[3] > CLEAR).collect() };
        return Some(Split { letter, rows: vec![[0.0; 4]; h], mask, faint });
    }
    // The reach is half of the distance from the letter color to the median of the half of the pixels that are farthest from it.
    // Pixels beyond the reach count as background.
    let mut far: Vec<(f32, [f32; 4])> = pixels.iter().map(|&p| (gap(p, letter), p)).collect();
    far.sort_by(|a, b| b.0.total_cmp(&a.0));
    let reach = gap(letter, median4(far.iter().take(far.len().div_ceil(2)).map(|&(_, p)| p).collect())) * 0.5;
    let found: Vec<Option<[f32; 4]>> = (0..h)
        .map(|y| {
            let plain: Vec<[f32; 4]> = pixels[y * w..(y + 1) * w].iter().copied().filter(|&p| gap(p, letter) > reach).collect();
            (plain.len() >= 3).then(|| median4(plain))
        })
        .collect();
    // A row with too few background pixels takes the median of its half that is farthest from the letter color.
    let rows: Vec<[f32; 4]> = (0..h)
        .map(|y| {
            found[y].unwrap_or_else(|| {
                let mut far: Vec<(f32, [f32; 4])> = pixels[y * w..(y + 1) * w].iter().map(|&p| (gap(p, letter), p)).collect();
                far.sort_by(|a, b| b.0.total_cmp(&a.0));
                median4(far.iter().take(w.div_ceil(2)).map(|&(_, p)| p).collect())
            })
        })
        .collect();
    let mut mask = Plane::new(w, h);
    let mut faint = Mask::new(w, h);
    for y in 0..h {
        // The coverage is the projection of the pixel on the line from the background to the letter color.
        let bg = rows[y];
        let span: Vec<f32> = (0..4).map(|k| letter[k] - bg[k]).collect();
        let length: f32 = span.iter().map(|v| v * v).sum::<f32>().max(1e-4);
        for x in 0..w {
            let p = pixels[y * w + x];
            let t = (0..4).map(|k| (p[k] - bg[k]) * span[k]).sum::<f32>() / length;
            mask.set(x, y, ((t - 0.15) / 0.5).clamp(0.0, 1.0));
            faint.set(x, y, t.abs() > FAINT);
        }
    }
    // Rows and columns that are almost entirely covered are lines of the frame, not letters, so they are removed from the mask.
    let lines_x: Vec<usize> = (0..w).filter(|&x| (0..h).filter(|&y| mask.at(x, y) > 0.5).count() as f32 >= LINE_SHARE * h as f32).collect();
    let lines_y: Vec<usize> = (0..h).filter(|&y| (0..w).filter(|&x| mask.at(x, y) > 0.5).count() as f32 >= LINE_SHARE * w as f32).collect();
    for x in lines_x {
        (0..h).for_each(|y| mask.set(x, y, 0.0));
    }
    for y in lines_y {
        (0..w).for_each(|x| mask.set(x, y, 0.0));
    }
    Some(Split { letter, rows, mask, faint })
}

/// Erases the marked pixels of a box by filling them from the pixels around that are not letters.
/// The fill is a Gaussian weighted mean.
/// A pixel with no known neighbors takes the background color of its row.
fn fill(region: &Rgba, found: &Split, erase: &Mask) -> Rgba {
    let (w, h) = (region.w, region.h);
    let pixels: Vec<[f32; 4]> = region.v.iter().map(|&p| premultiplied(p)).collect();
    let known: Vec<bool> = (0..w * h).map(|i| !erase.v[i] && found.mask.v[i] == 0.0).collect();
    let reach = (FILL_SIGMA * 3.0).ceil() as usize;
    let smooth: Vec<Option<[f32; 4]>> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let (mut sum, mut weight) = ([0.0; 4], 0.0);
            for yy in y.saturating_sub(reach)..(y + reach + 1).min(h) {
                for xx in x.saturating_sub(reach)..(x + reach + 1).min(w) {
                    let j = yy * w + xx;
                    if known[j] {
                        let (dx, dy) = (xx as f32 - x as f32, yy as f32 - y as f32);
                        let k = (-(dx * dx + dy * dy) / (2.0 * FILL_SIGMA * FILL_SIGMA)).exp();
                        (0..4).for_each(|c| sum[c] += k * pixels[j][c]);
                        weight += k;
                    }
                }
            }
            (weight > 1e-3).then(|| sum.map(|v| v / weight))
        })
        .collect();
    let mut out = region.clone();
    for i in (0..w * h).filter(|&i| erase.v[i]) {
        out.v[i] = unpremultiplied(smooth[i].unwrap_or(found.rows[i / w]));
    }
    out
}

/// Draws a word as a plane of coverage with the given height, squeezed horizontally if needed to fit the room.
/// The room is the width and the height that are available, and the squeeze never goes below the condensed limit.
fn word(face: &Face, text: &str, height: f32, room: (f32, f32)) -> Option<Plane> {
    // The line is drawn large and then reduced, which gives smooth edges.
    let line = face.sized(40.0 * SS as f32).line(text, 0.0)?;
    let tall = (height / line.h as f32).min(room.1 / line.h as f32);
    let squeeze = (room.0 / (line.w as f32 * tall)).clamp(CONDENSED, 1.0);
    let tall = tall.min(room.0 / (line.w as f32 * squeeze));
    Some(resize(&line, ((line.w as f32 * tall * squeeze).round() as usize).max(1), ((line.h as f32 * tall).round() as usize).max(1), Filter::Lanczos).map(|v| v.clamp(0.0, 1.0)))
}

/// Blends a layer of coverage onto the picture at an offset, with a color that can change from row to row.
fn over(out: &mut Rgba, layer: &Plane, (ox, oy): (isize, isize), colour: impl Fn(usize) -> [f32; 3]) {
    for y in 0..layer.h {
        let ty = oy + y as isize;
        if ty < 0 || ty >= out.h as isize {
            continue;
        }
        let c = colour(y);
        for x in 0..layer.w {
            let tx = ox + x as isize;
            let m = layer.at(x, y).clamp(0.0, 1.0);
            if m <= 0.0 || tx < 0 || tx >= out.w as isize {
                continue;
            }
            let p = &mut out.v[ty as usize * out.w + tx as usize];
            let under = premultiplied(*p);
            *p = unpremultiplied([0, 1, 2, 3].map(|k| if k == 3 { m } else { c[k].clamp(0.0, 1.0) * m } + under[k] * (1.0 - m)));
        }
    }
}

/// Returns a copy of the plane with clear space around it.
fn padded(p: &Plane, pad: usize) -> Plane {
    let mut out = Plane::new(p.w + 2 * pad, p.h + 2 * pad);
    out.paste(p, pad as isize, pad as isize);
    out
}

/// Grows the shapes of a plane by a radius in pixels, with a soft edge of half a pixel.
fn swell(p: &Plane, radius: f32) -> Plane {
    let r = radius.ceil() as isize;
    let mut out = Plane::new(p.w, p.h);
    for y in 0..p.h as isize {
        for x in 0..p.w as isize {
            let mut most = 0.0f32;
            for dy in -r..=r {
                for dx in -r..=r {
                    let reach = ((dx * dx + dy * dy) as f32).sqrt();
                    let (sx, sy) = (x + dx, y + dy);
                    if reach > radius + 0.5 || sx < 0 || sy < 0 || sx >= p.w as isize || sy >= p.h as isize {
                        continue;
                    }
                    most = most.max(p.at(sx as usize, sy as usize) * (radius + 0.5 - reach).min(1.0));
                }
            }
            out.set(x as usize, y as usize, most);
        }
    }
    out
}

/// Paints a word onto the picture at a position, with its shadow or outline below the letters.
fn paint(out: &mut Rgba, ink: &Plane, (x, y): (usize, usize), paint: &Paint) {
    // The padding leaves room for the shadow and the outline around the word.
    let pad = (SHADOW.0 + 3.0 * SHADOW.1 + OUTLINE).ceil() as usize * SCALE;
    let big = padded(ink, pad);
    let at = (x as isize - pad as isize, y as isize - pad as isize);
    let rows = paint.rows.len();
    // The color of a row of the padded word is the color of the matching row of the original colors.
    let tint = |row: usize| paint.rows[(row.saturating_sub(pad) * rows / ink.h.max(1)).min(rows - 1)];
    match paint.look {
        Look::Flat | Look::Graded => {}
        Look::Shadow | Look::GradedShadow => {
            let step = (SHADOW.0 * SCALE as f32).round() as isize;
            let mut moved = Plane::new(big.w, big.h);
            moved.paste(&big, step, step);
            let shadow = gaussian(&moved, SHADOW.1 * SCALE as f32).map(|v| (v * SHADOW.2).clamp(0.0, 1.0));
            over(out, &shadow, at, |_| paint.dark);
        }
        Look::GradedOutline => over(out, &swell(&big, OUTLINE * SCALE as f32), at, |_| paint.dark),
    }
    over(out, &big, at, tint);
}

/// Works out the colors to paint a word with from the old word.
/// The dark color comes from the darkest pixels around the letters.
/// A graded look takes the color of the letters row by row.
fn colours(region: &Rgba, found: &Split, erase: &Mask, (ink, look): (Ink, Look), (y0, y1): (usize, usize)) -> Paint {
    let plain = |p: [f32; 4]| [p[0], p[1], p[2]];
    let lum = |c: [f32; 3]| c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
    let mut near: Vec<[f32; 4]> = (0..region.w * region.h).filter(|&i| erase.v[i] && region.v[i][3] > 0.5).map(|i| region.v[i]).collect();
    near.sort_by(|p, q| lum(plain(*p)).total_cmp(&lum(plain(*q))));
    let share = near.len().div_ceil(DARK_SHARE).max(1);
    let dark = if near.is_empty() { [0.0; 3] } else { plain(median4(near[..share].to_vec())) };
    let a = found.letter[3].max(1e-4);
    let letter = match ink {
        Ink::Opaque if !near.is_empty() => plain(median4(near[near.len() - share..].to_vec())),
        _ => [found.letter[0] / a, found.letter[1] / a, found.letter[2] / a],
    };
    if !look.graded() {
        return Paint { look, rows: vec![letter], dark };
    }
    // A pixel counts as bright when it is on the side of the middle luminance that the letters are on.
    let middle = (lum(dark) + lum(letter)) / 2.0;
    let bright = |p: [f32; 4]| p[3] > 0.5 && (lum(plain(p)) - middle) * (lum(letter) - middle) > 0.0;
    let lit: Vec<usize> = (y0..y1).filter(|&y| (0..region.w).any(|x| found.mask.at(x, y) > 0.5 && bright(region.at(x, y)))).collect();
    let rows = match (lit.first(), lit.last()) {
        (Some(&top), Some(&bottom)) => (top..=bottom)
            .map(|y| {
                let picked: Vec<[f32; 4]> = (0..region.w).map(|x| region.at(x, y)).filter(|&p| bright(p)).collect();
                if picked.is_empty() { letter } else { plain(median4(picked)) }
            })
            .collect(),
        _ => vec![letter],
    };
    Paint { look, rows, dark }
}

/// Redraws the words of a picture and enlarges the picture to four times the size.
/// The old letters are erased.
/// The picture is enlarged with the given function.
/// The new words are painted on the enlarged picture.
/// A word whose letters cannot be found is left as it was.
pub fn buttons(image: &Rgba, words: &[Word], face: &Face, enlarge: fn(&Rgba) -> Rgba) -> Rgba {
    let mut cleaned = image.clone();
    let mut drawn = Vec::new();
    for w in words {
        let b = w.at;
        let region = image.crop(b[0], b[1], b[2], b[3]);
        let Some(found) = split(&region, w.ink) else { continue };
        // The rows with at least two letter pixels give the top and the bottom of the old word.
        let inked: Vec<usize> = (0..region.h).filter(|&y| (0..region.w).filter(|&x| found.mask.at(x, y) > 0.5).count() >= 2).collect();
        let (Some(&y0), Some(&last)) = (inked.first(), inked.last()) else { continue };
        let y1 = last + 1;
        // A shadow of a word with light or dark ink extends further, so more pixels around the letters are erased.
        let reach = if matches!(w.look, Look::Shadow | Look::GradedShadow) && w.ink != Ink::Opaque { 3 } else { 1 };
        let core = Mask::from(&found.mask, |v| v > 0.5);
        let touched = Mask::from(&found.mask, |v| v > 0.05).or(&found.faint);
        let erase = dilate(&core, reach).or(&dilate(&core, reach + 1).and(&touched));
        cleaned.paste(&fill(&region, &found, &erase), b[0], b[1]);
        let colour = colours(&region, &found, &erase, (w.ink, w.look), (y0, y1));
        // The shadow or the outline of opaque ink is part of the old word, so its size is taken off the height of the new letters.
        let (above, below) = match (w.ink, w.look) {
            (Ink::Opaque, Look::Shadow | Look::GradedShadow) => (0.0, SHADOW.0),
            (Ink::Opaque, Look::GradedOutline) => (OUTLINE, OUTLINE),
            _ => (0.0, 0.0),
        };
        let room = (((b[2] - b[0]) * SCALE) as f32 - 2.0 * FIT_MARGIN, ((b[3] - b[1]) * SCALE) as f32 - 2.0 * FIT_MARGIN);
        let face = if w.slant > 0.0 { face.slanted(w.slant) } else { face.clone() };
        let Some(ink) = word(&face, w.text, ((y1 - y0) as f32 - above - below) * SCALE as f32 * w.size, room) else { continue };
        let x = match w.align {
            Align::Centre => b[0] * SCALE + ((b[2] - b[0]) * SCALE - ink.w) / 2,
            Align::Left => {
                let first = (0..region.w).find(|&x| (y0..y1).any(|y| found.mask.at(x, y) > 0.5)).unwrap_or(0);
                ((b[0] + first) * SCALE).min(b[2] * SCALE - ink.w)
            }
            Align::Right => {
                let last = (0..region.w).rev().find(|&x| (y0..y1).any(|y| found.mask.at(x, y) > 0.5)).unwrap_or(region.w - 1);
                ((b[0] + last + 1) * SCALE).saturating_sub(ink.w).max(b[0] * SCALE)
            }
        };
        // The new word is centered vertically on the old one.
        let centre = (b[1] as f32 + (y0 as f32 + above + y1 as f32 - below) / 2.0) * SCALE as f32;
        let y = ((centre - ink.h as f32 / 2.0).round() as usize).clamp(b[1] * SCALE, (b[3] * SCALE).saturating_sub(ink.h));
        drawn.push((x, y, ink, colour));
    }
    let mut out = enlarge(&cleaned);
    for (x, y, ink, colour) in drawn {
        paint(&mut out, &ink, (x, y), &colour);
    }
    out
}

/// Tells whether two pixels have the same color within a tolerance, or are both clear.
fn alike(a: [f32; 4], b: [f32; 4]) -> bool {
    if a[3] < CLEAR || b[3] < CLEAR {
        return a[3] < CLEAR && b[3] < CLEAR;
    }
    (0..3).all(|k| (a[k] - b[k]).abs() <= ALIKE)
}

/// The boxes of the defense, avoid and healing icons of the terrain panel, in the unit picture.
/// These icons are pixel art, so they keep their square pixels.
pub const TERRAIN_ICONS: [Box4; 3] = [[14, 26, 22, 36], [32, 26, 41, 36], [54, 27, 63, 35]];

/// Enlarges the unit picture like faithful_art, and the icons of the terrain panel with square pixels.
pub fn unit_art(image: &Rgba) -> Rgba {
    let mut out = faithful_art(image);
    for b in TERRAIN_ICONS {
        out.paste(&square_pixels(&image.crop(b[0], b[1], b[2], b[3])), b[0] * SCALE, b[1] * SCALE);
    }
    out
}

/// Enlarges a picture to four times the size.
/// Each pixel becomes a square of the same color.
fn square_pixels(image: &Rgba) -> Rgba {
    let mut out = Rgba::new(image.w * SCALE, image.h * SCALE);
    let wide = out.w;
    for (i, p) in out.v.iter_mut().enumerate() {
        *p = image.v[(i / wide / SCALE) * image.w + i % wide / SCALE];
    }
    out
}

/// Enlarges a picture to four times the size without inventing colors.
/// Each sample blends only the neighboring pixels that have the same color as the pixel that fits best.
/// As a result, flat areas stay flat and edges stay sharp.
pub fn faithful_art(image: &Rgba) -> Rgba {
    let (w, h) = (image.w, image.h);
    let pre: Vec<[f32; 4]> = image.v.iter().map(|&p| premultiplied(p)).collect();
    let plain: Vec<[f32; 4]> = pre.iter().map(|&p| unpremultiplied(p)).collect();
    let samples = SCALE * FAITHFUL_SAMPLES;
    let mut out = Rgba::new(w * SCALE, h * SCALE);
    for oy in 0..h * SCALE {
        for ox in 0..w * SCALE {
            let mut sum = [0.0f32; 4];
            for sy in 0..FAITHFUL_SAMPLES {
                for sx in 0..FAITHFUL_SAMPLES {
                    // These are the position of the sample in the original picture, and the four pixels around it with their bilinear weights.
                    let u = ((ox * FAITHFUL_SAMPLES + sx) as f32 + 0.5) / samples as f32;
                    let v = ((oy * FAITHFUL_SAMPLES + sy) as f32 + 0.5) / samples as f32;
                    let own = (v as usize).min(h - 1) * w + (u as usize).min(w - 1);
                    let (bx, by) = ((u - 0.5).floor(), (v - 0.5).floor());
                    let (tx, ty) = (u - 0.5 - bx, v - 0.5 - by);
                    let at = |dx: isize, dy: isize| (by as isize + dy).clamp(0, h as isize - 1) as usize * w + (bx as isize + dx).clamp(0, w as isize - 1) as usize;
                    let corners = [(at(0, 0), (1.0 - tx) * (1.0 - ty)), (at(1, 0), tx * (1.0 - ty)), (at(0, 1), (1.0 - tx) * ty), (at(1, 1), tx * ty)];
                    let share = |i: usize| corners.iter().filter(|&&(j, _)| alike(plain[j], plain[i])).map(|&(_, weight)| weight).sum::<f32>();
                    // The best pixel is the one whose color is shared by the largest weight of the corners, and the pixel that holds the sample wins a tie.
                    let (mut best, mut most) = (own, share(own));
                    for &(j, _) in &corners {
                        let s = share(j);
                        if s > most + 1e-4 {
                            (best, most) = (j, s);
                        }
                    }
                    for &(j, weight) in &corners {
                        if alike(plain[j], plain[best]) {
                            (0..4).for_each(|k| sum[k] += pre[j][k] * weight / most.max(1e-6));
                        }
                    }
                }
            }
            out.v[oy * out.w + ox] = unpremultiplied(sum.map(|v| v / (FAITHFUL_SAMPLES * FAITHFUL_SAMPLES) as f32));
        }
    }
    out
}

/// Enlarges a picture and redraws the strokes of its core as smooth lines in the light color of the strokes.
/// It is used for the parts of a picture that are kept and not redrawn with the new font.
fn strokes(image: &Rgba, core: &Plane) -> Rgba {
    let mut out = super::labels::enlarge(image);
    let up = resize(core, core.w * SCALE, core.h * SCALE, Filter::Bicubic).map(|v| v.clamp(0.0, 1.0));
    // Each value is divided by the peak nearby, which evens out strokes of different strength, and then a smoothstep sharpens the edge.
    let peak = swell(&up, STROKE_REACH * SCALE as f32);
    let line = up.zip(&peak, |v, p| {
        let t = ((v / p.max(STROKE_FLOOR) - STROKE_EDGE.0) / (STROKE_EDGE.1 - STROKE_EDGE.0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t) * p
    });
    let lit: Vec<[f32; 4]> = (0..core.v.len()).filter(|&i| core.v[i] > 0.5).map(|i| image.v[i]).collect();
    if lit.is_empty() {
        return out;
    }
    let light = median4(lit);
    over(&mut out, &line, (0, 0), |_| [light[0], light[1], light[2]]);
    out
}

/// Redraws the text of a picture in the boxes of the given words, at four times the size.
/// The strokes inside the keep boxes are not redrawn but only enlarged.
/// The kind says how the letters are filled and what goes around them.
/// Returns `None` when a box has no letters or a word does not fit.
pub fn redraw(image: &Rgba, words: &[(Box4, &str)], keep: &[Box4], tone: banners::Tone, kind: banners::Kind, face: &Face) -> Option<Rgba> {
    let core = banners::core(image);
    let keep_mask = kept(&core, keep);
    // The text is the core of the letters without the parts that are kept.
    let text = core.zip(&keep_mask.as_plane(), |c, k| if k > 0.0 { 0.0 } else { c });
    let probe = face.sized(40.0 * SS as f32);
    let mut big = Plane::new(image.w * SCALE, image.h * SCALE);
    for &(b, words) in words {
        let (x0, x1, y0, y1) = text.crop(b[0], b[1], b[2], b[3]).bbox(0.5)?;
        let bounds = (b[0] + x0, b[0] + x1, b[1] + y0, b[1] + y1);
        let mask = probe.line(words, 0.0)?;
        let fit = search(&mask, &text, (b[1], b[3]), bounds, &TITLE)?;
        let canvas = place(&mask, &text, bounds, &fit, SCALE, TITLE.edge)?;
        for (v, c) in big.v.iter_mut().zip(&canvas.v) {
            *v = v.max(*c);
        }
    }
    // The kept parts are cleared before the redraw, and put back from the enlarged strokes afterwards.
    let mut cleared = image.clone();
    for (p, &k) in cleared.v.iter_mut().zip(&keep_mask.v) {
        if k {
            *p = [0.0; 4];
        }
    }
    let mut out = banners::redraw(&cleared, &text, kind, tone, &big);
    if keep_mask.any() {
        let enlarged = strokes(image, &core);
        for (i, p) in out.v.iter_mut().enumerate() {
            let (x, y) = (i % out.w / SCALE, i / out.w / SCALE);
            if keep_mask.at(x, y) {
                *p = enlarged.v[i];
            }
        }
    }
    Some(out)
}

/// Enlarges the logo of the title screen to four times the size as a gray picture with smooth edges, and redraws its "TM" mark in the new font.
pub fn logo(image: &Rgba, face: &Face) -> Rgba {
    // The luminance is enlarged to a finer scale, blurred, thresholded and then reduced, which gives smooth edges.
    let fine = SCALE * 4;
    let up = resize(&image.lum(), image.w * fine, image.h * fine, Filter::Bicubic);
    let level = gaussian(&up, LOGO_SMOOTH * fine as f32).map(|v| {
        let x = ((v - LOGO_EDGE.0) / (LOGO_EDGE.1 - LOGO_EDGE.0)).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    });
    let level = level.down(4);
    let mut out = Rgba::new(level.w, level.h);
    for (p, &l) in out.v.iter_mut().zip(&level.v) {
        *p = [l, l, l, 1.0];
    }
    // The old "TM" mark is painted black first.
    let b = TM_BOX;
    for y in b[1] * SCALE..b[3] * SCALE {
        out.v[y * out.w + b[0] * SCALE..y * out.w + b[2] * SCALE].fill([0.0, 0.0, 0.0, 1.0]);
    }
    let room = (((b[2] - b[0]) * SCALE) as f32, ((b[3] - b[1]) * SCALE) as f32);
    if let Some(ink) = word(face, "TM", ((TM_ROWS.1 - TM_ROWS.0) * SCALE) as f32, room) {
        let x = b[0] * SCALE + ((b[2] - b[0]) * SCALE).saturating_sub(ink.w) / 2;
        let y = ((TM_ROWS.0 + TM_ROWS.1) * SCALE / 2).saturating_sub(ink.h / 2);
        over(&mut out, &ink, (x as isize, y as isize), |_| [1.0; 3]);
    }
    out
}
