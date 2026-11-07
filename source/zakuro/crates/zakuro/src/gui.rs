//! egui, fed the window's events and turned into an overlay the presenters
//! draw over the screens.

use egui::epaint::Primitive;
use egui::{TextureId, ViewportId};
use winit::event::WindowEvent;
use winit::window::Window;

use zakuro_gpu::{Overlay, OverlayMesh, OverlayTexture, OverlayVertex};

pub struct Gui {
    pub ctx: egui::Context,
    state: egui_winit::State,
}

impl Gui {
    pub fn new(window: &Window) -> Gui {
        let ctx = context();
        let state = egui_winit::State::new(
            ctx.clone(),
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(8192),
        );
        Gui { ctx, state }
    }

    /// hands egui an event, true when it wants it for itself.
    pub fn event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    /// whether egui is using the keyboard or the pointer, in which case the
    /// game should not see them.
    pub fn wants_keyboard(&self) -> bool {
        self.ctx.egui_wants_keyboard_input()
    }

    pub fn wants_pointer(&self) -> bool {
        self.ctx.egui_wants_pointer_input()
    }

    /// runs the interface for a frame and returns what to draw.
    pub fn frame(&mut self, window: &Window, ui: impl FnMut(&mut egui::Ui)) -> Overlay {
        let input = self.state.take_egui_input(window);
        let mut output = self.ctx.run_ui(input, ui);
        self.state.handle_platform_output(window, output.platform_output);
        let pixels_per_point = output.pixels_per_point;
        let size = window.inner_size();

        let textures = std::mem::take(&mut output.textures_delta.set)
            .into_iter()
            .flat_map(|(id, deltas)| {
                deltas.into_iter().map(move |delta| {
                    let egui::ImageData::Color(image) = &delta.image;
                    OverlayTexture {
                        id: texture(id),
                        offset: delta.pos.map(|[x, y]| [x as u32, y as u32]),
                        size: [image.width() as u32, image.height() as u32],
                        pixels: image.pixels.iter().flat_map(|color| color.to_array()).collect(),
                        linear: delta.options.magnification == egui::TextureFilter::Linear,
                    }
                })
            })
            .collect();
        let meshes = self
            .ctx
            .tessellate(output.shapes, pixels_per_point)
            .into_iter()
            .filter_map(|clipped| {
                let Primitive::Mesh(mesh) = clipped.primitive else { return None };
                let rect = clipped.clip_rect;
                let pixel = |value: f32, most: u32| ((value * pixels_per_point).round().max(0.0) as u32).min(most);
                Some(OverlayMesh {
                    texture: texture(mesh.texture_id),
                    clip: [
                        pixel(rect.min.x, size.width),
                        pixel(rect.min.y, size.height),
                        pixel(rect.max.x, size.width),
                        pixel(rect.max.y, size.height),
                    ],
                    vertices: mesh
                        .vertices
                        .iter()
                        .map(|vertex| OverlayVertex {
                            position: [vertex.pos.x * pixels_per_point, vertex.pos.y * pixels_per_point],
                            uv: [vertex.uv.x, vertex.uv.y],
                            color: vertex.color.to_array(),
                        })
                        .collect(),
                    indices: mesh.indices,
                })
            })
            .collect();
        let free = std::mem::take(&mut output.textures_delta.free).into_iter().map(texture).collect();
        Overlay { textures, meshes, free }
    }
}

pub const BOTTOM_SCREEN: TextureId = TextureId::User(0);

/// egui's two kinds of texture in one space of ids.
fn texture(id: TextureId) -> u64 {
    match id {
        TextureId::Managed(n) => n * 2,
        TextureId::User(n) => n * 2 + 1,
    }
}

pub fn context() -> egui::Context {
    let ctx = egui::Context::default();
    let mut fonts = fonts(system_fonts());
    crate::theme::add_fonts(&mut fonts, crate::theme::bold_face());
    ctx.set_fonts(fonts);
    crate::theme::apply(&ctx);
    ctx
}

/// egui's own fonts, then extra ones to fall back on for what they lack.
/// game names come in Japanese, Korean and Chinese, whose characters
/// egui's fonts have none of and drew as boxes.
fn fonts(extra: Vec<Vec<u8>>) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    for (i, font) in extra.into_iter().enumerate() {
        let name = format!("system {i}");
        fonts.font_data.insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(font)));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push(name.clone());
        }
    }
    fonts
}

/// the system's fonts for Japanese, Korean and Chinese, the first one
/// there of each kind that has the kind's characters.
fn system_fonts() -> Vec<Vec<u8>> {
    font_candidates()
        .into_iter()
        .filter_map(|(character, paths)| first_drawing(character, paths.iter().filter_map(|path| std::fs::read(path).ok())))
        .collect()
}

/// the first of fonts with a glyph for character. fontconfig names a font
/// even when the system has none for the language asked for.
fn first_drawing(character: char, fonts: impl IntoIterator<Item = Vec<u8>>) -> Option<Vec<u8>> {
    use ab_glyph::Font;
    fonts.into_iter().find(|font| ab_glyph::FontRef::try_from_slice_and_index(font, 0).is_ok_and(|font| font.glyph_id(character).0 != 0))
}

/// where Windows keeps fonts for Japanese, Korean and Chinese, each kind
/// a character of its own and the fonts to try in turn.
#[cfg(windows)]
fn font_candidates() -> Vec<(char, Vec<std::path::PathBuf>)> {
    let folder = std::path::PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts");
    let kinds: [(char, &[&str]); 3] = [('あ', &["YuGothM.ttc", "meiryo.ttc", "msgothic.ttc"]), ('한', &["malgun.ttf"]), ('汉', &["msyh.ttc", "simsun.ttc"])];
    kinds.iter().map(|(character, names)| (*character, names.iter().map(|name| folder.join(name)).collect())).collect()
}

/// where macOS keeps fonts for Japanese, Korean and Chinese.
#[cfg(target_os = "macos")]
fn font_candidates() -> Vec<(char, Vec<std::path::PathBuf>)> {
    let kinds: [(char, &[&str]); 3] = [
        ('あ', &["/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", "/System/Library/Fonts/Hiragino Sans GB.ttc"]),
        ('한', &["/System/Library/Fonts/AppleSDGothicNeo.ttc"]),
        ('汉', &["/System/Library/Fonts/PingFang.ttc"]),
    ];
    kinds.iter().map(|(character, paths)| (*character, paths.iter().map(std::path::PathBuf::from).collect())).collect()
}

/// where Linux distributions put Noto's collection, which has Japanese,
/// Korean and Chinese all in one, or else the font fontconfig picks for
/// Japanese.
#[cfg(all(unix, not(target_os = "macos")))]
fn font_candidates() -> Vec<(char, Vec<std::path::PathBuf>)> {
    let mut paths: Vec<std::path::PathBuf> = [
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ]
    .iter()
    .map(std::path::PathBuf::from)
    .collect();
    if !paths.iter().any(|path| path.exists()) {
        let matched = std::process::Command::new("fc-match").args(["--format=%{file}", ":lang=ja"]).output();
        if let Some(output) = matched.ok().filter(|output| output.status.success()) {
            paths.push(String::from_utf8_lossy(&output.stdout).into_owned().into());
        }
    }
    vec![('あ', paths)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bottom_screen_is_the_presenters_texture() {
        assert_eq!(texture(BOTTOM_SCREEN), zakuro_gpu::BOTTOM_SCREEN_TEXTURE);
    }

    fn paint(ctx: &egui::Context, size: [u32; 2], pointer: Option<[f32; 2]>, run: impl FnMut(&mut egui::Ui)) -> image::RgbaImage {
        paint_with(ctx, size, pointer, Vec::new(), run)
    }

    fn paint_with(ctx: &egui::Context, size: [u32; 2], pointer: Option<[f32; 2]>, own: Vec<(TextureId, egui::ColorImage)>, mut run: impl FnMut(&mut egui::Ui)) -> image::RgbaImage {
        use egui::epaint::{ClippedPrimitive, Primitive};
        use std::collections::HashMap;
        let zoom = 1.5;
        ctx.set_zoom_factor(zoom);
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32) / zoom)),
            events: pointer.map(|[x, y]| egui::Event::PointerMoved(egui::pos2(x, y) / zoom)).into_iter().collect(),
            ..Default::default()
        };
        fn take(textures: &mut HashMap<egui::TextureId, egui::ColorImage>, set: &mut egui::TexturesDelta) {
            for (id, deltas) in std::mem::take(&mut set.set) {
                for delta in deltas {
                    let egui::ImageData::Color(image) = &delta.image;
                    match delta.pos {
                        None => {
                            textures.insert(id, (**image).clone());
                        }
                        Some([x, y]) => {
                            let texture = textures.get_mut(&id).expect("a texture updated before it was made");
                            for row in 0..image.size[1] {
                                for column in 0..image.size[0] {
                                    texture.pixels[(y + row) * texture.size[0] + x + column] = image.pixels[row * image.size[0] + column];
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut textures: HashMap<egui::TextureId, egui::ColorImage> = own.into_iter().collect();
        let mut output = ctx.run_ui(input(), &mut run);
        for _ in 0..2 {
            take(&mut textures, &mut output.textures_delta);
            output = ctx.run_ui(input(), &mut run);
        }
        take(&mut textures, &mut output.textures_delta);
        let ppp = output.pixels_per_point;
        let primitives: Vec<ClippedPrimitive> = ctx.tessellate(output.shapes, ppp);
        let (width, height) = (size[0] as usize, size[1] as usize);
        let mut out = vec![[0x20u8, 0x18, 0x10, 255]; width * height];
        for ClippedPrimitive { clip_rect, primitive } in primitives {
            let Primitive::Mesh(mesh) = primitive else { continue };
            let Some(texture) = textures.get(&mesh.texture_id) else { continue };
            let sample = |u: f32, v: f32| -> [f32; 4] {
                let x = (u * texture.size[0] as f32 - 0.5).clamp(0.0, texture.size[0] as f32 - 1.0);
                let y = (v * texture.size[1] as f32 - 0.5).clamp(0.0, texture.size[1] as f32 - 1.0);
                let (x0, y0) = (x.floor() as usize, y.floor() as usize);
                let (x1, y1) = ((x0 + 1).min(texture.size[0] - 1), (y0 + 1).min(texture.size[1] - 1));
                let (fx, fy) = (x - x0 as f32, y - y0 as f32);
                let texel = |x: usize, y: usize| texture.pixels[y * texture.size[0] + x].to_array().map(f32::from);
                let (a, b, c, d) = (texel(x0, y0), texel(x1, y0), texel(x0, y1), texel(x1, y1));
                std::array::from_fn(|i| (a[i] * (1.0 - fx) + b[i] * fx) * (1.0 - fy) + (c[i] * (1.0 - fx) + d[i] * fx) * fy)
            };
            let clip = [clip_rect.min.x * ppp, clip_rect.min.y * ppp, clip_rect.max.x * ppp, clip_rect.max.y * ppp];
            for triangle in mesh.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|i| &mesh.vertices[triangle[i] as usize]);
                let [pa, pb, pc] = [a, b, c].map(|v| (v.pos.x * ppp, v.pos.y * ppp));
                let area = (pb.0 - pa.0) * (pc.1 - pa.1) - (pb.1 - pa.1) * (pc.0 - pa.0);
                if area.abs() < 1e-6 {
                    continue;
                }
                let left = pa.0.min(pb.0).min(pc.0).max(clip[0]).max(0.0).floor() as usize;
                let right = (pa.0.max(pb.0).max(pc.0).min(clip[2]).min(width as f32).ceil() as usize).min(width);
                let top = pa.1.min(pb.1).min(pc.1).max(clip[1]).max(0.0).floor() as usize;
                let bottom = (pa.1.max(pb.1).max(pc.1).min(clip[3]).min(height as f32).ceil() as usize).min(height);
                for y in top..bottom {
                    for x in left..right {
                        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                        if px < clip[0] || px >= clip[2] || py < clip[1] || py >= clip[3] {
                            continue;
                        }
                        let w0 = ((pb.0 - px) * (pc.1 - py) - (pb.1 - py) * (pc.0 - px)) / area;
                        let w1 = ((pc.0 - px) * (pa.1 - py) - (pc.1 - py) * (pa.0 - px)) / area;
                        let w2 = 1.0 - w0 - w1;
                        if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                            continue;
                        }
                        let u = w0 * a.uv.x + w1 * b.uv.x + w2 * c.uv.x;
                        let v = w0 * a.uv.y + w1 * b.uv.y + w2 * c.uv.y;
                        let color: [f32; 4] = std::array::from_fn(|i| {
                            w0 * a.color.to_array()[i] as f32 + w1 * b.color.to_array()[i] as f32 + w2 * c.color.to_array()[i] as f32
                        });
                        let texel = sample(u, v);
                        let src: [f32; 4] = std::array::from_fn(|i| color[i] * texel[i] / 255.0);
                        let dst = &mut out[y * width + x];
                        let keep = 1.0 - src[3] / 255.0;
                        for i in 0..4 {
                            dst[i] = (src[i] + dst[i] as f32 * keep).round().clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
        }
        image::RgbaImage::from_raw(size[0], size[1], out.into_iter().flatten().collect()).unwrap()
    }

    #[test]
    #[ignore = "draws pictures to look at, ZAKURO_SNAPSHOT=folder cargo test -p zakuro snapshot -- --ignored"]
    fn snapshot_the_menus() {
        let Some(folder) = std::env::var_os("ZAKURO_SNAPSHOT").map(std::path::PathBuf::from) else { return };
        std::fs::create_dir_all(&folder).unwrap();
        let mut menus = crate::menus::Menus::default();
        menus.menu_open = true;
        use crate::menus::Confirm;
        for (name, pointer, confirm) in [
            ("paused.png", None, None),
            ("paused-chosen.png", Some([640.0, 524.0]), None),
            ("paused-quit.png", None, Some(Confirm::Quit)),
        ] {
            menus.confirm = confirm;
            let ctx = crate::gui::context();
            let paused = paint(&ctx, [1280, 960], pointer, |ui| {
                menus.game(ui, "Fire Emblem Awakening", None, true, false, Some("1 Nov, 21:04"));
            });
            paused.save(folder.join(name)).unwrap();
        }
        use crate::menus::SettingsTab;
        for (name, tab, pointer, original) in [
            ("settings-system.png", SettingsTab::System, None, false),
            ("settings-graphics.png", SettingsTab::Graphics, None, false),
            ("settings-graphics-original.png", SettingsTab::Graphics, None, true),
            ("settings-keybinds.png", SettingsTab::Keybinds, None, false),
        ] {
            let ctx = crate::gui::context();
            let mut menus = crate::menus::Menus::default();
            menus.settings_open = true;
            menus.settings_tab = tab;
            let mut settings = crate::settings::Settings::default();
            settings.set_original(original);
            let window = paint(&ctx, [1280, 1100], pointer, |ui| {
                menus.settings(ui.ctx(), &mut settings);
            });
            window.save(folder.join(name)).unwrap();
        }
    }

    #[test]
    #[ignore = "draws pictures to look at, ZAKURO_SNAPSHOT=folder cargo test -p zakuro snapshot -- --ignored"]
    fn snapshot_the_library() {
        let Some(folder) = std::env::var_os("ZAKURO_SNAPSHOT").map(std::path::PathBuf::from) else { return };
        std::fs::create_dir_all(&folder).unwrap();
        let mut library = crate::library::Library::default();
        library.games.push(crate::library::Game {
            path: "awakening.3ds".into(),
            name: "Fire Emblem: Awakening".to_owned(),
            program_id: 0x0004_0000_0009_F100,
            icon: None,
            recompiled: true,
            problem: None,
            update: None,
            dlc: Vec::new(),
            dlp_child: None,
        });
        let mut menus = crate::menus::Menus::default();
        let settings = crate::settings::Settings { game_file: Some("awakening.3ds".into()), ..crate::settings::Settings::default() };
        let ctx = crate::gui::context();
        let picture = paint(&ctx, [1280, 800], None, |ui| {
            menus.library(ui, &library, &settings, &[]);
        });
        picture.save(folder.join("library.png")).unwrap();
    }

    #[test]
    #[ignore = "draws pictures to look at, ZAKURO_SNAPSHOT=folder ZAKURO_BATTLE=folder cargo test -p zakuro snapshot -- --ignored"]
    fn snapshot_the_battle_panels() {
        let (Some(folder), Some(battle)) = (std::env::var_os("ZAKURO_SNAPSHOT"), std::env::var_os("ZAKURO_BATTLE")) else { return };
        let (folder, battle) = (std::path::PathBuf::from(folder), std::path::PathBuf::from(battle));
        std::fs::create_dir_all(&folder).unwrap();
        let bottom = image::open(battle.join("zakuro-bottom.png")).unwrap().to_rgba8();
        let top = image::open(battle.join("zakuro-top.png")).unwrap().to_rgba8();
        let awakening = 0x0004_0000_0009_F100;
        let game_ui = zakuro_core::game_ui::for_game(awakening).unwrap();
        let read = crate::banners::read(bottom.as_raw(), game_ui.battle_banners, &[]).unwrap();
        let (panels, buttons) = (game_ui.battle_panels, game_ui.battle_buttons);
        let size = [1800u32, 1080];
        let window = zakuro_gpu::Viewport { x: 0.0, y: 0.0, width: size[0] as f32, height: size[1] as f32 };
        let bottom_image = egui::ColorImage::from_rgba_unmultiplied([bottom.width() as usize, bottom.height() as usize], bottom.as_raw());
        let whole = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let none: &[zakuro_gpu::Piece] = &[];
        let looks = [("battle-panels", panels, false), ("battle-banners", none, true), ("battle-buttons", none, false)];
        let pictures = looks.into_iter().flat_map(|(name, panels, banners)| [(format!("{name}.png"), [panels, buttons], banners), (format!("{name}-hidden.png"), [panels, none], banners)]);
        for (name, drawn, banners) in pictures {
            let drawn = drawn.map(|pieces| {
                let places = zakuro_gpu::piece_places(window, pieces, &[]);
                let cuts: Vec<Vec<[u16; 3]>> = places
                    .iter()
                    .map(|place| match place.cut {
                        Some(ground) => {
                            let [left, top, width, height] = place.part;
                            let rect = [left * 320.0, top * 240.0, width * 320.0, height * 240.0].map(|value| value.round().max(0.0) as u16);
                            zakuro_gpu::cut_out(bottom.as_raw(), rect, ground)
                        }
                        None => Vec::new(),
                    })
                    .collect();
                (places, cuts)
            });
            let ctx = crate::gui::context();
            let mut textures = crate::overlays::BannerTextures::default();
            let mut background: Option<egui::TextureHandle> = None;
            let picture = paint_with(&ctx, size, None, vec![(BOTTOM_SCREEN, bottom_image.clone())], |ui| {
                let rect = ui.ctx().content_rect();
                let id = background
                    .get_or_insert_with(|| {
                        let image = egui::ColorImage::from_rgba_unmultiplied([top.width() as usize, top.height() as usize], top.as_raw());
                        ui.ctx().load_texture("top screen", image, egui::TextureOptions::LINEAR)
                    })
                    .id();
                ui.ctx().layer_painter(egui::LayerId::background()).image(id, rect, whole, egui::Color32::WHITE);
                if banners {
                    crate::overlays::banners(ui.ctx(), &mut textures, &read, rect, 1.0);
                }
                let ppp = ui.ctx().pixels_per_point();
                for (places, cuts) in &drawn {
                    crate::overlays::pieces(ui.ctx(), places, cuts, crate::PIECE_SHADOW * window.height / 240.0 / ppp, 1.0);
                }
            });
            picture.save(folder.join(name)).unwrap();
        }
        fn side(s: &crate::banners::Side) -> [&crate::banners::Text; 6] {
            [&s.name, &s.hp, &s.weapon, &s.values[0], &s.values[1], &s.values[2]]
        }
        let pieces = side(&read.sides[0]).into_iter().chain(side(&read.sides[1])).chain(read.labels.iter());
        for (n, text) in pieces.enumerate() {
            let [width, height] = text.size;
            let mut out = image::RgbaImage::from_pixel(width.max(1) as u32 * 8, height.max(1) as u32 * 8, image::Rgba([255, 0, 255, 255]));
            for (i, px) in text.rgba.chunks(4).enumerate() {
                let a = px[3] as f32 / 255.0;
                let c = [0, 1, 2].map(|k| (px[k] as f32 * a + [255.0, 0.0, 255.0][k] * (1.0 - a)) as u8);
                for dy in 0..8 {
                    for dx in 0..8 {
                        out.put_pixel((i % width) as u32 * 8 + dx, (i / width) as u32 * 8 + dy, image::Rgba([c[0], c[1], c[2], 255]));
                    }
                }
            }
            out.save(folder.join(format!("piece-{n:02}.png"))).unwrap();
        }
    }
    use egui::epaint::text::{Fonts, TextOptions};

    /// Japanese names have every character to draw with the system's
    /// fonts, where the system has them, and none with egui's alone. only
    /// their Japanese characters are asked about, epaint says no for any
    /// character the font with its replacement glyph has, Hack's letters
    /// for monospace among them.
    #[test]
    fn japanese_names_are_drawn_with_the_system_fonts() {
        let name = "イナズマイレブン・クロノストーン";
        let mut plain = Fonts::new(TextOptions::default(), fonts(Vec::new()));
        assert!(!plain.has_glyphs(&egui::FontId::proportional(14.0), name), "egui's own fonts have no Japanese");
        let found = system_fonts();
        if first_drawing('あ', found.clone()).is_none() {
            eprintln!("no font for Japanese on this system");
            return;
        }
        let mut with = Fonts::new(TextOptions::default(), fonts(found));
        for font in [egui::FontId::proportional(14.0), egui::FontId::monospace(14.0)] {
            assert!(with.has_glyphs(&font, name), "{font:?}");
        }
    }

    /// a font without the characters asked for is passed over, like the
    /// one fontconfig names on a system with no font for Japanese.
    #[test]
    fn fonts_without_japanese_are_passed_over() {
        let latin = egui::FontDefinitions::default().font_data["Ubuntu-Light"].font.to_vec();
        assert_eq!(first_drawing('あ', [latin.clone()]), None);
        assert_eq!(first_drawing('A', [latin.clone()]), Some(latin));
    }
}
