use super::banners;
use super::labels::enlarge;
use super::ops::{dilate, Mask, Plane, Rgba};
use super::text::Face;
use super::titles::{place, search, SCALE, SS, TITLE};

pub type Box4 = [usize; 4];

pub const PRESS_ANY_BUTTON_BOX: Box4 = [44, 0, 212, 32];
pub const PRESS_ANY_BUTTON_KEEP: [Box4; 2] = [[0, 0, 47, 32], [211, 0, 256, 32]];
pub const MENU_BOXES: [Box4; 8] = [[0, 0, 170, 24], [0, 24, 170, 50], [0, 50, 170, 73], [0, 73, 170, 97], [0, 97, 170, 126], [170, 26, 256, 40], [170, 67, 256, 84], [170, 88, 256, 124]];
pub const MENU_KEEP: [Box4; 2] = [[170, 0, 256, 26], [170, 40, 256, 67]];
pub const COPYRIGHT_BOX: Box4 = [0, 0, 512, 32];
const SOFT: banners::Tone = banners::Tone { share: 1.0, grey: 0.92 };

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

pub fn redraw(image: &Rgba, words: &[(Box4, &str)], keep: &[Box4], face: &Face) -> Option<Rgba> {
    let core = banners::core(image);
    let keep_mask = kept(&core, keep);
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
    let mut cleared = image.clone();
    for (p, &k) in cleared.v.iter_mut().zip(&keep_mask.v) {
        if k {
            *p = [0.0; 4];
        }
    }
    let mut out = banners::redraw(&cleared, &text, banners::Kind::Plain, SOFT, &big);
    if keep_mask.any() {
        let enlarged = enlarge(image);
        for (i, p) in out.v.iter_mut().enumerate() {
            let (x, y) = (i % out.w / SCALE, i / out.w / SCALE);
            if keep_mask.at(x, y) {
                *p = enlarged.v[i];
            }
        }
    }
    Some(out)
}
