//! Overlays that are drawn on top of the picture of the game.
//!
//! This covers the bottom screen laid over the top screen, the pad pointer, the pieces of the bottom screen, the buttons of a battle and the HD banners of the battle forecast.
//! It also holds the checks that tell which screen of the game is shown.

use std::time::{Duration, Instant};

use egui::Color32;
use zakuro_core::services::hid::{InputState, PadState};

/// The kind of scene the game currently shows, as far as the overlays are concerned.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scene {
    /// Any scene that needs no special handling.
    Plain,
    /// A battle scene, with the pieces of the bottom screen that hold the buttons.
    Battle(&'static [zakuro_gpu::Piece]),
    /// The world map.
    WorldMap,
    /// The view of a unit.
    UnitView,
    /// The level up screen.
    LevelUp,
    /// The battle forecast.
    Forecast,
    /// A scene whose bottom screen is drawn as separate pieces, such as the title screen.
    Pieces(&'static [zakuro_gpu::Piece]),
}

/// The size of the soft edge of the bottom screen laid over the top screen.
pub(crate) const BOTTOM_OVER_EDGE: f32 = 4.0;
/// The width of the bottom screen laid over the top screen, as a fraction of the width of the top screen.
pub(crate) const BOTTOM_OVER_WIDTH: f32 = 0.45;

/// The size of the soft shadow below each piece of the bottom screen.
pub(crate) const PIECE_SHADOW: f32 = 1.5;
/// The time in seconds that a piece of the bottom screen takes to fade.
pub(crate) const PIECE_FADE: f32 = 0.2;

/// The time in seconds that the buttons of a battle take to fade in.
const BUTTONS_IN: f32 = 0.1;
/// The time in seconds that the buttons of a battle take to fade out.
const BUTTONS_OUT: f32 = 0.3;
/// The time that the buttons of a battle stay fully visible after the last request to show them.
const BUTTONS_STAY: Duration = Duration::from_secs(3);

/// The buttons of a battle that are currently shown or fading, with the times that drive the fade.
#[derive(Clone, Copy)]
pub(crate) struct ShownButtons {
    /// The pieces of the bottom screen that are drawn as the buttons.
    pub(crate) pieces: &'static [zakuro_gpu::Piece],
    /// The moment the fade in started, shifted back when the buttons were already partly visible.
    since: Instant,
    /// The moment the fade out starts.
    until: Instant,
}

impl ShownButtons {
    /// Returns how visible the buttons are at the given moment, from 0.0 to 1.0.
    pub(crate) fn opacity(&self, now: Instant) -> f32 {
        let coming = |at: Instant| (at.saturating_duration_since(self.since).as_secs_f32() / BUTTONS_IN).min(1.0);
        if now < self.until {
            coming(now)
        } else {
            (coming(self.until) - now.duration_since(self.until).as_secs_f32() / BUTTONS_OUT).max(0.0)
        }
    }

    /// Returns the buttons to show after this frame.
    ///
    /// Without pieces, the buttons start to fade out at once.
    /// A press of a button, or a touch while the buttons are visible, shows them again and restarts the time they stay.
    pub(crate) fn next(shown: Option<ShownButtons>, pieces: Option<&'static [zakuro_gpu::Piece]>, pressed: zakuro_core::services::hid::PadState, touched: bool, any_button: bool, now: Instant) -> Option<ShownButtons> {
        let Some(pieces) = pieces else {
            return shown.map(|shown| ShownButtons { until: shown.until.min(now), ..shown });
        };
        let opacity = shown.map_or(0.0, |shown| shown.opacity(now));
        use zakuro_core::services::hid::PadState;
        // The sticks do not count as a press, so moving them does not bring the buttons.
        let sticks = PadState::C_STICK_RIGHT | PadState::C_STICK_LEFT | PadState::C_STICK_UP | PadState::C_STICK_DOWN | PadState::CIRCLE_RIGHT | PadState::CIRCLE_LEFT | PadState::CIRCLE_UP | PadState::CIRCLE_DOWN;
        let asked = any_button || !pressed.difference(sticks).is_empty() || (opacity > 0.0 && touched);
        if !asked {
            return shown;
        }
        // Start the fade in early by the part that is already visible, so the opacity does not jump.
        let since = now.checked_sub(Duration::from_secs_f32(opacity * BUTTONS_IN)).unwrap_or(now);
        Some(ShownButtons { pieces, since, until: now + BUTTONS_STAY })
    }
}

/// The radius of the rounded corners of the bottom screen laid over the top screen.
pub(crate) const BOTTOM_OVER_CORNER: f32 = 6.0;

/// The top speed of the pad pointer, in pixels of the bottom screen per second.
const POINTER_SPEED: f32 = 280.0;
/// The radius of the pad pointer, in pixels of the bottom screen.
const POINTER_RADIUS: f32 = 10.0;

/// A pointer on the bottom screen that is steered with a stick of the gamepad.
pub(crate) struct PadPointer {
    /// The position of the pointer, in pixels of the bottom screen.
    pub(crate) at: [f32; 2],
    /// Whether the pointer is visible.
    pub(crate) shown: bool,
    /// The moment of the last steer, used to measure the time between two steers.
    last: Option<Instant>,
}

impl Default for PadPointer {
    /// Places the pointer in the middle of the bottom screen, hidden.
    fn default() -> Self {
        PadPointer { at: [160.0, 120.0], shown: false, last: None }
    }
}

impl PadPointer {
    /// Moves the pointer by the lean of the stick over the time since the last steer.
    ///
    /// A lean inside the dead zone does nothing.
    /// Any other lean shows the pointer.
    pub(crate) fn steer(&mut self, lean: [f32; 2], now: Instant) {
        // Cap the time step so a long pause does not throw the pointer across the screen.
        let elapsed = self.last.replace(now).map_or(0.0, |last| now.duration_since(last).as_secs_f32().min(0.05));
        let length = lean[0].hypot(lean[1]);
        let deadzone = crate::gamepad::DEADZONE;
        if length <= deadzone {
            return;
        }
        self.shown = true;
        // The speed grows with the square of the lean past the dead zone, and the division by the length turns the lean into a direction.
        let speed = POINTER_SPEED * ((length - deadzone) / (1.0 - deadzone)).min(1.0).powi(2) / length;
        // The y axis of the stick points up, while the y axis of the screen points down.
        self.at = [(self.at[0] + lean[0] * speed * elapsed).clamp(0.0, 319.0), (self.at[1] - lean[1] * speed * elapsed).clamp(0.0, 239.0)];
    }

    /// Hides the pointer and forgets the moment of the last steer.
    pub(crate) fn rest(&mut self) {
        self.shown = false;
        self.last = None;
    }

    /// Returns the position of the pointer rounded to whole pixels of the bottom screen.
    pub(crate) fn spot(&self) -> (u16, u16) {
        (self.at[0].round() as u16, self.at[1].round() as u16)
    }
}

/// Draws the pad pointer at the given position of the bottom screen, which fills the given rectangle.
///
/// The pointer is smaller and brighter while it is pressed.
pub fn pad_pointer(ctx: &egui::Context, rect: egui::Rect, at: [f32; 2], pressed: bool) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    // The bottom screen is 320 pixels wide.
    let scale = rect.width() / 320.0;
    let center = rect.min + egui::vec2(at[0], at[1]) * scale;
    let radius = POINTER_RADIUS * scale * if pressed { 0.8 } else { 1.0 };
    let fill = Color32::from_white_alpha(if pressed { 110 } else { 25 });
    painter.circle(center, radius, fill, egui::Stroke::new(scale.max(1.0) * 2.6, Color32::from_black_alpha(150)));
    painter.circle_stroke(center, radius, egui::Stroke::new(scale.max(1.0) * 1.2, Color32::WHITE));
    painter.circle(center, 1.6 * scale.max(1.0), Color32::WHITE, egui::Stroke::new(scale.max(1.0) * 0.8, Color32::from_black_alpha(150)));
}

/// The time after a touch during which the touch still counts as recent.
pub(crate) const TOUCH_KEEPS_OVER: Duration = Duration::from_secs(1);

/// The point on the bottom screen that is touched to open the map button.
pub(crate) const MAP_BUTTON: (u16, u16) = (305, 212);
/// The point on the bottom screen that is touched to open the inventory tab.
pub(crate) const INVENTORY_TAB: (u16, u16) = (305, 70);
/// The point on the bottom screen that is touched to close the story screen.
pub(crate) const STORY_CLOSE: (u16, u16) = (293, 213);

/// Returns whether the point on the bottom screen lies on the portrait in the top left corner.
pub(crate) fn on_portrait((x, y): (u16, u16)) -> bool {
    x < 64 && y < 62
}
/// The number of frames that a tap on the bottom screen is held.
pub(crate) const TAP_FRAMES: u8 = 6;
/// The largest number of frames to wait for the overview of the world map to show.
pub(crate) const OVERVIEW_WAIT: u8 = 45;
/// The number of frames after a tap on the portrait during which the scene counts as the view of the unit.
pub(crate) const UNIT_VIEW_START: u64 = 40;
/// The number of frames after the game last talked during which the scene still counts as the view of the unit.
pub(crate) const UNIT_VIEW_GAP: u64 = 90;

/// Returns whether the picture of the bottom screen shows the level up screen.
///
/// The picture is RGBA with 320 by 240 pixels.
/// The level up screen has dark margins, a light panel and a mid-dark strip below the panel.
pub(crate) fn level_up_screen(picture: &[u8]) -> bool {
    if picture.len() < 320 * 240 * 4 {
        return false;
    }
    // The average brightness of a region of the picture, from 0 to 255.
    let light = |[left, top, right, bottom]: [usize; 4]| {
        let mut sum = 0;
        for y in top..bottom {
            for x in left..right {
                let at = (y * 320 + x) * 4;
                sum += u32::from(picture[at]) + u32::from(picture[at + 1]) + u32::from(picture[at + 2]);
            }
        }
        sum / (3 * ((right - left) * (bottom - top)) as u32)
    };
    // The regions and limits are tuned to the look of the screen.
    let (margin, panel, strip) = (light([284, 32, 318, 220]), light([60, 40, 260, 200]), light([60, 226, 260, 238]));
    margin < 30 && panel > 150 && (25..80).contains(&strip)
}

/// The screens of the bottom screen that can be told apart on the world map.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MapScreen {
    /// The inventory.
    Inventory,
    /// The overview of the map.
    Overview,
    /// The story page.
    Story,
}

/// Returns which screen of the world map the picture of the bottom screen shows, or `None` when it matches none of them.
///
/// The picture is RGBA with 320 by 240 pixels.
pub(crate) fn map_screen(picture: &[u8]) -> Option<MapScreen> {
    if picture.len() < 320 * 240 * 4 {
        return None;
    }
    // The average brightness of a region of the picture, from 0 to 255.
    let light = |[left, top, right, bottom]: [usize; 4]| {
        let mut sum = 0;
        for y in top..bottom {
            for x in left..right {
                let at = (y * 320 + x) * 4;
                sum += u32::from(picture[at]) + u32::from(picture[at + 1]) + u32::from(picture[at + 2]);
            }
        }
        sum / (3 * ((right - left) * (bottom - top)) as u32)
    };
    let (corner, icon, edge) = (light([225, 22, 300, 38]), light([292, 198, 318, 226]), light([290, 105, 318, 185]));
    let (page, close, bar) = (light([30, 60, 260, 165]), light([282, 204, 304, 222]), light([80, 12, 250, 28]));
    // The order matters: the story page is checked first, and a bright right edge rules out the other two screens.
    match () {
        _ if page > 130 && close > 150 && bar < 90 => Some(MapScreen::Story),
        _ if edge >= 40 => None,
        _ if corner > 115 && icon < 75 => Some(MapScreen::Inventory),
        _ if corner < 85 && icon > 90 => Some(MapScreen::Overview),
        _ => None,
    }
}

/// Decides whether the bottom screen stays laid over the top screen, and updates the scene that is kept.
///
/// A scene is kept when a touch on the bottom screen opened it while the game is talking.
/// The kept scene is dropped as soon as the game stops talking.
pub(crate) fn keeps_bottom_over(moved_on: bool, opens: bool, talking: bool, touched_lately: bool, scene: Scene, kept: &mut Option<Scene>) -> bool {
    if kept.is_some() && !talking {
        *kept = None;
        return false;
    }
    if !moved_on {
        return true;
    }
    if opens && talking && (touched_lately || *kept == Some(scene)) {
        *kept = Some(scene);
        return true;
    }
    *kept = None;
    false
}

/// Returns the part of the held input that is passed on to the game while the bottom screen is laid over the top screen.
///
/// The touch is always passed on.
/// The buttons that are passed on depend on the scene.
pub(crate) fn bottom_over_input(held: InputState, scene: Scene) -> InputState {
    let buttons = match scene {
        Scene::Plain | Scene::UnitView => held.buttons.intersection(PadState::L),
        Scene::LevelUp => held.buttons,
        _ => PadState::empty(),
    };
    InputState { touch: held.touch, buttons, ..Default::default() }
}

/// Draws the pieces of the bottom screen at the places that the renderer reports.
///
/// The cuts hold, for each piece, the runs of pixels that belong to it, as `[row, from, to]`.
/// The opacity scales the shade and the alpha of every piece.
pub fn pieces(ctx: &egui::Context, places: &[zakuro_gpu::PiecePlace], cuts: &[Vec<[u16; 3]>], edge: f32, opacity: f32) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let ppp = ctx.pixels_per_point();
    // Maps a pixel of the bottom screen to a position on the window and to a texture coordinate.
    let map = |place: &zakuro_gpu::PiecePlace, x: f32, y: f32| {
        let [left, top, width, height] = place.part;
        let (u, v) = (x / 320.0, y / 240.0);
        let at = place.viewport;
        let pos = egui::pos2(at.x + (u - left) / width * at.width, at.y + (v - top) / height * at.height) / ppp;
        (pos, egui::pos2(u, v))
    };
    // The rectangle on the window that encloses all runs of a cut piece.
    let cut_box = |place: &zakuro_gpu::PiecePlace, runs: &[[u16; 3]]| {
        let (mut low, mut high) = (egui::pos2(f32::MAX, f32::MAX), egui::pos2(f32::MIN, f32::MIN));
        for &[row, from, to] in runs {
            for (x, y) in [(from as f32, row as f32), (to as f32, row as f32 + 1.0)] {
                let (pos, _) = map(place, x, y);
                (low, high) = (low.min(pos), high.max(pos));
            }
        }
        egui::Rect::from_two_pos(low, high)
    };
    let rect = |place: &zakuro_gpu::PiecePlace| {
        let at = place.viewport;
        egui::Rect::from_min_size(egui::pos2(at.x, at.y) / ppp, egui::vec2(at.width, at.height) / ppp)
    };
    let fade = |value: u8| (value as f32 * opacity.clamp(0.0, 1.0)).round() as u8;
    // The corner radii of a piece, converted from pixels of the viewport to points.
    let corners = |place: &zakuro_gpu::PiecePlace| {
        let [nw, ne, sw, se] = place.corners.map(|radius| (radius / ppp).round().clamp(0.0, 255.0) as u8);
        egui::CornerRadius { nw, ne, sw, se }
    };
    let no_runs = Vec::new();
    let runs = |index: usize| cuts.get(index).unwrap_or(&no_runs);
    for (index, place) in places.iter().enumerate().filter(|(_, place)| place.shadow) {
        let shape = match place.cut {
            Some(_) if runs(index).is_empty() => continue,
            Some(_) => cut_box(place, runs(index)),
            None => rect(place),
        };
        painter.add(shadow(edge, Color32::from_rgba_unmultiplied(0, 0, 0, fade(77))).as_shape(shape, corners(place)));
    }
    // The shadows are drawn first, so that no shadow covers a piece.
    for (index, place) in places.iter().enumerate() {
        let [left, top, width, height] = place.part;
        let uv = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height));
        let shade = fade(place.shade);
        // A light piece has no alpha with a premultiplied color, so it adds light instead of covering what lies below.
        let alpha = if place.light { 0 } else { fade(255) };
        let tint = egui::Color32::from_rgba_premultiplied(shade, shade, shade, alpha);
        if !place.outline.is_empty() {
            // Pieces that share an outline are drawn once.
            if places[..index].iter().any(|other| other.outline == place.outline) {
                continue;
            }
            // The points of an outline are stored in tenths of a pixel.
            let points: Vec<_> = place.outline.iter().map(|&[x, y]| map(place, x as f32 / 10.0, y as f32 / 10.0)).collect();
            painter.add(egui::Shape::mesh(outline_mesh(crate::gui::BOTTOM_SCREEN, &points, tint, 1.0 / ppp)));
            continue;
        }
        // A cut piece is drawn as one quad for each run of pixels.
        if place.cut.is_some() {
            let mut mesh = egui::Mesh::with_texture(crate::gui::BOTTOM_SCREEN);
            for &[row, from, to] in runs(index) {
                let corners = [(from, row), (to, row), (to, row + 1), (from, row + 1)].map(|(x, y)| map(place, x as f32, y as f32));
                let first = mesh.vertices.len() as u32;
                for (pos, uv) in corners {
                    mesh.vertices.push(egui::epaint::Vertex { pos, uv, color: tint });
                }
                mesh.indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
            }
            painter.add(egui::Shape::mesh(mesh));
            continue;
        }
        let shape = egui::epaint::RectShape::filled(rect(place), corners(place), tint);
        painter.add(shape.with_blur_width(place.soft / ppp).with_texture(crate::gui::BOTTOM_SCREEN, uv));
    }
}

/// The textures of the banners, kept between frames so that they are only uploaded when the text changes.
#[derive(Default)]
pub struct BannerTextures {
    /// One texture for each text of the banners, with the text it was made from.
    slots: Vec<Option<(crate::banners::Text, egui::TextureHandle)>>,
    /// The grain texture that is laid over the panels.
    noise: Option<egui::TextureHandle>,
}

impl BannerTextures {
    /// Returns the grain texture, and creates it on the first call.
    ///
    /// The texture holds random light and dark dots made with a small xorshift generator that has a fixed seed, so the grain looks the same on every run.
    fn noise(&mut self, ctx: &egui::Context) -> egui::TextureId {
        let handle = self.noise.get_or_insert_with(|| {
            let (width, height) = (160, 24);
            let mut seed = 0x2545_F491_u32;
            let mut rgba = Vec::with_capacity(width * height * 4);
            for _ in 0..width * height {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let v = (seed % 1000) as f32 / 500.0 - 1.0;
                let (shade, alpha) = if v > 0.0 { (255, v * 0.10) } else { (0, -v * 0.14) };
                rgba.extend_from_slice(&[shade, shade, shade, (alpha * 255.0) as u8]);
            }
            let image = egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba);
            ctx.load_texture("banner noise", image, egui::TextureOptions::LINEAR)
        });
        handle.id()
    }

    /// Returns the texture of a text in the given slot, or `None` when the text is empty.
    ///
    /// The texture is only uploaded again when the text differs from the one that the slot holds.
    fn texture(&mut self, ctx: &egui::Context, slot: usize, text: &crate::banners::Text) -> Option<egui::TextureId> {
        if text.size[0] == 0 || text.size[1] == 0 {
            return None;
        }
        if self.slots.len() <= slot {
            self.slots.resize_with(slot + 1, || None);
        }
        let image = || egui::ColorImage::from_rgba_unmultiplied(text.size, &text.rgba);
        match &mut self.slots[slot] {
            Some((kept, handle)) if kept == text => Some(handle.id()),
            Some((kept, handle)) => {
                handle.set(image(), egui::TextureOptions::LINEAR);
                *kept = text.clone();
                Some(handle.id())
            }
            None => {
                let handle = ctx.load_texture(format!("banner {slot}"), image(), egui::TextureOptions::LINEAR);
                let id = handle.id();
                self.slots[slot] = Some((text.clone(), handle));
                Some(id)
            }
        }
    }
}

/// The largest number of ticks in one row of the HP bar.
const TICKS_A_ROW: u32 = 40;
/// The distance between the starts of two ticks of the HP bar, in units of the banner.
const TICK_PITCH: f32 = 9.0;

/// The free space between the last tick and the tip of the HP bar, in units of the banner.
const TICKS_CLEAR: f32 = 5.0;

/// The x position of the tip of the wide panels of the banner, in units of the banner.
const BANNER_TIP: f32 = 540.0;

/// How far the shadow of a panel of the banner is moved down, in units of the banner.
const PANEL_SHADOW_DROP: f32 = 3.0;
/// How far the shadow of a panel of the banner reaches out, in units of the banner.
const PANEL_SHADOW_REACH: f32 = 7.0;
/// The opacity of the shadow of a panel of the banner.
const PANEL_SHADOW_ALPHA: f32 = 0.55;

/// Draws the two banners of the battle forecast at the bottom of the top screen.
///
/// Each side shows the name, the HP bar, the weapon and the stats of a unit.
/// The text comes from the read banners and is drawn from textures.
/// The opacity fades the whole drawing.
pub fn banners(ctx: &egui::Context, textures: &mut BannerTextures, read: &crate::banners::Read, top: egui::Rect, opacity: f32) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let opacity = opacity.clamp(0.0, 1.0);
    let color = |[r, g, b]: [u8; 3], alpha: f32| Color32::from_rgba_unmultiplied(r, g, b, (alpha * opacity * 255.0).round() as u8);
    let lighter = |c: [u8; 3], by: u8| c.map(|v| v.saturating_add(by));
    // The banner is designed for a screen that is 1080 pixels high, and the 0.85 shrinks it a little.
    let unit = top.height() / 1080.0 * 0.85;
    // The banner is 312 units high and sits 50 units above the bottom of the top screen.
    let group_top = top.bottom() - (50.0 + 312.0) * unit;
    let white = color([255, 255, 255], 1.0);
    let noise = textures.noise(ctx);
    let fade = Color32::from_white_alpha((opacity * 255.0).round() as u8);
    // Lays the grain texture over a convex polygon, as a fan of triangles around its center.
    let grain = |points: &[egui::Pos2]| {
        let mut mesh = egui::Mesh::with_texture(noise);
        let bounds = egui::Rect::from_points(points);
        let uv = |p: egui::Pos2| egui::pos2((p.x - bounds.left()) / bounds.width().max(1.0), (p.y - bounds.top()) / bounds.height().max(1.0));
        let middle = bounds.center();
        mesh.vertices.push(egui::epaint::Vertex { pos: middle, uv: uv(middle), color: fade });
        for &p in points {
            mesh.vertices.push(egui::epaint::Vertex { pos: p, uv: uv(p), color: fade });
        }
        let n = points.len() as u32;
        for i in 0..n {
            mesh.indices.extend([0, 1 + i, 1 + (i + 1) % n]);
        }
        painter.add(egui::Shape::mesh(mesh));
    };
    // Draws a text texture centered on a point. Every call takes the next texture slot, so the order of the calls must stay the same from frame to frame.
    let mut slot = 0;
    let mut draw_text = |text: &crate::banners::Text, center: egui::Pos2, scale: f32| {
        let id = textures.texture(ctx, slot, text);
        slot += 1;
        if let Some(id) = id {
            let size = egui::vec2(text.size[0] as f32, text.size[1] as f32) * scale * unit;
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let center = center + egui::vec2(0.0, text.down * scale * unit);
            painter.image(id, egui::Rect::from_center_size(center, size), uv, white);
        }
    };
    for (index, side) in read.sides.iter().enumerate() {
        // The second side is mirrored on the right edge of the screen.
        let right = index == 1;
        // Maps a point of the banner to the screen, measured from the near edge of the side.
        let at = |x: f32, y: f32| egui::pos2(if right { top.right() - x * unit } else { top.left() + x * unit }, group_top + y * unit);
        let place = |points: &[(f32, f32)]| {
            let mut points: Vec<egui::Pos2> = points.iter().map(|&(x, y)| at(x, y)).collect();
            // Mirroring flips the winding of the polygon, so the points are reversed to keep it.
            if right {
                points.reverse();
            }
            points
        };
        let shape = |points: &[(f32, f32)], fill: Color32, stroke: (f32, Color32)| egui::Shape::convex_polygon(place(points), fill, egui::Stroke::new(stroke.0 * unit, stroke.1));
        let diamond = |cx: f32, cy: f32, r: f32| [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
        // On the right side the stat columns run in the opposite order, so that the same stat sits next to the middle on both sides.
        let stat_columns = if right { [(390.0, 530.0), (230.0, 370.0), (70.0, 210.0)] } else { [(70.0, 210.0), (230.0, 370.0), (390.0, 530.0)] };
        let band = [(84.0, 0.0), (516.0, 0.0), (BANNER_TIP, 27.0), (516.0, 54.0), (84.0, 54.0), (60.0, 27.0)];
        let (bar, inside, width, height, gap) = hp_bar(side.ticks.len());
        let hp = diamond(104.0, 110.0, 48.0);
        let weapon = [(60.0, 266.0), (BANNER_TIP, 266.0), (BANNER_TIP, 312.0), (60.0, 312.0)];
        let boxes = |(a, b): (f32, f32)| ([(a, 166.0), (b, 166.0), (b, 206.0), (a, 206.0)], [(a, 208.0), (b, 208.0), (b, 248.0), (a, 248.0)]);
        let stat_boxes: Vec<_> = stat_columns.iter().map(|&col| boxes(col)).collect();
        let mut outlines: Vec<&[(f32, f32)]> = vec![&band, &bar, &hp, &weapon];
        for (tab, cell) in &stat_boxes {
            outlines.extend([&tab[..], &cell[..]]);
        }
        // Every panel casts a soft shadow that is moved down a little.
        let shade = color([0, 0, 0], PANEL_SHADOW_ALPHA);
        for outline in outlines {
            let below: Vec<(f32, f32)> = outline.iter().map(|&(x, y)| (x, y + PANEL_SHADOW_DROP)).collect();
            let points: Vec<(egui::Pos2, egui::Pos2)> = place(&below).into_iter().map(|p| (p, egui::epaint::WHITE_UV)).collect();
            painter.add(egui::Shape::mesh(outline_mesh(egui::TextureId::default(), &points, shade, PANEL_SHADOW_REACH * unit)));
        }
        let edge = color([230, 205, 140], 1.0);
        painter.add(shape(&band, color(side.name.ground, 1.0), (3.0, edge)));
        grain(&place(&band));
        for x in [90.0, 510.0] {
            painter.add(shape(&diamond(x, 27.0, 9.0), color([255, 255, 255], 0.55), (0.0, Color32::TRANSPARENT)));
        }
        draw_text(&side.name, at(300.0, 29.0), 3.3);
        let ground = if right { [44, 12, 16] } else { [18, 16, 30] };
        painter.add(shape(&bar, color(ground, 1.0), (2.0, color([220, 220, 230], 1.0))));
        let fill = if right { [235, 70, 50] } else { [70, 130, 235] };
        // The HP bar shows at most two rows of ticks, and each pair holds the lit ticks and all ticks of the row.
        for (row, &[lit, all]) in side.ticks.iter().take(2).enumerate() {
            let y0 = 110.0 - inside / 2.0 + row as f32 * (height + gap);
            for tick in 0..all.min(TICKS_A_ROW) {
                let x0 = 159.0 + tick as f32 * TICK_PITCH;
                let x1 = x0 + width;
                // Lit ticks alternate between two shades, and unlit ticks are grey.
                let (body, top) = match tick < lit {
                    true if tick % 2 == 1 => (lighter(fill, 25), lighter(fill, 85)),
                    true => (fill, lighter(fill, 60)),
                    false => ([52, 52, 60], [66, 66, 76]),
                };
                painter.add(shape(&[(x0, y0), (x1, y0), (x1, y0 + height), (x0, y0 + height)], color(body, 1.0), (0.0, Color32::TRANSPARENT)));
                painter.add(shape(&[(x0, y0), (x1, y0), (x1, y0 + height * 0.35), (x0, y0 + height * 0.35)], color(top, 1.0), (0.0, Color32::TRANSPARENT)));
            }
        }
        painter.add(shape(&hp, color(side.hp.ground, 1.0), (3.0, color([230, 225, 240], 1.0))));
        grain(&place(&diamond(104.0, 110.0, 46.0)));
        draw_text(&side.hp, at(104.0, 110.0), 2.0);
        painter.add(shape(&weapon, color(side.weapon.ground, 1.0), (2.0, color([235, 235, 235], 1.0))));
        grain(&place(&weapon));
        for x in [76.0, 524.0] {
            painter.add(shape(&diamond(x, 289.0, 6.0), color([240, 240, 240], 1.0), (0.0, Color32::TRANSPARENT)));
        }
        draw_text(&side.weapon, at(300.0, 289.0), 2.4);
        for (stat, (&(a, b), (tab, cell))) in stat_columns.iter().zip(&stat_boxes).enumerate() {
            let label = &read.labels[stat];
            painter.add(shape(tab, color(label.ground, 1.0), (2.0, color([235, 225, 200], 1.0))));
            grain(&place(tab));
            draw_text(label, at((a + b) / 2.0, 186.0), 1.9);
            let value = &side.values[stat];
            painter.add(shape(cell, color(value.ground, 1.0), (2.0, white)));
            grain(&place(cell));
            draw_text(value, at((a + b) / 2.0, 228.0), 2.6);
        }
    }
}

/// Returns the outline of the HP bar and the sizes of its ticks, in units of the banner.
///
/// The result is the outline, the height inside the bar, and the width, the height and the gap of the ticks.
/// The bar holds one or two rows, and its tip is pulled in so that it never reaches the last tick.
fn hp_bar(rows: usize) -> ([(f32, f32); 5], f32, f32, f32, f32) {
    let rows = rows.clamp(1, 2) as f32;
    let (width, height, gap) = (TICK_PITCH * 2.0 / 3.0, TICK_PITCH * 8.0 / 3.0, TICK_PITCH * 2.0 / 3.0);
    let inside = rows * height + (rows - 1.0) * gap;
    let half = inside / 2.0 + 4.0;
    let start = 148.0 - half;
    let ticks_end = 159.0 + (TICKS_A_ROW - 1) as f32 * TICK_PITCH + width;
    let point = (half * 0.65).min((BANNER_TIP - ticks_end - TICKS_CLEAR) * half / (inside / 2.0));
    let bar = [(start, 110.0 - half), (BANNER_TIP - point, 110.0 - half), (BANNER_TIP, 110.0), (BANNER_TIP - point, 110.0 + half), (start, 110.0 + half)];
    (bar, inside, width, height, gap)
}

/// Builds a mesh that fills a convex polygon and fades out to transparent over a soft edge.
///
/// Each point holds a position and a texture coordinate.
/// The edge reaches out by the feather, measured along the normal at each corner.
fn outline_mesh(texture: egui::TextureId, points: &[(egui::Pos2, egui::Pos2)], tint: Color32, feather: f32) -> egui::Mesh {
    let mut mesh = egui::Mesh::with_texture(texture);
    let n = points.len();
    if n < 3 {
        return mesh;
    }
    let (mut low, mut high) = (points[0], points[0]);
    for &(pos, uv) in points {
        low = (low.0.min(pos), low.1.min(uv));
        high = (high.0.max(pos), high.1.max(uv));
    }
    let middle = (low.0 + (high.0 - low.0) / 2.0, low.1 + (high.1 - low.1) / 2.0);
    // The sign of the area tells the winding of the polygon, which decides which side is outward.
    let area: f32 = (0..n).map(|i| points[i].0.x * points[(i + 1) % n].0.y - points[(i + 1) % n].0.x * points[i].0.y).sum();
    let outward = |a: egui::Pos2, b: egui::Pos2| {
        let d = (b - a).normalized();
        if area > 0.0 { egui::vec2(d.y, -d.x) } else { egui::vec2(-d.y, d.x) }
    };
    mesh.vertices.push(egui::epaint::Vertex { pos: middle.0, uv: middle.1, color: tint });
    for &(pos, uv) in points {
        mesh.vertices.push(egui::epaint::Vertex { pos, uv, color: tint });
    }
    for i in 0..n {
        let (before, here, after) = (points[(i + n - 1) % n].0, points[i].0, points[(i + 1) % n].0);
        let (a, b) = (outward(before, here), outward(here, after));
        let normal = (a + b).normalized();
        // Divide by the cosine of the corner so the edge keeps the same width, and cap it at sharp corners.
        let reach = feather / normal.dot(a).max(0.3);
        mesh.vertices.push(egui::epaint::Vertex { pos: here + normal * reach, uv: points[i].1, color: Color32::TRANSPARENT });
    }
    for i in 0..n as u32 {
        let (here, next) = (1 + i, 1 + (i + 1) % n as u32);
        let (out_here, out_next) = (here + n as u32, next + n as u32);
        mesh.indices.extend([0, here, next, here, out_next, next, here, out_here, out_next]);
    }
    mesh
}

/// Returns a shadow with the given color whose blur and spread follow the size of the edge.
fn shadow(edge: f32, color: Color32) -> egui::epaint::Shadow {
    let points = |value: f32| value.round().clamp(0.0, 255.0) as u8;
    egui::epaint::Shadow { offset: [0, 0], blur: points(edge * 1.5), spread: points(edge / 2.0), color }
}

/// Draws the bottom screen with a shadow and rounded corners inside the given rectangle.
pub fn bottom_over(ctx: &egui::Context, rect: egui::Rect, edge: f32, corner: f32) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let points = |value: f32| value.round().clamp(0.0, 255.0) as u8;
    painter.add(shadow(edge, Color32::from_rgba_unmultiplied(0x2A, 0x1A, 0x0A, 180)).as_shape(rect, points(corner)));
    let screen = egui::epaint::RectShape::filled(rect, points(corner), Color32::WHITE);
    painter.add(screen.with_texture(crate::gui::BOTTOM_SCREEN, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_up_screen_is_a_light_panel_between_dark_edges() {
        let mut picture = vec![180; 320 * 240 * 4];
        let fill = |picture: &mut Vec<u8>, [left, top, right, bottom]: [usize; 4], value: u8| {
            for y in top..bottom {
                for x in left..right {
                    picture[(y * 320 + x) * 4..][..3].fill(value);
                }
            }
        };
        fill(&mut picture, [280, 0, 320, 240], 20);
        fill(&mut picture, [0, 224, 280, 240], 50);
        assert!(level_up_screen(&picture));
        fill(&mut picture, [280, 0, 320, 240], 120);
        assert!(!level_up_screen(&picture));
        assert!(!level_up_screen(&[]));
    }

    #[test]
    fn the_map_screens_tell_apart() {
        let paint = |regions: &[([usize; 4], u8)]| {
            let mut picture = vec![30; 320 * 240 * 4];
            for &([left, top, right, bottom], value) in regions {
                for y in top..bottom {
                    for x in left..right {
                        picture[(y * 320 + x) * 4..][..3].fill(value);
                    }
                }
            }
            picture
        };
        assert_eq!(map_screen(&paint(&[([225, 22, 300, 38], 140)])), Some(MapScreen::Inventory));
        assert_eq!(map_screen(&paint(&[([292, 198, 318, 226], 110)])), Some(MapScreen::Overview));
        assert_eq!(map_screen(&paint(&[([225, 22, 300, 38], 140), ([292, 198, 318, 226], 110)])), None);
        assert_eq!(map_screen(&paint(&[([290, 105, 318, 185], 200), ([225, 22, 300, 38], 140)])), None);
        assert_eq!(map_screen(&paint(&[([30, 60, 260, 165], 160), ([282, 204, 304, 222], 190)])), Some(MapScreen::Story));
        assert_eq!(map_screen(&paint(&[([30, 60, 260, 165], 160), ([282, 204, 304, 222], 190), ([80, 12, 250, 28], 140)])), None);
        assert_eq!(map_screen(&[]), None);
    }

    #[test]
    fn a_touch_on_the_bottom_screen_keeps_it_through_what_it_opens() {
        let mut kept = None;
        assert!(keeps_bottom_over(false, true, false, false, Scene::WorldMap, &mut kept));
        assert!(!keeps_bottom_over(true, true, true, false, Scene::Plain, &mut kept));
        assert!(keeps_bottom_over(true, true, true, true, Scene::WorldMap, &mut kept));
        assert!(keeps_bottom_over(true, true, true, true, Scene::Plain, &mut kept));
        assert!(keeps_bottom_over(true, true, true, false, Scene::Plain, &mut kept));
        assert!(keeps_bottom_over(true, true, true, true, Scene::WorldMap, &mut kept));
        assert!(!keeps_bottom_over(false, true, false, false, Scene::WorldMap, &mut kept));
        assert!(kept.is_none());
        assert!(!keeps_bottom_over(true, true, false, true, Scene::UnitView, &mut kept));
        assert!(!keeps_bottom_over(true, false, true, true, Scene::Forecast, &mut kept));
    }

    #[test]
    fn the_pointer_follows_the_stick_and_stays_on_the_screen() {
        let start = Instant::now();
        let mut pointer = PadPointer::default();
        pointer.steer([0.05, 0.0], start);
        assert!(!pointer.shown);
        pointer.steer([1.0, 1.0], start + Duration::from_millis(40));
        assert!(pointer.shown);
        assert!(pointer.at[0] > 160.0 && pointer.at[1] < 120.0);
        for step in 2..200 {
            pointer.steer([1.0, 0.0], start + Duration::from_millis(40 * step));
        }
        assert_eq!(pointer.at[0], 319.0);
        pointer.rest();
        assert!(!pointer.shown);
        assert_eq!(pointer.spot().0, 319);
    }

    #[test]
    fn battle_buttons_come_on_any_button_and_stay_while_pressed() {
        use zakuro_core::services::hid::PadState;
        let pieces: &'static [zakuro_gpu::Piece] = &[];
        let start = Instant::now();
        let at = |seconds: f32| start + Duration::from_secs_f32(seconds);
        let step = |shown, pressed, touched, seconds| ShownButtons::next(shown, Some(pieces), pressed, touched, false, at(seconds));
        assert!(step(None, PadState::CIRCLE_LEFT | PadState::C_STICK_UP, true, 0.0).is_none(), "neither the sticks nor a touch brings them");
        let near = |value: f32, wanted: f32| (value - wanted).abs() < 0.01;
        for button in [PadState::A, PadState::B, PadState::X, PadState::L, PadState::START, PadState::DOWN] {
            assert!(near(step(None, button, false, 0.0).unwrap().opacity(at(1.0)), 1.0), "{button:?} brings them");
        }
        let shown = step(None, PadState::B, false, 0.0).unwrap();
        assert!(near(shown.opacity(at(0.05)), 0.5), "coming in");
        assert!(near(shown.opacity(at(2.9)), 1.0));
        assert!(near(shown.opacity(at(3.15)), 0.5), "fading after three seconds");
        assert!(near(shown.opacity(at(3.4)), 0.0));
        let kept = step(Some(shown), PadState::A, false, 2.5).unwrap();
        assert!(near(kept.opacity(at(5.4)), 1.0), "another press keeps them");
        let touched = step(Some(shown), PadState::empty(), true, 2.5).unwrap();
        assert!(near(touched.opacity(at(5.4)), 1.0), "so does a touch while they show");
        let back = step(Some(shown), PadState::A, false, 4.0).unwrap();
        assert!(near(back.opacity(at(4.05)), 0.5), "a press once gone brings them back");
        let any_key = ShownButtons::next(None, Some(pieces), PadState::empty(), false, true, at(0.0)).unwrap();
        assert!(near(any_key.opacity(at(2.9)), 1.0), "so does any key or controller button");
        let gone = ShownButtons::next(Some(shown), None, PadState::B, false, false, at(1.0)).unwrap();
        assert!(near(gone.opacity(at(1.0)), 1.0));
        assert!(near(gone.opacity(at(1.3)), 0.0), "the battle over, they go");
    }
}
