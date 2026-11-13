//! presentation backends.

#[cfg(feature = "opengl")]
pub mod gl;
#[cfg(feature = "vulkan")]
pub mod vulkan;
#[cfg(feature = "vulkan")]
mod vulkan_overlay;

/// one screen's pixels, already converted to straight RGBA8, or where they
/// are on the GPU, for a presenter sharing the renderer's device.
pub struct ScreenImage<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
    pub gpu: Option<GpuScreen>,
}

impl ScreenImage<'_> {
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0 || (self.pixels.is_empty() && self.gpu.is_none())
    }
}

/// a screen's picture upright in an image of the renderer's, in the general
/// layout, which the Vulkan presenter draws straight from when the two share
/// a device.
#[derive(Debug, Clone, Copy)]
pub struct GpuScreen {
    #[cfg(feature = "vulkan")]
    pub(crate) view: ash::vk::ImageView,
    /// where the screen's pixels lie in the image, as texture coordinates,
    /// the corner and the size, then the corners inset by half a texel,
    /// which filtering keeps within.
    pub(crate) area: [f32; 4],
    pub(crate) bounds: [f32; 4],
}

/// what is drawn over the screens, a user interface, as textured triangles
/// in window pixels.
#[derive(Default)]
pub struct Overlay {
    /// textures to make or change before drawing.
    pub textures: Vec<OverlayTexture>,
    pub meshes: Vec<OverlayMesh>,
    /// textures that go away once this frame is drawn.
    pub free: Vec<u64>,
}

/// The id of the texture of a mesh that stands for the bottom screen.
/// The presenter draws the picture of the screen for it.
pub const BOTTOM_SCREEN_TEXTURE: u64 = 1;

/// pixels for a texture of the overlay.
pub struct OverlayTexture {
    pub id: u64,
    /// where the pixels go in a texture that exists, none to make it anew at
    /// this size.
    pub offset: Option<[u32; 2]>,
    pub size: [u32; 2],
    /// RGBA, premultiplied by alpha, in sRGB.
    pub pixels: Vec<u8>,
    /// filtered when scaled, rather than nearest.
    pub linear: bool,
}

/// one corner of an overlay triangle.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct OverlayVertex {
    /// window pixels from the top left.
    pub position: [f32; 2],
    pub uv: [f32; 2],
    /// premultiplied sRGB.
    pub color: [u8; 4],
}

/// triangles of the overlay that share a texture and a clip.
pub struct OverlayMesh {
    pub texture: u64,
    /// the part of the window they may draw in, left, top, right, bottom.
    pub clip: [u32; 4],
    pub vertices: Vec<OverlayVertex>,
    pub indices: Vec<u32>,
}

/// where a screen goes in the window, in pixels from the top left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Viewport {
    /// The left, top, right and bottom edges in whole pixels.
    /// Rounding the edges instead of the size keeps the last column and row of the viewport.
    pub(crate) fn pixels(&self) -> [i32; 4] {
        let edge = |coordinate: f32| coordinate.round() as i32;
        [edge(self.x), edge(self.y), edge(self.x + self.width), edge(self.y + self.height)]
    }
}

/// how the screens are arranged in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenLayout {
    /// the top screen over the bottom one, as on the console.
    #[default]
    Stacked,
    /// next to each other, the top screen on the left.
    SideBySide,
    /// the top screen alone.
    TopOnly,
    /// the bottom screen alone.
    BottomOnly,
    /// the top screen big, the bottom one small at its lower right.
    LargeTop,
    /// The bottom screen drawn over the middle of the top one, as wide as this share of the top screen in ten thousandths.
    BottomOverTop { width: u16 },
    /// The top screen alone, with pieces of the bottom screen drawn over it.
    TopWithPieces(&'static [Piece]),
}

/// A part of the bottom screen shown over the top screen.
/// It is made of rounded rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// The box that holds the piece in the picture of the bottom screen, as the left, top, width and height in pixels.
    pub from: [u16; 4],
    /// Where the box lies across the top screen.
    pub across: Anchor,
    /// Where the box lies down the top screen.
    pub down: Anchor,
    /// The scale of the piece in percent.
    pub size: u16,
    /// The rectangles that make up the piece.
    pub shapes: &'static [Rounded],
    /// The id of a marker the piece moves with, and the position of the marker at rest as the pixels of the picture.
    pub follows: Option<(u32, [u16; 2])>,
}

/// One rectangle of a piece, taken from the picture of the bottom screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rounded {
    /// The left, top, width and height in the picture of the bottom screen, in pixels.
    pub rect: [u16; 4],
    /// How far the rectangle moves from its place in the picture, in pixels.
    pub shift: [i16; 2],
    /// Whether the rectangle is flipped from left to right.
    pub mirror: bool,
    /// The radii of the corners in pixels, in the order top left, top right, bottom left and bottom right.
    pub corners: [u8; 4],
    /// The width of the blur at the edge, in pixels.
    pub soft: u8,
    /// Whether the rectangle is drawn without alpha, so it adds light.
    pub light: bool,
    /// The brightness of the tint, where 255 leaves the picture as it is.
    pub shade: u8,
    /// Whether the rectangle casts a shadow.
    pub shadow: bool,
    /// The points of an outline in tenths of a pixel of the picture.
    /// When it is not empty, the outline is drawn instead of the rectangle.
    pub outline: &'static [[u16; 2]],
    /// When set, only the pixels of the rectangle that are not ground are drawn.
    pub cut: Option<Ground>,
}

/// The places in the picture of the bottom screen that the banners of a battle read their text from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Banners {
    /// The left and the right banner.
    pub sides: [BannerSide; 2],
    /// The rectangles of three labels that do not belong to a side.
    pub labels: [[u16; 4]; 3],
}

/// The places in the picture of the bottom screen that hold the text of one side of a battle.
/// Each rectangle is the left, top, width and height in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BannerSide {
    /// The name of the unit.
    pub name: [u16; 4],
    /// The hit points.
    pub hp: [u16; 4],
    /// The gauge.
    pub gauge: [u16; 4],
    /// The name of the weapon.
    pub weapon: [u16; 4],
    /// Three more numbers.
    pub values: [[u16; 4]; 3],
    /// The id of a marker the side moves with, and the position of the marker at rest as the pixels of the picture.
    pub follows: Option<(u32, [u16; 2])>,
}

/// How to tell the ground of a rectangle from what is drawn on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ground {
    /// How much more red than green a pixel needs to be taken as ground.
    pub redder: u8,
    /// The sum of red, green and blue that a pixel of the ground stays under.
    pub darker: u16,
}

/// The pixels of a rectangle of the picture of the bottom screen that are not ground, as runs of the row, the first column and the end column.
/// The ground is the part that touches the edge of the rectangle.
/// The picture holds 320 by 240 pixels as RGBA.
/// An empty list comes back when the picture is too small or the rectangle lies outside it.
pub fn cut_out(picture: &[u8], rect: [u16; 4], ground: Ground) -> Vec<[u16; 3]> {
    let [left, top, width, height] = rect.map(usize::from);
    // The picture is 320 by 240 pixels of four bytes.
    // The rectangle must lie inside it.
    if picture.len() < 320 * 240 * 4 || left + width > 320 || top + height > 240 || width == 0 || height == 0 {
        return Vec::new();
    }
    // A pixel of the ground is reddish and dark.
    let is_ground = |x: usize, y: usize| {
        let at = ((top + y) * 320 + left + x) * 4;
        let [red, green, blue] = [picture[at], picture[at + 1], picture[at + 2]].map(u16::from);
        red > green + ground.redder as u16 && red + green + blue < ground.darker
    };
    let mut reached = vec![false; width * height];
    // A flood fill starts from every pixel on the border, so a dark pixel inside the drawing is kept.
    let mut stack: Vec<(usize, usize)> = (0..width).map(|x| (x, 0)).chain((0..height).flat_map(|y| [(0, y), (width - 1, y)])).collect();
    while let Some((x, y)) = stack.pop() {
        if reached[y * width + x] || !is_ground(x, y) {
            continue;
        }
        reached[y * width + x] = true;
        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < width {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < height {
            stack.push((x, y + 1));
        }
    }
    // Each row turns into runs of the pixels the fill did not reach.
    let mut runs = Vec::new();
    for y in 0..height {
        let mut x = 0;
        while x < width {
            if reached[y * width + x] {
                x += 1;
                continue;
            }
            let from = x;
            while x < width && !reached[y * width + x] {
                x += 1;
            }
            runs.push([(top + y) as u16, (left + from) as u16, (left + x) as u16]);
        }
    }
    runs
}

/// One rectangle of a piece, placed in the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PiecePlace {
    /// Where the rectangle is drawn, in pixels of the window.
    pub viewport: Viewport,
    /// The part of the picture of the bottom screen to draw, as the left, top, width and height in fractions of the picture.
    /// A negative width flips it.
    pub part: [f32; 4],
    /// The radii of the corners in pixels of the window.
    pub corners: [f32; 4],
    /// The width of the blur at the edge in pixels of the window.
    pub soft: f32,
    /// As in `Rounded`.
    pub light: bool,
    /// As in `Rounded`.
    pub shade: u8,
    /// As in `Rounded`.
    pub shadow: bool,
    /// As in `Rounded`.
    pub outline: &'static [[u16; 2]],
    /// As in `Rounded`.
    pub cut: Option<Ground>,
}

/// Where a box lies along one side of the top screen.
/// The numbers are pixels of a picture 240 high.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// The distance from the start edge.
    Start(u16),
    /// The offset from the middle.
    Middle(i16),
    /// The distance from the end edge.
    End(u16),
}

/// Places the rectangles of pieces over the top screen.
/// The spans are the current areas of the markers.
/// A piece that follows a marker moves with the span of that marker, by at most a tenth of the picture.
pub fn piece_places(top: Viewport, pieces: &[Piece], spans: &[(u32, [f32; 4])]) -> Vec<PiecePlace> {
    // The pictures of the game are 240 pixels high, so this turns their pixels into pixels of the window.
    let scale = top.height / 240.0;
    let position = |anchor: Anchor, start: f32, length: f32, size: f32| match anchor {
        Anchor::Start(margin) => start + margin as f32 * scale,
        Anchor::Middle(offset) => start + (length - size * scale) / 2.0 + offset as f32 * scale,
        Anchor::End(margin) => start + length - (margin as f32 + size) * scale,
    };
    let mut places = Vec::new();
    for piece in pieces {
        let [box_left, box_top, box_width, box_height] = piece.from.map(f32::from);
        let size = piece.size as f32 / 100.0;
        let x = position(piece.across, top.x, top.width, box_width * size);
        let y = position(piece.down, top.y, top.height, box_height * size);
        let step = scale * size;
        // How far the marker moved from its place at rest, in fractions of the picture and limited to a tenth.
        let [moved_x, moved_y] = piece
            .follows
            .and_then(|(id, [rest_x, rest_y])| spans.iter().find(|&&(marked, _)| marked == id).map(|&(_, [x, y, ..])| [x - rest_x as f32 / 320.0, y - rest_y as f32 / 240.0]))
            .map_or([0.0; 2], |moved| moved.map(|by| by.clamp(-0.1, 0.1)));
        for shape in piece.shapes {
            let [shape_x, shape_y, shape_width, shape_height] = shape.rect.map(f32::from);
            let [shift_x, shift_y] = shape.shift.map(f32::from);
            let (to_x, to_y) = (x + (shape_x + shift_x - box_left) * step, y + (shape_y + shift_y - box_top) * step);
            let viewport = Viewport { x: to_x, y: to_y, width: shape_width * step, height: shape_height * step };
            // A mirrored part starts at its right edge and has a negative width.
            let part = match shape.mirror {
                false => [shape_x / 320.0 + moved_x, shape_y / 240.0 + moved_y, shape_width / 320.0, shape_height / 240.0],
                true => [(shape_x + shape_width) / 320.0 + moved_x, shape_y / 240.0 + moved_y, -shape_width / 320.0, shape_height / 240.0],
            };
            let corners = shape.corners.map(|radius| radius as f32 * step);
            let soft = shape.soft as f32 * step;
            places.push(PiecePlace { viewport, part, corners, soft, light: shape.light, shade: shape.shade, shadow: shape.shadow, outline: shape.outline, cut: shape.cut });
        }
    }
    places
}

/// how the screens' pixels are drawn when scaled to the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenFilter {
    /// blended between pixels.
    #[default]
    Smooth,
    /// whole pixels kept sharp and blended only where two meet, so that a
    /// screen scaled by an odd amount stays crisp without stairs.
    Sharp,
    /// every pixel a square of the nearest one.
    Pixels,
}

impl ScreenFilter {
    /// the number the present shaders know it by.
    pub(crate) fn mode(self) -> i32 {
        match self {
            ScreenFilter::Smooth => 0,
            ScreenFilter::Pixels => 1,
            ScreenFilter::Sharp => 2,
        }
    }
}

impl ScreenLayout {
    /// the size the screens take up together at the console's resolution,
    /// with the top screen top_width wide.
    pub fn size(self, top_width: f32) -> (u32, u32) {
        let (width, height) = self.size_f(top_width);
        (width.ceil() as u32, height as u32)
    }

    fn size_f(self, top_width: f32) -> (f32, f32) {
        match self {
            ScreenLayout::Stacked => (top_width.max(320.0), 480.0),
            ScreenLayout::SideBySide => (top_width + 320.0, 240.0),
            ScreenLayout::TopOnly | ScreenLayout::BottomOverTop { .. } | ScreenLayout::TopWithPieces(_) => (top_width, 240.0),
            ScreenLayout::BottomOnly => (320.0, 240.0),
            ScreenLayout::LargeTop => (top_width + LARGE_TOP_BOTTOM.0 as f32, 240.0),
        }
    }
}

/// the top screen's width at the console's resolution, its own shape. a
/// wider one shows a game that a widescreen code makes draw a wider scene
/// into the same 400x240.
pub const TOP_WIDTH: f32 = 400.0;

/// the bottom screen's size next to the big top one, half the console's.
const LARGE_TOP_BOTTOM: (u32, u32) = (160, 120);

/// works out where the screens go inside a window, keeping the 3DS's
/// aspect ratio, or the top screen top_w wide, and centring them, scaled by
/// a whole number when integer asks and the window has room. a screen that
/// does not show has no place.
pub fn layout(window_width: u32, window_height: u32, arrangement: ScreenLayout, top_w: f32, integer: bool) -> (Option<Viewport>, Option<Viewport>) {
    let (total_width, total_height) = arrangement.size_f(top_w);

    let mut scale = (window_width as f32 / total_width).min(window_height as f32 / total_height);
    if integer && scale >= 1.0 {
        scale = scale.floor();
    }
    let mut offset_x = (window_width as f32 - total_width * scale) / 2.0;
    let mut offset_y = (window_height as f32 - total_height * scale) / 2.0;
    if integer {
        // whole pixels, or every texel would not get the same size
        offset_x = offset_x.floor();
        offset_y = offset_y.floor();
    }

    let top = (arrangement != ScreenLayout::BottomOnly).then_some(Viewport {
        x: offset_x,
        y: offset_y,
        width: top_w * scale,
        height: 240.0 * scale,
    });
    let bottom = match arrangement {
        // the bottom screen is narrower, so it is centered under the top one.
        ScreenLayout::Stacked => Some(Viewport {
            x: offset_x + (total_width - 320.0) / 2.0 * scale,
            y: offset_y + 240.0 * scale,
            width: 320.0 * scale,
            height: 240.0 * scale,
        }),
        ScreenLayout::SideBySide => Some(Viewport {
            x: offset_x + top_w * scale,
            y: offset_y,
            width: 320.0 * scale,
            height: 240.0 * scale,
        }),
        ScreenLayout::LargeTop => {
            // half the top screen's scale, a whole number of it when the
            // screens are scaled by whole numbers
            let half = (LARGE_TOP_BOTTOM.0 as f32 / 320.0) * scale;
            let small = if integer && half >= 1.0 { half.floor() } else { half };
            Some(Viewport {
                x: offset_x + top_w * scale,
                y: offset_y + 240.0 * scale - 240.0 * small,
                width: 320.0 * small,
                height: 240.0 * small,
            })
        }
        ScreenLayout::TopOnly | ScreenLayout::TopWithPieces(_) => None,
        ScreenLayout::BottomOverTop { width } => {
            // The width is a share of the top screen in ten thousandths.
            // The width is limited so that the height of the bottom screen does not pass the height of the top one.
            let (top_width, top_height) = (top_w * scale, 240.0 * scale);
            let width = (top_width * width as f32 / 10_000.0).min(top_height * 320.0 / 240.0);
            let height = width * 240.0 / 320.0;
            let whole = |coordinate: f32| if integer { coordinate.floor() } else { coordinate };
            let x = offset_x + (top_width - width) / 2.0;
            let y = offset_y + (top_height - height) / 2.0;
            Some(Viewport { x: whole(x), y: whole(y), width, height })
        }
        ScreenLayout::BottomOnly => Some(Viewport {
            x: offset_x,
            y: offset_y,
            width: 320.0 * scale,
            height: 240.0 * scale,
        }),
    };
    (top, bottom)
}

/// how the top screen's two pictures, one for each eye, are shown when the
/// title draws in 3D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StereoMode {
    /// the left eye's picture alone, as with the 3D slider down.
    #[default]
    Off,
    /// the eyes mixed into red for the left one and cyan for the right, for
    /// glasses of those colors.
    Anaglyph,
    /// each eye in its half of the window, for 3D TVs and headsets.
    SideBySide,
    /// the same squeezed to half width, which a 3D TV stretches back.
    HalfSideBySide,
    /// the eyes on alternate rows of the window, for passive 3D monitors.
    Interlaced,
}

/// the 3D a presenter shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stereo {
    pub mode: StereoMode,
    /// the right eye's picture where the left one's goes, for viewing cross
    /// eyed, glasses worn the other way round or rows counted the other way.
    pub swap: bool,
}

/// which picture a place in the window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// the top screen, as the left eye sees it.
    Top,
    /// the top screen as the right eye sees it.
    Right,
    Bottom,
    /// the top screen's two pictures mixed the way the stereo mode does.
    Both,
}

/// the screens a frame draws, where and from which picture. the 3D shows
/// when the title drew a picture for the right eye, side by side shows the
/// left one's twice otherwise, so a 3D TV keeps its halves.
pub fn place(window_width: u32, window_height: u32, arrangement: ScreenLayout, top_width: f32, integer: bool, stereo: Stereo, right: bool) -> Vec<(Viewport, Shown)> {
    // The overlay draws the bottom screen in this layout, so the presenter does not.
    let bottom_over_top = matches!(arrangement, ScreenLayout::BottomOverTop { .. });
    let screens = |(top, bottom): (Option<Viewport>, Option<Viewport>), eye: Shown| {
        top.map(|viewport| (viewport, eye)).into_iter().chain(bottom.filter(|_| !bottom_over_top).map(|viewport| (viewport, Shown::Bottom)))
    };
    let (first, second) = match (right, stereo.swap) {
        (false, _) => (Shown::Top, Shown::Top),
        (true, false) => (Shown::Top, Shown::Right),
        (true, true) => (Shown::Right, Shown::Top),
    };
    match stereo.mode {
        StereoMode::Off => screens(layout(window_width, window_height, arrangement, top_width, integer), Shown::Top).collect(),
        StereoMode::Anaglyph | StereoMode::Interlaced => {
            let top = if right { Shown::Both } else { Shown::Top };
            screens(layout(window_width, window_height, arrangement, top_width, integer), top).collect()
        }
        StereoMode::SideBySide => {
            // each half a window of its own
            let half = window_width / 2;
            let places = layout(half, window_height, arrangement, top_width, integer);
            let moved = |(viewport, shown): (Viewport, Shown)| (Viewport { x: viewport.x + half as f32, ..viewport }, shown);
            screens(places, first).chain(screens(places, second).map(moved)).collect()
        }
        StereoMode::HalfSideBySide => {
            // the whole window's places, squeezed into each half
            let places = layout(window_width, window_height, arrangement, top_width, integer);
            let half = window_width as f32 / 2.0;
            let squeezed = |by: f32| move |(viewport, shown): (Viewport, Shown)| {
                (Viewport { x: viewport.x / 2.0 + by, width: viewport.width / 2.0, ..viewport }, shown)
            };
            screens(places, first).map(squeezed(0.0)).chain(screens(places, second).map(squeezed(half))).collect()
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PresentError {
    #[error("{0}")]
    Backend(String),
    #[error("the swapchain is out of date and was recreated")]
    OutOfDate,
}

pub trait Presenter {
    fn name(&self) -> &'static str;

    /// draws both screens into the window, and the overlay over them. right
    /// is the top screen's picture for the right eye, empty when the title
    /// draws no 3D or none is shown.
    fn present(
        &mut self,
        top: ScreenImage<'_>,
        bottom: ScreenImage<'_>,
        right: ScreenImage<'_>,
        overlay: &Overlay,
    ) -> Result<(), PresentError>;

    /// the window changed size.
    fn resize(&mut self, width: u32, height: u32);

    /// how the screens are arranged from the next frame on.
    fn set_layout(&mut self, arrangement: ScreenLayout);

    /// how wide the top screen shows, TOP_WIDTH for the console's shape.
    fn set_top_width(&mut self, width: f32);

    /// how the screens are filtered, and whether they are scaled by whole
    /// numbers, from the next frame on.
    fn set_scaling(&mut self, filter: ScreenFilter, integer: bool);

    /// how the 3D shows from the next frame on.
    fn set_stereo(&mut self, stereo: Stereo);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_is_cut_out_of_its_ground() {
        let mut picture = vec![0u8; 320 * 240 * 4];
        let mut put = |x: usize, y: usize, rgb: [u8; 3]| picture[(y * 320 + x) * 4..][..3].copy_from_slice(&rgb);
        for y in 0..240 {
            for x in 0..320 {
                put(x, y, [18, 10, 2]);
            }
        }
        for y in 21..24 {
            for x in 11..14 {
                put(x, y, [40, 120, 30]);
            }
        }
        put(12, 22, [17, 17, 0]);
        put(13, 24, [18, 10, 2]);
        for x in 10..15 {
            put(x, 24, [200, 160, 80]);
        }
        put(12, 23, [18, 10, 2]);
        put(12, 24, [18, 10, 2]);
        let runs = cut_out(&picture, [10, 20, 5, 5], Ground { redder: 3, darker: 90 });
        assert_eq!(runs, [[21, 11, 14], [22, 11, 14], [23, 11, 14], [24, 10, 15]]);
        assert!(cut_out(&picture[..100], [10, 20, 5, 5], Ground { redder: 3, darker: 90 }).is_empty());
    }

    #[test]
    fn a_piece_is_its_rounded_rectangles() {
        const PLAIN: Rounded = Rounded { rect: [0; 4], shift: [0, 0], mirror: false, corners: [0; 4], soft: 0, light: false, shade: 255, shadow: false, outline: &[], cut: None };
        const SHAPES: &[Rounded] = &[
            Rounded { rect: [101, 50, 2, 2], corners: [1, 0, 0, 1], soft: 3, light: true, shade: 200, ..PLAIN },
            Rounded { rect: [100, 51, 4, 1], ..PLAIN },
            Rounded { rect: [100, 50, 1, 1], shift: [3, 0], mirror: true, ..PLAIN },
        ];
        let piece = Piece { from: [100, 50, 4, 2], across: Anchor::End(2), down: Anchor::Start(10), size: 100, shapes: SHAPES, follows: None };
        let top = Viewport { x: 0.0, y: 0.0, width: 800.0, height: 480.0 };
        let places = piece_places(top, &[piece], &[]);
        assert_eq!(places.len(), 3, "a place each");
        let PiecePlace { viewport: first, part, corners, soft, light, shade, shadow, .. } = places[0];
        assert_eq!((first.x, first.y, first.width, first.height), (800.0 - 12.0 + 2.0, 20.0, 4.0, 4.0));
        assert_eq!(part, [101.0 / 320.0, 50.0 / 240.0, 2.0 / 320.0, 2.0 / 240.0]);
        assert_eq!((corners, soft, light, shade, shadow), ([2.0, 0.0, 0.0, 2.0], 6.0, true, 200, false));
        let mirrored = places[2];
        assert_eq!((mirrored.viewport.x, mirrored.viewport.y), (800.0 - 12.0 + 6.0, 20.0));
        assert_eq!(mirrored.part, [101.0 / 320.0, 50.0 / 240.0, -1.0 / 320.0, 1.0 / 240.0]);
        let following = Piece { follows: Some((3, [96, 48])), ..piece };
        let shaken = piece_places(top, &[following], &[(3, [98.0 / 320.0, 48.0 / 240.0, 0.5, 0.4])])[0];
        assert_eq!(shaken.viewport, places[0].viewport);
        assert!((shaken.part[0] - 103.0 / 320.0).abs() < 1e-6 && (shaken.part[1] - places[0].part[1]).abs() < 1e-6);
        let last = places[1].viewport;
        assert_eq!((last.x, last.y, last.width), (800.0 - 12.0, 22.0, 8.0));
        let half = piece_places(top, &[Piece { size: 50, ..piece }], &[])[0];
        assert_eq!((half.viewport.x, half.viewport.y, half.viewport.width, half.viewport.height, half.corners[0]), (800.0 - 4.0 - 4.0 + 1.0, 20.0, 2.0, 2.0, 1.0));
    }

    #[test]
    fn layout_keeps_the_aspect_ratio_and_centers() {
        // a window exactly 400x480 needs no scaling or offset.
        let (top, bottom) = layout(400, 480, ScreenLayout::Stacked, TOP_WIDTH, false);
        let (top, bottom) = (top.unwrap(), bottom.unwrap());
        assert_eq!(top.x, 0.0);
        assert_eq!(top.y, 0.0);
        assert_eq!(top.width, 400.0);
        assert_eq!(bottom.y, 240.0);
        assert_eq!(bottom.x, 40.0);
        assert_eq!(bottom.width, 320.0);

        // doubling both dimensions doubles the scale.
        let top = layout(800, 960, ScreenLayout::Stacked, TOP_WIDTH, false).0.unwrap();
        assert_eq!(top.width, 800.0);
        assert_eq!(top.height, 480.0);

        // a window that is too wide letterboxes horizontally.
        let top = layout(1000, 480, ScreenLayout::Stacked, TOP_WIDTH, false).0.unwrap();
        assert_eq!(top.width, 400.0);
        assert_eq!(top.x, 300.0);
    }

    #[test]
    fn the_other_layouts_place_the_screens_their_way() {
        // side by side, the bottom screen starts where the top one ends
        let (top, bottom) = layout(1440, 480, ScreenLayout::SideBySide, TOP_WIDTH, false);
        let (top, bottom) = (top.unwrap(), bottom.unwrap());
        assert_eq!((top.x, top.y, top.width), (0.0, 0.0, 800.0));
        assert_eq!((bottom.x, bottom.y, bottom.width, bottom.height), (800.0, 0.0, 640.0, 480.0));

        // the top screen alone fills a window of its shape, and a taller
        // one centers it
        let (top, bottom) = layout(800, 960, ScreenLayout::TopOnly, TOP_WIDTH, false);
        assert!(bottom.is_none());
        let top = top.unwrap();
        assert_eq!((top.x, top.y, top.width, top.height), (0.0, 240.0, 800.0, 480.0));

        // and so does the bottom screen alone, a wider window centering it
        let (top, bottom) = layout(800, 480, ScreenLayout::BottomOnly, TOP_WIDTH, false);
        assert!(top.is_none());
        let bottom = bottom.unwrap();
        assert_eq!((bottom.x, bottom.y, bottom.width, bottom.height), (80.0, 0.0, 640.0, 480.0));
    }

    #[test]
    fn the_large_top_screen_takes_most_of_the_window() {
        // the bottom screen at half size sits at the top one's lower right
        let (top, bottom) = layout(1120, 480, ScreenLayout::LargeTop, TOP_WIDTH, false);
        let (top, bottom) = (top.unwrap(), bottom.unwrap());
        assert_eq!((top.x, top.y, top.width, top.height), (0.0, 0.0, 800.0, 480.0));
        assert_eq!((bottom.x, bottom.y, bottom.width, bottom.height), (800.0, 240.0, 320.0, 240.0));
    }

    /// a wider top screen fills a window of its shape, and the screens
    /// around it move over to make room.
    #[test]
    fn a_wider_top_screen_takes_its_shape() {
        let wide = 240.0 * 16.0 / 9.0;
        let (top, _) = layout(1920, 1080, ScreenLayout::TopOnly, wide, false);
        let top = top.unwrap();
        assert_eq!((top.x, top.y, top.width.round(), top.height), (0.0, 0.0, 1920.0, 1080.0));

        // stacked, the bottom screen stays centered under it
        let (top, bottom) = layout(560, 480, ScreenLayout::Stacked, 560.0, false);
        let (top, bottom) = (top.unwrap(), bottom.unwrap());
        assert_eq!((top.x, top.width), (0.0, 560.0));
        assert_eq!((bottom.x, bottom.width), (120.0, 320.0));

        // side by side, it starts where the top screen ends
        let (_, bottom) = layout(880, 240, ScreenLayout::SideBySide, 560.0, false);
        assert_eq!(bottom.unwrap().x, 560.0);
        assert_eq!(ScreenLayout::TopOnly.size(wide), (427, 240));
    }

    #[test]
    fn the_bottom_screen_goes_over_the_middle_of_the_top_one() {
        let wide = 240.0 * 16.0 / 9.0;
        let over = |width| ScreenLayout::BottomOverTop { width };
        let (top, bottom) = layout(1920, 1080, over(3425), wide, false);
        let (alone, _) = layout(1920, 1080, ScreenLayout::TopOnly, wide, false);
        let (top, bottom) = (top.unwrap(), bottom.unwrap());
        assert_eq!(Some(top), alone);
        assert_eq!((bottom.width.round(), bottom.height.round()), (658.0, 493.0));
        assert_eq!(((bottom.x + bottom.width / 2.0).round(), (bottom.y + bottom.height / 2.0).round()), (960.0, 540.0));
        let full = layout(1920, 1080, over(10_000), wide, false).1.unwrap();
        assert_eq!((full.width.round(), full.height), (1440.0, 1080.0));
        assert_eq!(over(3425).size(wide), ScreenLayout::TopOnly.size(wide));
        let places = place(1920, 1080, over(3425), wide, false, Stereo::default(), false);
        assert_eq!(places.iter().map(|(_, shown)| *shown).collect::<Vec<_>>(), [Shown::Top], "the overlay draws the bottom screen");
    }

    #[test]
    fn a_viewport_keeps_its_last_column() {
        assert_eq!(Viewport { x: 1262.4, y: 0.0, width: 657.6, height: 10.0 }.pixels(), [1262, 0, 1920, 10]);
    }

    /// the screens each stereo mode draws, from which picture and where.
    #[test]
    fn the_stereo_modes_place_the_eyes() {
        let on = |mode| Stereo { mode, swap: false };
        let shown = |places: &[(Viewport, Shown)]| places.iter().map(|(_, shown)| *shown).collect::<Vec<_>>();
        let at = |places: &[(Viewport, Shown)], index: usize| {
            let viewport = places[index].0;
            (viewport.x, viewport.y, viewport.width, viewport.height)
        };

        // off, or a title drawing no 3D, the screens as ever
        let off = place(400, 480, ScreenLayout::Stacked, TOP_WIDTH, false, on(StereoMode::Off), true);
        assert_eq!(shown(&off), [Shown::Top, Shown::Bottom]);
        let flat = place(400, 480, ScreenLayout::Stacked, TOP_WIDTH, false, on(StereoMode::Anaglyph), false);
        assert_eq!(shown(&flat), [Shown::Top, Shown::Bottom]);

        // red and cyan or rows, both pictures where the top screen goes
        let mixed = place(400, 480, ScreenLayout::Stacked, TOP_WIDTH, false, on(StereoMode::Interlaced), true);
        assert_eq!(shown(&mixed), [Shown::Both, Shown::Bottom]);
        assert_eq!(at(&mixed, 0), (0.0, 0.0, 400.0, 240.0));

        // side by side, each half a window of its own
        let halves = place(800, 480, ScreenLayout::Stacked, TOP_WIDTH, false, on(StereoMode::SideBySide), true);
        assert_eq!(shown(&halves), [Shown::Top, Shown::Bottom, Shown::Right, Shown::Bottom]);
        assert_eq!(at(&halves, 0), (0.0, 0.0, 400.0, 240.0));
        assert_eq!(at(&halves, 2), (400.0, 0.0, 400.0, 240.0));
        assert_eq!(at(&halves, 3), (440.0, 240.0, 320.0, 240.0));

        // squeezed, the whole window's places at half width in each half
        let squeezed = place(800, 480, ScreenLayout::Stacked, TOP_WIDTH, false, on(StereoMode::HalfSideBySide), true);
        assert_eq!(shown(&squeezed), [Shown::Top, Shown::Bottom, Shown::Right, Shown::Bottom]);
        assert_eq!(at(&squeezed, 0), (100.0, 0.0, 200.0, 240.0));
        assert_eq!(at(&squeezed, 1), (120.0, 240.0, 160.0, 240.0));
        assert_eq!(at(&squeezed, 2), (500.0, 0.0, 200.0, 240.0));

        // swapped, the right eye on the left, and with no 3D drawn both
        // halves the left eye's
        let swapped = place(800, 480, ScreenLayout::Stacked, TOP_WIDTH, false, Stereo { mode: StereoMode::SideBySide, swap: true }, true);
        assert_eq!(shown(&swapped), [Shown::Right, Shown::Bottom, Shown::Top, Shown::Bottom]);
        let flat = place(800, 480, ScreenLayout::TopOnly, TOP_WIDTH, false, on(StereoMode::SideBySide), false);
        assert_eq!(shown(&flat), [Shown::Top, Shown::Top]);
    }

    #[test]
    fn integer_scaling_keeps_whole_multiples() {
        // 2.5 times fits, 2 is used, and the rest is border
        let top = layout(1000, 1200, ScreenLayout::Stacked, TOP_WIDTH, true).0.unwrap();
        assert_eq!((top.width, top.height), (800.0, 480.0));
        assert_eq!((top.x, top.y), (100.0, 120.0));
        // a window smaller than the console's screens still shows them whole
        let top = layout(200, 240, ScreenLayout::Stacked, TOP_WIDTH, true).0.unwrap();
        assert_eq!(top.width, 200.0);
        // an odd window puts the screens on whole pixels
        let (top, bottom) = layout(1001, 961, ScreenLayout::Stacked, TOP_WIDTH, true);
        assert_eq!((top.unwrap().x, top.unwrap().y), (100.0, 0.0));
        assert_eq!(bottom.unwrap().x, 180.0);
        // the small bottom screen of the big top layout keeps a whole scale
        let (top, bottom) = layout(1680, 720, ScreenLayout::LargeTop, TOP_WIDTH, true);
        assert_eq!(top.unwrap().width, 1200.0);
        assert_eq!((bottom.unwrap().width, bottom.unwrap().height), (320.0, 240.0));
    }
}
