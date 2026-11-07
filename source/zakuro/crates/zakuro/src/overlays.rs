use std::time::{Duration, Instant};

use egui::Color32;
use zakuro_core::services::hid::{InputState, PadState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scene {
    Plain,
    Battle(&'static [zakuro_gpu::Piece]),
    WorldMap,
    UnitView,
    Forecast,
    Pieces(&'static [zakuro_gpu::Piece]),
}

pub(crate) const BOTTOM_OVER_EDGE: f32 = 4.0;
pub(crate) const BOTTOM_OVER_WIDTH: f32 = 0.45;

pub(crate) const PIECE_SHADOW: f32 = 1.5;
pub(crate) const PIECE_FADE: f32 = 0.2;

const BUTTONS_IN: f32 = 0.1;
const BUTTONS_OUT: f32 = 0.3;
const BUTTONS_STAY: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
pub(crate) struct ShownButtons {
    pub(crate) pieces: &'static [zakuro_gpu::Piece],
    since: Instant,
    until: Instant,
}

impl ShownButtons {
    pub(crate) fn opacity(&self, now: Instant) -> f32 {
        let coming = |at: Instant| (at.saturating_duration_since(self.since).as_secs_f32() / BUTTONS_IN).min(1.0);
        if now < self.until {
            coming(now)
        } else {
            (coming(self.until) - now.duration_since(self.until).as_secs_f32() / BUTTONS_OUT).max(0.0)
        }
    }

    pub(crate) fn next(shown: Option<ShownButtons>, pieces: Option<&'static [zakuro_gpu::Piece]>, pressed: zakuro_core::services::hid::PadState, touched: bool, any_button: bool, now: Instant) -> Option<ShownButtons> {
        let Some(pieces) = pieces else {
            return shown.map(|shown| ShownButtons { until: shown.until.min(now), ..shown });
        };
        let opacity = shown.map_or(0.0, |shown| shown.opacity(now));
        use zakuro_core::services::hid::PadState;
        let sticks = PadState::C_STICK_RIGHT | PadState::C_STICK_LEFT | PadState::C_STICK_UP | PadState::C_STICK_DOWN | PadState::CIRCLE_RIGHT | PadState::CIRCLE_LEFT | PadState::CIRCLE_UP | PadState::CIRCLE_DOWN;
        let asked = any_button || !pressed.difference(sticks).is_empty() || (opacity > 0.0 && touched);
        if !asked {
            return shown;
        }
        let since = now.checked_sub(Duration::from_secs_f32(opacity * BUTTONS_IN)).unwrap_or(now);
        Some(ShownButtons { pieces, since, until: now + BUTTONS_STAY })
    }
}

pub(crate) const BOTTOM_OVER_CORNER: f32 = 6.0;

pub(crate) fn bottom_over_input(held: InputState, scene: Scene) -> InputState {
    let buttons = match scene {
        Scene::Plain | Scene::UnitView => held.buttons.intersection(PadState::L),
        _ => PadState::empty(),
    };
    InputState { touch: held.touch, buttons, ..Default::default() }
}

pub fn pieces(ctx: &egui::Context, places: &[zakuro_gpu::PiecePlace], cuts: &[Vec<[u16; 3]>], edge: f32, opacity: f32) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let ppp = ctx.pixels_per_point();
    let map = |place: &zakuro_gpu::PiecePlace, x: f32, y: f32| {
        let [left, top, width, height] = place.part;
        let (u, v) = (x / 320.0, y / 240.0);
        let at = place.viewport;
        let pos = egui::pos2(at.x + (u - left) / width * at.width, at.y + (v - top) / height * at.height) / ppp;
        (pos, egui::pos2(u, v))
    };
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
    for (index, place) in places.iter().enumerate() {
        let [left, top, width, height] = place.part;
        let uv = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height));
        let shade = fade(place.shade);
        let alpha = if place.light { 0 } else { fade(255) };
        let tint = egui::Color32::from_rgba_premultiplied(shade, shade, shade, alpha);
        if !place.outline.is_empty() {
            if places[..index].iter().any(|other| other.outline == place.outline) {
                continue;
            }
            let points: Vec<_> = place.outline.iter().map(|&[x, y]| map(place, x as f32 / 10.0, y as f32 / 10.0)).collect();
            painter.add(egui::Shape::mesh(outline_mesh(crate::gui::BOTTOM_SCREEN, &points, tint, 1.0 / ppp)));
            continue;
        }
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

#[derive(Default)]
pub struct BannerTextures {
    slots: Vec<Option<(crate::banners::Text, egui::TextureHandle)>>,
    noise: Option<egui::TextureHandle>,
}

impl BannerTextures {
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

const TICKS_A_ROW: u32 = 40;
const TICK_PITCH: f32 = 9.0;

const TICKS_CLEAR: f32 = 5.0;

const BANNER_TIP: f32 = 540.0;

const PANEL_SHADOW_DROP: f32 = 3.0;
const PANEL_SHADOW_REACH: f32 = 5.0;
const PANEL_SHADOW_ALPHA: f32 = 0.4;

pub fn banners(ctx: &egui::Context, textures: &mut BannerTextures, read: &crate::banners::Read, top: egui::Rect, opacity: f32) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let opacity = opacity.clamp(0.0, 1.0);
    let color = |[r, g, b]: [u8; 3], alpha: f32| Color32::from_rgba_unmultiplied(r, g, b, (alpha * opacity * 255.0).round() as u8);
    let lighter = |c: [u8; 3], by: u8| c.map(|v| v.saturating_add(by));
    let unit = top.height() / 1080.0 * 0.85;
    let group_top = top.bottom() - (50.0 + 312.0) * unit;
    let white = color([255, 255, 255], 1.0);
    let noise = textures.noise(ctx);
    let fade = Color32::from_white_alpha((opacity * 255.0).round() as u8);
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
        let right = index == 1;
        let at = |x: f32, y: f32| egui::pos2(if right { top.right() - x * unit } else { top.left() + x * unit }, group_top + y * unit);
        let place = |points: &[(f32, f32)]| {
            let mut points: Vec<egui::Pos2> = points.iter().map(|&(x, y)| at(x, y)).collect();
            if right {
                points.reverse();
            }
            points
        };
        let shape = |points: &[(f32, f32)], fill: Color32, stroke: (f32, Color32)| egui::Shape::convex_polygon(place(points), fill, egui::Stroke::new(stroke.0 * unit, stroke.1));
        let diamond = |cx: f32, cy: f32, r: f32| [(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)];
        let stat_columns = if right { [(390.0, 530.0), (230.0, 370.0), (70.0, 210.0)] } else { [(70.0, 210.0), (230.0, 370.0), (390.0, 530.0)] };
        let band = [(84.0, 0.0), (516.0, 0.0), (BANNER_TIP, 27.0), (516.0, 54.0), (84.0, 54.0), (60.0, 27.0)];
        let (bar, inside, width, height, gap) = hp_bar(side.ticks.len());
        let hp = diamond(104.0, 110.0, 48.0);
        let weapon = [(60.0, 166.0), (BANNER_TIP, 166.0), (BANNER_TIP, 212.0), (60.0, 212.0)];
        let boxes = |(a, b): (f32, f32)| ([(a, 230.0), (b, 230.0), (b, 270.0), (a, 270.0)], [(a, 272.0), (b, 272.0), (b, 312.0), (a, 312.0)]);
        let stat_boxes: Vec<_> = stat_columns.iter().map(|&col| boxes(col)).collect();
        let mut outlines: Vec<&[(f32, f32)]> = vec![&band, &bar, &hp, &weapon];
        for (tab, cell) in &stat_boxes {
            outlines.extend([&tab[..], &cell[..]]);
        }
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
        for (row, &[lit, all]) in side.ticks.iter().take(2).enumerate() {
            let y0 = 110.0 - inside / 2.0 + row as f32 * (height + gap);
            for tick in 0..all.min(TICKS_A_ROW) {
                let x0 = 159.0 + tick as f32 * TICK_PITCH;
                let x1 = x0 + width;
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
            painter.add(shape(&diamond(x, 189.0, 6.0), color([240, 240, 240], 1.0), (0.0, Color32::TRANSPARENT)));
        }
        draw_text(&side.weapon, at(300.0, 189.0), 2.4);
        for (stat, (&(a, b), (tab, cell))) in stat_columns.iter().zip(&stat_boxes).enumerate() {
            let label = &read.labels[stat];
            painter.add(shape(tab, color(label.ground, 1.0), (2.0, color([235, 225, 200], 1.0))));
            grain(&place(tab));
            draw_text(label, at((a + b) / 2.0, 250.0), 1.9);
            let value = &side.values[stat];
            painter.add(shape(cell, color(value.ground, 1.0), (2.0, white)));
            grain(&place(cell));
            draw_text(value, at((a + b) / 2.0, 292.0), 2.6);
        }
    }
}

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

fn shadow(edge: f32, color: Color32) -> egui::epaint::Shadow {
    let points = |value: f32| value.round().clamp(0.0, 255.0) as u8;
    egui::epaint::Shadow { offset: [0, 0], blur: points(edge * 1.5), spread: points(edge / 2.0), color }
}

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
