use egui::epaint::{Mesh, Shadow};
use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Pos2, Rect, Response, RichText, Sense, Stroke, TextStyle, Vec2};

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub const PARCHMENT: Color32 = rgb(0xF4, 0xDB, 0xB2);
const PARCHMENT_DEEP: Color32 = rgb(0xEC, 0xCF, 0x9F);
const FRAME: Color32 = rgb(0x95, 0x73, 0x51);
const FRAME_DARK: Color32 = rgb(0x44, 0x33, 0x23);
const BACKDROP: Color32 = rgb(0x9E, 0x8E, 0x6D);
const IVORY: Color32 = rgb(0xF9, 0xF1, 0xD8);
const IVORY_LOW: Color32 = rgb(0xE5, 0xDC, 0xBB);
const CORAL: Color32 = rgb(0xFA, 0x85, 0x6C);
const CORAL_LOW: Color32 = rgb(0xE3, 0x74, 0x5B);
const CORAL_PRESSED: Color32 = rgb(0xD8, 0x5E, 0x45);
const ICON_SQUARE: Color32 = rgb(0x44, 0x22, 0x11);
const BAND: Color32 = rgb(0x1A, 0x12, 0x0A);
const BAND_LINE: Color32 = rgb(0x37, 0x2F, 0x1F);
const GOLD: Color32 = rgb(0xEA, 0xB7, 0x2F);
const CREAM: Color32 = rgb(0xFD, 0xEC, 0xB9);
const RED: Color32 = rgb(0xF9, 0x44, 0x2B);
const BUTTON: Color32 = rgb(0x48, 0x37, 0x1F);
const INK: Color32 = rgb(0x3B, 0x2A, 0x1A);
const INK_FADED: Color32 = rgb(0x7A, 0x62, 0x48);
const CORAL_EDGE: Color32 = rgb(0xB5, 0x50, 0x3C);
const CORAL_PRESSED_EDGE: Color32 = rgb(0x9E, 0x3E, 0x2C);

const TAB_CORNERS: CornerRadius = CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };

const BOLD: &str = "bold";

pub fn bold_face() -> Option<Vec<u8>> {
    std::fs::read(std::path::Path::new("C:/Windows/Fonts").join("cambriab.ttf")).ok()
}

pub fn add_fonts(fonts: &mut egui::FontDefinitions, face: Option<Vec<u8>>) {
    let mut bold = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(face) = face {
        fonts.font_data.insert(BOLD.to_owned(), std::sync::Arc::new(egui::FontData::from_owned(face)));
        bold.insert(0, BOLD.to_owned());
    }
    fonts.families.insert(FontFamily::Name(BOLD.into()), bold);
}

fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

pub fn apply(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, bold(12.0)),
            (TextStyle::Body, bold(15.0)),
            (TextStyle::Button, bold(15.0)),
            (TextStyle::Heading, bold(19.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        let spacing = &mut style.spacing;
        spacing.item_spacing = Vec2::new(8.0, 6.0);
        spacing.button_padding = Vec2::new(10.0, 4.0);
        spacing.interact_size = Vec2::new(40.0, 24.0);
        spacing.window_margin = Margin::same(12);
        spacing.slider_width = 180.0;
        spacing.combo_width = 160.0;

        let visuals = &mut style.visuals;
        visuals.dark_mode = false;
        visuals.override_text_color = None;
        visuals.weak_text_color = Some(INK_FADED);
        visuals.hyperlink_color = rgb(0xA8, 0x45, 0x2E);
        visuals.faint_bg_color = PARCHMENT_DEEP;
        visuals.extreme_bg_color = IVORY;
        visuals.text_edit_bg_color = Some(IVORY);
        visuals.code_bg_color = IVORY_LOW;
        visuals.warn_fg_color = rgb(0xB0, 0x5A, 0x00);
        visuals.error_fg_color = rgb(0xB8, 0x24, 0x18);
        visuals.window_fill = PARCHMENT;
        visuals.window_stroke = Stroke::new(3.0, FRAME);
        visuals.window_corner_radius = CornerRadius::same(8);
        visuals.window_shadow = Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(110) };
        visuals.popup_shadow = Shadow { offset: [0, 3], blur: 8, spread: 0, color: Color32::from_black_alpha(90) };
        visuals.menu_corner_radius = CornerRadius::same(4);
        visuals.panel_fill = BACKDROP;
        visuals.selection.bg_fill = CORAL_LOW;
        visuals.selection.stroke = Stroke::new(1.0, CREAM);
        visuals.slider_trailing_fill = true;
        visuals.striped = true;
        visuals.text_cursor.stroke = Stroke::new(2.0, INK);

        let widgets = &mut visuals.widgets;
        let line = Stroke::new(1.0, FRAME);
        widgets.noninteractive.bg_fill = PARCHMENT;
        widgets.noninteractive.weak_bg_fill = PARCHMENT;
        widgets.noninteractive.bg_stroke = line;
        widgets.noninteractive.fg_stroke = Stroke::new(1.0, INK);
        widgets.noninteractive.corner_radius = CornerRadius::same(3);
        for (state, fill, stroke, text) in [
            (&mut widgets.inactive, IVORY, line, INK),
            (&mut widgets.hovered, CORAL, Stroke::new(1.0, CORAL_EDGE), CREAM),
            (&mut widgets.active, CORAL_PRESSED, Stroke::new(1.0, CORAL_PRESSED_EDGE), CREAM),
            (&mut widgets.open, CORAL, Stroke::new(1.0, CORAL_EDGE), CREAM),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = stroke;
            state.fg_stroke = Stroke::new(1.5, text);
            state.corner_radius = CornerRadius::same(3);
            state.expansion = 0.0;
        }
    });
}

pub fn window(id: &str) -> egui::Window<'static> {
    egui::Window::new(id.to_owned()).id(egui::Id::new(id)).title_bar(false).frame(frame())
}

pub fn frame() -> egui::Frame {
    egui::Frame::window(&egui::Style::default()).fill(PARCHMENT).stroke(Stroke::new(3.0, FRAME)).corner_radius(8).inner_margin(Margin::same(0))
}

pub fn header(ui: &mut egui::Ui, title: &str, open: Option<&mut bool>) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, TAB_CORNERS, BAND);
    painter.hline(rect.x_range(), rect.bottom() - 1.0, Stroke::new(2.0, BAND_LINE));
    painter.text(rect.left_center() + Vec2::new(14.0, 0.0), egui::Align2::LEFT_CENTER, title, bold(19.0), GOLD);
    if let Some(open) = open {
        let button = Rect::from_min_size(Pos2::new(rect.right() - 82.0, rect.top() + 5.0), Vec2::new(72.0, 24.0));
        let response = ui.interact(button, ui.id().with("close"), Sense::click());
        dark_button(ui, button, "Close", &response);
        if response.clicked() {
            *open = false;
        }
    }
}

pub fn tabs<T: PartialEq + Copy>(ui: &mut egui::Ui, shown: &mut T, all: &[(T, &str)]) {
    ui.add_space(-ui.spacing().item_spacing.y);
    let width = ui.available_width();
    let (row, _) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::hover());
    ui.painter().rect_filled(row, 0, BAND);
    let galleys: Vec<_> = all.iter().map(|(_, label)| ui.painter().layout_no_wrap(label.to_string(), bold(15.0), Color32::PLACEHOLDER)).collect();
    let labels_width: f32 = galleys.iter().map(|galley| galley.size().x).sum();
    let (margin, gap, count) = (10.0, 4.0, all.len() as f32);
    let padding = ((width - 2.0 * margin - gap * (count - 1.0) - labels_width) / (2.0 * count)).clamp(6.0, 16.0);
    let mut x = row.left() + margin;
    for (i, ((value, _), galley)) in all.iter().zip(galleys).enumerate() {
        let rect = Rect::from_min_max(Pos2::new(x, row.top() + 6.0), Pos2::new(x + galley.size().x + 2.0 * padding, row.bottom()));
        x = rect.right() + gap;
        let response = ui.interact(rect, ui.id().with(("tab", i)), Sense::click());
        if response.clicked() {
            *shown = *value;
        }
        let painter = ui.painter();
        let (rect, fill, text) = if *shown == *value {
            (rect, PARCHMENT, INK)
        } else if response.hovered() || response.has_focus() {
            (rect.with_min_y(rect.top() + 3.0), CORAL, CREAM)
        } else {
            (rect.with_min_y(rect.top() + 3.0), BUTTON, CREAM)
        };
        painter.rect_filled(rect, TAB_CORNERS, fill);
        painter.galley(rect.center() - galley.size() / 2.0, galley, text);
    }
}

pub fn body<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new().inner_margin(Margin { left: 14, right: 14, top: 10, bottom: 14 }).show(ui, add).inner
}

pub fn menu_body<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new().inner_margin(Margin { left: 26, right: 26, top: 10, bottom: 16 }).show(ui, add).inner
}

pub fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).weak().size(13.0));
}

pub fn command(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool) -> Response {
    let width = ui.available_width().max(200.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), if enabled { Sense::click() } else { Sense::hover() });
    let chosen = enabled && (response.hovered() || response.has_focus());
    let painter = ui.painter();
    let (high, low) = match (enabled, chosen, response.is_pointer_button_down_on()) {
        (true, true, true) => (CORAL_PRESSED, CORAL_PRESSED),
        (true, true, false) => (CORAL, CORAL_LOW),
        _ => (IVORY, IVORY_LOW),
    };
    gradient(painter, rect, high, low);
    painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, FRAME));
    let square = Rect::from_center_size(Pos2::new(rect.left() + 18.0, rect.center().y), Vec2::splat(24.0));
    painter.rect_filled(square, CornerRadius::same(3), ICON_SQUARE);
    painter.text(square.center(), egui::Align2::CENTER_CENTER, icon, bold(15.0), CREAM);
    let text = match (enabled, chosen) {
        (false, _) => INK_FADED,
        (true, true) => CREAM,
        (true, false) => INK,
    };
    painter.text(Pos2::new(square.right() + 10.0, rect.center().y), egui::Align2::LEFT_CENTER, label, bold(16.0), text);
    if chosen {
        glove(painter, Pos2::new(square.left(), rect.center().y));
    }
    response
}

pub fn commands<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .stroke(Stroke::new(2.0, FRAME_DARK))
        .corner_radius(2)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            add(ui)
        })
        .inner
}

fn dark_button(ui: &egui::Ui, rect: Rect, label: &str, response: &Response) {
    let painter = ui.painter();
    let fill = if response.hovered() { RED } else { BUTTON };
    painter.rect(rect, CornerRadius::same(8), fill, Stroke::new(1.5, FRAME), egui::StrokeKind::Inside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, label, bold(14.0), CREAM);
}

fn gradient(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(mesh);
}

fn glove(painter: &egui::Painter, tip: Pos2) {
    let outline = Stroke::new(1.5, FRAME_DARK);
    let finger = Rect::from_min_max(Pos2::new(tip.x - 14.0, tip.y - 7.0), Pos2::new(tip.x, tip.y - 2.0));
    let hand = Rect::from_min_max(Pos2::new(tip.x - 26.0, tip.y - 8.0), Pos2::new(tip.x - 11.0, tip.y + 8.0));
    let cuff = Rect::from_min_max(Pos2::new(tip.x - 31.0, tip.y - 7.0), Pos2::new(tip.x - 25.0, tip.y + 7.0));
    for (shape, radius) in [(cuff, 2), (hand, 5), (finger, 3)] {
        painter.rect(shape, CornerRadius::same(radius), Color32::WHITE, outline, egui::StrokeKind::Middle);
    }
    for y in [tip.y + 1.0, tip.y + 4.5] {
        painter.hline((tip.x - 20.0)..=(tip.x - 12.0), y, Stroke::new(1.0, rgb(0xA8, 0xA0, 0x98)));
    }
}
