//! the library, the menu over a game and the settings, drawn with egui.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

use egui::{Color32, RichText, Vec2};
use winit::keyboard::KeyCode;
use zakuro_core::services::keyboard::Request;

use crate::library::{Game, Library, ICON_SIZE};
use crate::recompile::{Job, Stage};
use crate::gamepad::button_name;
use crate::theme;
use crate::settings::{BattleUi, Filter, Keys, Language, PadButtons, Renderer, Screens, Settings, Stereo3d, TopShape};

/// The number of slots offered for saving and loading states.
pub const STATE_SLOTS: usize = 5;

/// what the user asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Play(PathBuf),
    /// recompile the library's game at this index.
    Recompile(usize),
    /// Install the mod at this index.
    /// The choice tells whether the optional part of the mod, such as the AI cleanup, is wanted.
    /// No choice means the player has not been asked yet.
    InstallMod(usize, Option<bool>),
    /// Remove the files of the mod at this index.
    UninstallMod(usize),
    /// Turn the mod at this index on or off.
    ModEnabled(usize, bool),
    /// Look for a newer launcher, or install the one found.
    Update,
    /// download a compiler, then recompile the library's game at this
    /// index with it.
    DownloadCompiler(usize),
    /// show the mods folder of the game with this program id.
    Mods(u64),
    CancelRecompile(u64),
    /// Pick the file of the game.
    ChooseGame,
    /// Pick the DLC file of the game.
    ChooseUpdate,
    /// Remove the chosen DLC file.
    ForgetUpdate,
    /// save the game's state to the slot chosen, or load it from there.
    SaveState,
    LoadState,
    /// open a room and join it, join another's, or leave the one joined.
    HostRoom,
    JoinRoom,
    LeaveRoom,
    /// take part in the Download Play offer at this place in the list, or
    /// leave the one taken part in.
    DlpJoin(usize),
    DlpLeave,
    Resume,
    Reset,
    Library,
    Fullscreen,
    /// copy what a report on the game wants.
    CopyReport,
    Quit,
    /// the settings changed, to save and apply.
    Settings,
    /// close the game's keyboard with this text and the button pressed.
    Keyboard(String, usize),
    /// turn the running game's cheat at this index on or off.
    Cheat(usize, bool),
    /// Set the value of the cheat at this index.
    CheatValue(usize, u16),
    /// add a cheat, its name and its code, to the running game.
    AddCheat(String, String),
    /// read the running game's cheats from their file again.
    ReloadCheats,
    /// show the cheats folder.
    CheatsFolder,
}

/// The question of whether to install a mod with its optional part, which needs downloads.
#[derive(Clone, Copy)]
pub struct InstallChoice {
    /// The index of the mod step.
    pub step: usize,
    /// Whether the downloads of the optional part are already on the computer.
    pub downloaded: bool,
}

#[derive(Default)]
pub struct Menus {
    /// the menu over a running game.
    pub menu_open: bool,
    /// the cheats window, and the cheat being written in it.
    pub cheats_open: bool,
    cheat_name: String,
    cheat_code: String,
    pub settings_open: bool,
    /// The tab of the settings that is shown.
    pub settings_tab: SettingsTab,
    /// the binding waiting for a key, by its place in Keys::all_mut.
    pub rebinding: Option<usize>,
    /// the controller binding waiting for a button, by its place in
    /// PadButtons::all_mut.
    pub rebinding_pad: Option<usize>,
    /// something to tell the user, until they close it.
    pub message: Option<String>,
    /// something to tell the user for a moment, over the game, and since
    /// when.
    pub toast: Option<(String, std::time::Instant)>,
    /// the slot states are saved to and loaded from, 1 to 9.
    pub slot: u8,
    /// the multiplayer window, and the password typed in it.
    pub multiplayer_open: bool,
    pub room_password: String,
    /// the icons of the games offered for Download Play.
    dlp_icons: HashMap<u64, egui::TextureHandle>,
    /// a report on the game, to copy from the message it goes with, and
    /// from no other.
    pub report: Option<(String, String)>,
    /// what is being typed into the game's keyboard, and what it asked for.
    typing: Option<(Request, String)>,
    /// the library's game to recompile once the player agrees to download a
    /// compiler for it.
    pub compiler_offer: Option<usize>,
    /// The question to show before installing a mod that has an optional part.
    pub install_choice: Option<InstallChoice>,
    /// The state of the update button in the library, or none when it is hidden.
    pub update: Option<UpdateButton>,
    /// The mods of the game and their states, for the Mods tab.
    pub mods: Vec<ModSwitch>,
    /// The choice waiting for a confirmation in the menu over the game.
    pub(crate) confirm: Option<Confirm>,
    icons: HashMap<PathBuf, egui::TextureHandle>,
    background: Background,
}

/// A choice of the menu over the game that asks for a confirmation first.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Confirm {
    Reset,
    Quit,
    /// Save the state to this slot.
    Save(u8),
    /// Load the state from this slot.
    Load(u8),
}

/// The tabs of the settings window.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum SettingsTab {
    #[default]
    System,
    Graphics,
    Sound,
    Keybinds,
    Controller,
    Mods,
}

/// The tabs of the settings in the order they are shown, with their labels.
const SETTINGS_TABS: [(SettingsTab, &str); 6] = [
    (SettingsTab::System, "System"),
    (SettingsTab::Graphics, "Graphics"),
    (SettingsTab::Mods, "Mods"),
    (SettingsTab::Sound, "Sound"),
    (SettingsTab::Keybinds, "Keybinds"),
    (SettingsTab::Controller, "Controller"),
];

/// A mod of the game, as the Mods tab shows it.
pub struct ModSwitch {
    pub label: &'static str,
    /// The program id of the game the mod belongs to.
    pub program_id: u64,
    /// A description of the mod, shown as hover text.
    pub about: &'static str,
    /// Whether the mod is turned on.
    pub on: bool,
    /// Whether the files of the mod are installed.
    pub installed: bool,
    /// The fraction done and the current step, while the mod is being built.
    pub progress: Option<(f32, &'static str)>,
}

/// the room the player is in, as the multiplayer window shows it.
pub struct RoomView {
    pub name: String,
    /// each member's nickname and the game they play.
    pub members: Vec<(String, String)>,
    /// the port of the room this program holds open, when it is its own.
    pub port: Option<u16>,
    /// Download Play, while no game runs.
    pub download_play: Option<DlpView>,
}

/// the Download Play offers in the room, and where the guest is.
pub struct DlpView {
    pub offers: Vec<OfferView>,
    /// what the guest is doing, and how far along, while it takes part.
    pub doing: Option<(String, Option<f32>)>,
}

pub struct OfferView {
    pub child: u64,
    pub title: String,
    pub host: String,
    /// RGBA, ICON_SIZE square.
    pub icon: Option<Vec<u8>>,
    pub guests: u8,
    pub most: u8,
    /// why the player can't take part, when they can't.
    pub blocked: Option<String>,
}

/// the picture behind the library, decoded away from the interface.
#[derive(Default)]
struct Background {
    /// the file shown or being read.
    path: Option<PathBuf>,
    texture: Option<egui::TextureHandle>,
    loading: Option<Receiver<Result<egui::ColorImage, String>>>,
}

/// the longest side a background is kept at, bigger ones are scaled down.
const BACKGROUND_SIDE: u32 = 2560;

impl Background {
    /// the texture for path, starting to read it when it changed, none until
    /// it is ready or when there is no picture.
    fn get(&mut self, ctx: &egui::Context, path: Option<&Path>) -> Result<Option<&egui::TextureHandle>, String> {
        if self.path.as_deref() != path {
            self.path = path.map(Path::to_owned);
            self.texture = None;
            self.loading = path.map(|path| {
                let (send, receive) = channel();
                let path = path.to_owned();
                std::thread::spawn(move || {
                    let _ = send.send(decode(&path));
                });
                receive
            });
        }
        if let Some(receive) = &self.loading {
            match receive.try_recv() {
                Ok(image) => {
                    self.loading = None;
                    self.texture = Some(ctx.load_texture("library background", image?, egui::TextureOptions::LINEAR));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint(),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.loading = None,
            }
        }
        Ok(self.texture.as_ref())
    }
}

fn decode(path: &Path) -> Result<egui::ColorImage, String> {
    let image = image::open(path).map_err(|error| format!("could not open {}, {error}", path.display()))?;
    let image = if image.width().max(image.height()) > BACKGROUND_SIDE {
        image.resize(BACKGROUND_SIDE, BACKGROUND_SIDE, image::imageops::FilterType::Triangle)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw()))
}

/// the part of a picture that fills an area without stretching it, cutting
/// off what sticks out on either side.
fn cover(area: Vec2, picture: [usize; 2]) -> egui::Rect {
    let (width, height) = (picture[0].max(1) as f32, picture[1].max(1) as f32);
    let area_ratio = area.x / area.y.max(1.0);
    let picture_ratio = width / height;
    if area_ratio > picture_ratio {
        // wider area, the picture's top and bottom are cut
        let shown = picture_ratio / area_ratio;
        egui::Rect::from_min_max(egui::pos2(0.0, (1.0 - shown) / 2.0), egui::pos2(1.0, (1.0 + shown) / 2.0))
    } else {
        let shown = area_ratio / picture_ratio;
        egui::Rect::from_min_max(egui::pos2((1.0 - shown) / 2.0, 0.0), egui::pos2((1.0 + shown) / 2.0, 1.0))
    }
}

impl Menus {
    /// the library, filling the window.
    pub fn library(&mut self, ui: &mut egui::Ui, library: &Library, settings: &Settings, jobs: &[Job]) -> Vec<Action> {
        let mut actions = Vec::new();
        if library.changed {
            self.icons.clear();
        }
        // The bar has the title on the left and the update button on the right.
        egui::Panel::top("library bar").frame(theme::band_frame()).show(ui, |ui| {
            egui::containers::Sides::new().height(32.0).show(
                ui,
                |ui| {
                    theme::band_title(ui, "Zakuro");
                },
                |ui| match &self.update {
                    Some(UpdateButton::Available) => {
                        if theme::band_button(ui, "Update Launcher", true)
                            .on_hover_text("Download the latest launcher from GitHub, put it in place of this one and restart. Your saves, settings and the game you built stay as they are.")
                            .clicked()
                        {
                            actions.push(Action::Update);
                        }
                    }
                    Some(UpdateButton::Idle) => {
                        if theme::band_button(ui, "Update Launcher", false).on_hover_text("Look for a newer launcher on GitHub.").clicked() {
                            actions.push(Action::Update);
                        }
                    }
                    Some(UpdateButton::Installing(fraction)) => {
                        theme::progress(ui, *fraction, &format!("Updating, {:.0}%", fraction * 100.0), 190.0, 28.0);
                    }
                    None => {}
                },
            );
        });
        let background = match self.background.get(ui.ctx(), settings.background.as_deref()) {
            Ok(texture) => texture.map(|texture| (texture.id(), texture.size())),
            Err(error) => {
                self.message = Some(error);
                None
            }
        };
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::BACKDROP_DARK).inner_margin(16)).show(ui, |ui| {
            if let Some((texture, size)) = background {
                let area = ui.clip_rect();
                // The opacity of the background is applied through the alpha of the tint.
                let alpha = (settings.background_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
                ui.painter().image(texture, area, cover(area.size(), size), Color32::from_white_alpha(alpha));
            }
            if settings.game_file.is_none() && settings.games.is_none() {
                // Without a game file the library shows a welcome card a third of the way down.
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 3.0);
                    ui.set_max_width(360.0);
                    theme::frame().show(ui, |ui| {
                        theme::header(ui, "Welcome", None);
                        theme::menu_body(ui, |ui| {
                            ui.label("Choose the game's file to see it here.");
                            ui.add_space(8.0);
                            theme::commands(ui, |ui| {
                                if theme::command(ui, "\u{1F4C2}", "Select game files", true).clicked() {
                                    actions.push(Action::ChooseGame);
                                }
                                if theme::command(ui, "\u{2699}", "Settings", true).clicked() {
                                    self.settings_open = true;
                                }
                            });
                        });
                    });
                });
                return;
            }
            if library.is_scanning() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    theme::on_backdrop(ui, "Looking for games…");
                });
                return;
            }
            if library.games.is_empty() {
                theme::on_backdrop(ui, "No .3ds, .cci, .cxi or .cia files in that folder.");
                ui.add_space(8.0);
                ui.set_max_width(320.0);
                theme::commands(ui, |ui| {
                    if theme::command(ui, "\u{2699}", "Settings", true).clicked() {
                        self.settings_open = true;
                    }
                });
                return;
            }
            let room = ui.available_height();
            egui::ScrollArea::vertical().show(ui, |ui| {
                // A single game is placed a third of the way down, like the welcome card.
                if let [game] = library.games.as_slice() {
                    let job = jobs.iter().find(|job| job.program_id == game.program_id);
                    ui.add_space(((room - card_height(job)) / 3.0).max(0.0));
                }
                for (index, game) in library.games.iter().enumerate() {
                    let job = jobs.iter().find(|job| job.program_id == game.program_id);
                    self.game_card(ui, index, game, job, &mut actions);
                    ui.add_space(16.0);
                }
            });
        });
        actions
    }

    /// Draws the card of a game in the library: its icon, its state, the commands and the progress of its recompile job.
    fn game_card(&mut self, ui: &mut egui::Ui, index: usize, game: &Game, job: Option<&Job>, actions: &mut Vec<Action>) {
        // The settings are opened after the card, because the card borrows the interface.
        let mut open_settings = false;
        // The card is centered, and it is drawn in a child interface so that its frame can fill the width.
        let width = ui.available_width().min(CARD_WIDTH);
        let left = ui.cursor().left() + (ui.available_width() - width) / 2.0;
        let room = egui::Rect::from_min_size(egui::pos2(left, ui.cursor().top()), egui::vec2(width, ui.available_height()));
        let mut card = ui.new_child(egui::UiBuilder::new().max_rect(room).layout(egui::Layout::top_down(egui::Align::Min)));
        theme::frame().show(&mut card, |ui| {
            ui.set_width(width);
            theme::header_with_note(ui, &game.name, &format!("Recompilation {}", crate::update::RELEASE));
            theme::menu_body(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let icon = game.icon.as_ref().map(|pixels| {
                        self.icons
                            .entry(game.path.clone())
                            .or_insert_with(|| {
                                let image = egui::ColorImage::from_rgba_unmultiplied([ICON_SIZE, ICON_SIZE], pixels);
                                ui.ctx().load_texture(game.path.display().to_string(), image, egui::TextureOptions::LINEAR)
                            })
                            .clone()
                    });
                    let icon_size = Vec2::splat(ICON);
                    match icon {
                        Some(texture) => {
                            ui.add(egui::Image::new(&texture).fit_to_exact_size(icon_size).corner_radius(10.0));
                        }
                        None => {
                            ui.allocate_space(icon_size);
                        }
                    }
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        if let Some(problem) = &game.problem {
                            ui.label(RichText::new(problem).size(15.0).color(theme::TAG_PROBLEM));
                            return;
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 14.0;
                            let building = job.is_some_and(|job| !job.stage().finished());
                            let (status, color) = match (building, game.recompiled) {
                                (true, _) => ("recompiling", theme::TAG_MISSING),
                                (false, true) => ("recompiled", theme::TAG_GOOD),
                                (false, false) => ("not recompiled", theme::TAG_MISSING),
                            };
                            ui.label(RichText::new(status).size(15.0).color(color));
                            if let Some(update) = &game.update {
                                let version = zakuro_fs::version_name(update.version);
                                ui.label(RichText::new(format!("update {version}")).size(15.0).color(theme::TAG_MISSING));
                            }
                            let dlc = if game.dlc.is_empty() { theme::TAG_MISSING } else { theme::TAG_GOOD };
                            ui.label(RichText::new("DLC").size(15.0).color(dlc));
                        });
                        ui.add_space(8.0);
                        // The commands take half the card, but never less than 220 points.
                        ui.set_max_width((ui.available_width() / 2.0).max(220.0));
                        theme::commands(ui, |ui| {
                            if theme::primary_command(ui, "\u{25B6}", "Play").clicked() {
                                actions.push(Action::Play(game.path.clone()));
                            }
                        });
                        ui.add_space(10.0);
                        theme::commands(ui, |ui| {
                            if !job.is_some_and(|job| !job.stage().finished()) {
                                let compiler = "It takes 10 to 20 minutes and needs a C compiler, gcc or clang, or Zig, which Zakuro offers to download when the computer has neither.";
                                let label = if game.recompiled { "Recompile again" } else { "Recompile" };
                                let hover = format!("Turn the game's code into native code with 3dsrecomp, which makes it run faster. {compiler} The mods are in Settings, Mods.");
                                if theme::command(ui, "\u{2692}", label, true).on_hover_text(hover).clicked() {
                                    actions.push(Action::Recompile(index));
                                }
                            }
                            if theme::command(ui, "\u{2699}", "Settings", true).clicked() {
                                open_settings = true;
                            }
                            if theme::command(ui, "\u{1F6AA}", "Exit", true).clicked() {
                                actions.push(Action::Quit);
                            }
                        });
                    });
                });
                match job.map(Job::stage) {
                    Some(stage) if !stage.finished() => {
                        let job = job.unwrap();
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            // The bar takes all the room left by the Cancel button.
                            let cancel = Vec2::new(110.0, 32.0);
                            let width = ui.available_width() - cancel.x - ui.spacing().item_spacing.x;
                            progress(ui, job, width, 32.0);
                            if ui.add(egui::Button::new(RichText::new("Cancel").size(15.0)).min_size(cancel)).clicked() {
                                actions.push(Action::CancelRecompile(job.program_id));
                            }
                        });
                    }
                    Some(Stage::Failed(error)) => {
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("Failed: {error}")).size(15.0).color(theme::TAG_PROBLEM));
                    }
                    _ => {}
                }
            });
        });
        // The child interface does not move the cursor, so it is moved past the card here.
        ui.advance_cursor_after_rect(card.min_rect());
        if open_settings {
            self.settings_open = true;
        }
    }

    /// what goes over a running game, the menu when it is open and the
    /// frame rate when asked for.
    #[allow(clippy::too_many_arguments)]
    pub fn game(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        fps: Option<(f32, f32)>,
        recompiled: bool,
        fast: bool,
        saved: &[Option<String>],
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let ctx = ui.ctx().clone();
        if let Some((text, since)) = &self.toast {
            if since.elapsed() < std::time::Duration::from_millis(2500) {
                egui::Area::new(egui::Id::new("toast"))
                    .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 8.0))
                    .interactable(false)
                    .show(&ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(text);
                        });
                    });
                ctx.request_repaint();
            } else {
                self.toast = None;
            }
        }
        if fast {
            egui::Area::new(egui::Id::new("fast forward"))
                .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-8.0, 8.0))
                .interactable(false)
                .show(&ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(RichText::new("» fast forward").monospace());
                    });
                });
        }
        if let Some(fps) = fps {
            // clicks go through to the screen under it
            egui::Area::new(egui::Id::new("fps")).fixed_pos(egui::pos2(8.0, 8.0)).interactable(false).show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(frame_rate(fps)).monospace().size(11.0)).wrap_mode(egui::TextWrapMode::Extend));
                });
            });
        }
        if !self.menu_open {
            self.confirm = None;
            return actions;
        }
        if self.settings_open {
            return actions;
        }
        theme::window("paused")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(&ctx, |ui| {
                ui.set_width(320.0);
                theme::header(ui, name, None);
                theme::menu_body(ui, |ui| {
                    theme::hint(ui, if recompiled { "Running its recompiled code" } else { "Interpreted" });
                    ui.add_space(6.0);
                    theme::commands(ui, |ui| {
                        if theme::command(ui, "\u{23F5}", "Resume", true).clicked() {
                            actions.push(Action::Resume);
                        }
                        if theme::command(ui, "\u{27F3}", "Reset", true).clicked() {
                            self.confirm = Some(Confirm::Reset);
                        }
                        if theme::command(ui, "\u{26F6}", "Fullscreen", true).clicked() {
                            actions.push(Action::Fullscreen);
                        }
                    });
                    ui.add_space(8.0);
                    // The hover text that tells what a slot holds.
                    let describe = |slot: usize| match saved.get(slot - 1).cloned().flatten() {
                        Some(when) => format!("Slot {slot}, {when}."),
                        None => format!("Slot {slot} is empty."),
                    };
                    for (row, action, keys) in [("Save", Action::SaveState, "F5"), ("Load", Action::LoadState, "F8")] {
                        theme::slot_row(ui, row, |ui| {
                            for slot in 1..=STATE_SLOTS {
                                let filled = saved.get(slot - 1).is_some_and(Option::is_some);
                                // An empty slot can be saved to, but not loaded from.
                                let enabled = filled || action == Action::SaveState;
                                let hint = format!("{} {keys} uses the outlined slot while playing.", describe(slot));
                                let response = theme::slot(ui, &slot.to_string(), filled, enabled, self.slot as usize == slot).on_hover_text(hint);
                                if response.clicked() {
                                    self.confirm = Some(if action == Action::SaveState { Confirm::Save(slot as u8) } else { Confirm::Load(slot as u8) });
                                }
                            }
                        });
                    }
                    ui.add_space(8.0);
                    theme::commands(ui, |ui| {
                        if theme::command(ui, "\u{2699}", "Settings", true).clicked() {
                            self.settings_open = true;
                        }
                        if theme::command(ui, "\u{2728}", "Cheats", true).clicked() {
                            self.cheats_open = true;
                        }
                        if theme::command(ui, "\u{1F465}", "Multiplayer", true).clicked() {
                            self.multiplayer_open = true;
                        }
                        if theme::command(ui, "\u{1F4CB}", "Copy info for a report", true).clicked() {
                            actions.push(Action::CopyReport);
                        }
                        if theme::command(ui, "\u{1F3E0}", "Back to the library", true).clicked() {
                            actions.push(Action::Library);
                        }
                        if theme::command(ui, "\u{1F6AA}", "Quit", true).clicked() {
                            self.confirm = Some(Confirm::Quit);
                        }
                    });
                });
            });
        // A confirmation modal for the choice of the menu that needs one.
        if let Some(confirm) = self.confirm {
            let (title, text, icon, label, action, slot) = match confirm {
                Confirm::Reset => ("Reset the game?".to_owned(), "It starts over, and what wasn't saved is lost.".to_owned(), "\u{27F3}", "Reset", Action::Reset, None),
                Confirm::Quit => ("Quit?".to_owned(), "The game closes, and what wasn't saved is lost.".to_owned(), "\u{1F6AA}", "Quit", Action::Quit, None),
                Confirm::Save(slot) => (
                    format!("Save to slot {slot}?"),
                    String::new(),
                    "\u{2193}",
                    "Save",
                    Action::SaveState,
                    Some(slot),
                ),
                Confirm::Load(slot) => (
                    format!("Load slot {slot}?"),
                    String::new(),
                    "\u{2191}",
                    "Load",
                    Action::LoadState,
                    Some(slot),
                ),
            };
            let modal = egui::Modal::new(egui::Id::new("confirm")).frame(theme::frame()).show(&ctx, |ui| {
                ui.set_width(300.0);
                theme::header(ui, &title, None);
                theme::menu_body(ui, |ui| {
                    if !text.is_empty() {
                        theme::hint(ui, &text);
                        ui.add_space(6.0);
                    }
                    theme::commands(ui, |ui| {
                        let cancel = theme::command(ui, "\u{2716}", "Cancel", true);
                        let chosen = theme::command(ui, icon, label, true);
                        // Focus starts on Cancel, so that Enter does not confirm by accident.
                        if !cancel.has_focus() && !chosen.has_focus() {
                            cancel.request_focus();
                        }
                        (cancel.clicked(), chosen.clicked())
                    })
                })
            });
            let (cancelled, confirmed) = modal.inner;
            if confirmed {
                if let Some(slot) = slot {
                    self.slot = slot;
                }
                actions.push(action);
                self.confirm = None;
            } else if cancelled || modal.should_close() {
                self.confirm = None;
            }
        }
        actions
    }

    /// the cheats window over a game, when open, its cheats by name, whether
    pub fn cheats(&mut self, ctx: &egui::Context, cheats: &[(String, bool, String, bool, Option<ModSetting>)], held: bool) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut open = self.cheats_open;
        if open {
            let room = ctx.content_rect().height() - 16.0;
            let (name, code) = (&mut self.cheat_name, &mut self.cheat_code);
            theme::window("cheats")
                .default_width(480.0)
                .min_width(480.0)
                .max_height(room)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    theme::header(ui, "Cheats", Some(&mut open));
                    theme::body(ui, |ui| {
                        if held {
                            theme::hint(ui, "Original mode is on, so these are all off.");
                            ui.add_space(6.0);
                        }
                        ui.add_enabled_ui(!held, |ui| cheat_lists(ui, cheats, &mut actions));
                        ui.add_space(10.0);
                        theme::section(ui, "Add a code");
                        ui.add(egui::TextEdit::singleline(name).hint_text("Name").desired_width(f32::INFINITY));
                        ui.add(egui::TextEdit::multiline(code).hint_text("00000000 00000000").desired_rows(4).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                        ui.add_space(8.0);
                        theme::commands(ui, |ui| {
                            if theme::command(ui, "\u{2714}", "Add", !held && !code.trim().is_empty()).clicked() {
                                actions.push(Action::AddCheat(std::mem::take(name), std::mem::take(code)));
                            }
                            if theme::command(ui, "\u{27F3}", "Reload", !held).on_hover_text("Read the codes again from their file, after changing it.").clicked() {
                                actions.push(Action::ReloadCheats);
                            }
                            if theme::command(ui, "\u{1F4C2}", "Open the cheats folder", true).clicked() {
                                actions.push(Action::CheatsFolder);
                            }
                        });
                    });
                });
        }
        self.cheats_open = open;
        actions
    }

    /// the multiplayer window, when open: the room the player is in, or
    /// joining one and opening one.
    pub fn multiplayer(&mut self, ctx: &egui::Context, settings: &mut Settings, room: Option<&RoomView>) -> Vec<Action> {
        let mut actions = Vec::new();
        let before = settings.clone();
        let mut open = self.multiplayer_open;
        egui::Window::new("Multiplayer").open(&mut open).default_width(380.0).show(ctx, |ui| {
            ui.label(
                RichText::new("Local wireless play with others goes through a room, which one of you opens and the others join over the network.")
                    .weak(),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Nickname");
                ui.add(egui::TextEdit::singleline(&mut settings.nickname).char_limit(10).hint_text("Player"))
                    .on_hover_text("The name others see, and the console's own, which games show. It changes with the next game started.");
            });
            ui.separator();
            match room {
                Some(room) => {
                    ui.label(RichText::new(format!("In the room {}", room.name)).strong());
                    if let Some(port) = room.port {
                        ui.label(format!("Open on port {port}. Others join with this computer's address and that port. Over the internet the router has to let that port through."));
                    }
                    ui.add_space(4.0);
                    for (nickname, game) in &room.members {
                        match game.is_empty() {
                            true => ui.label(nickname),
                            false => ui.label(format!("{nickname}, playing {game}")),
                        };
                    }
                    ui.add_space(4.0);
                    let leave = if room.port.is_some() { "Close the room" } else { "Leave the room" };
                    if ui.button(leave).clicked() {
                        actions.push(Action::LeaveRoom);
                    }
                    if let Some(download_play) = &room.download_play {
                        ui.separator();
                        ui.label(RichText::new("Download Play").strong());
                        match &download_play.doing {
                            Some((doing, progress)) => {
                                ui.label(doing);
                                if let Some(progress) = progress {
                                    ui.add(egui::ProgressBar::new(*progress).show_percentage());
                                }
                                if ui.button("Leave").clicked() {
                                    actions.push(Action::DlpLeave);
                                }
                            }
                            None if download_play.offers.is_empty() => {
                                ui.label(RichText::new("No one in the room offers a game to download. A game offers one from its own Download Play menu.").weak());
                            }
                            None => {
                                for (index, offer) in download_play.offers.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        if let Some(pixels) = &offer.icon {
                                            let texture = self.dlp_icons.entry(offer.child).or_insert_with(|| {
                                                let image = egui::ColorImage::from_rgba_unmultiplied([ICON_SIZE, ICON_SIZE], pixels);
                                                ui.ctx().load_texture(format!("download play {:016X}", offer.child), image, egui::TextureOptions::LINEAR)
                                            });
                                            ui.add(egui::Image::new(&*texture).fit_to_exact_size(Vec2::splat(32.0)));
                                        }
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(&offer.title).strong());
                                            ui.label(RichText::new(format!("from {}, {} of {} joined", offer.host, offer.guests, offer.most)).weak());
                                        });
                                        if ui.add_enabled(offer.blocked.is_none(), egui::Button::new("Join")).clicked() {
                                            actions.push(Action::DlpJoin(index));
                                        }
                                    });
                                    if let Some(why) = &offer.blocked {
                                        ui.label(RichText::new(why).weak());
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    ui.label(RichText::new("Join a room").strong());
                    ui.horizontal(|ui| {
                        ui.label("Address");
                        ui.add(egui::TextEdit::singleline(&mut settings.room_address).hint_text("192.168.0.10:24873"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Password");
                        ui.add(egui::TextEdit::singleline(&mut self.room_password).password(true).hint_text("none"));
                    });
                    if ui.add_enabled(!settings.room_address.trim().is_empty(), egui::Button::new("Join")).clicked() {
                        actions.push(Action::JoinRoom);
                    }
                    ui.separator();
                    ui.label(RichText::new("Open a room").strong());
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(&mut settings.room_name).hint_text("Zakuro"));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Port");
                        ui.add(egui::DragValue::new(&mut settings.room_port).range(1024..=65535));
                    });
                    ui.label(RichText::new("The password above, when there is one, is the room's.").weak());
                    if ui.button("Open").clicked() {
                        actions.push(Action::HostRoom);
                    }
                }
            }
        });
        self.multiplayer_open = open;
        if *settings != before {
            actions.push(Action::Settings);
        }
        actions
    }

    /// the settings window, when open.
    pub fn settings(&mut self, ctx: &egui::Context, settings: &mut Settings) -> Vec<Action> {
        let mut actions = Vec::new();
        let before = settings.clone();
        let mut open = self.settings_open;
        // taller than a screen has room for, so it scrolls, and it can be
        // dragged to any size that fits
        let room = ctx.content_rect().height() - 16.0;
        if open {
            // The Close button of the header sets this to false.
            let mut kept = true;
            theme::window("settings")
                .default_size([560.0, room.min(640.0)])
                .min_width(560.0)
                .max_height(room)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                theme::header(ui, "Settings", Some(&mut kept));
                theme::tabs(ui, &mut self.settings_tab, &SETTINGS_TABS);
                let tab = self.settings_tab;
                // The scroll area leaves room for the header and the tabs, and each tab keeps its own scroll position.
                egui::ScrollArea::vertical().id_salt(tab).max_height(room - 80.0).show(ui, |ui| {
                    ui.set_min_height((room - 80.0).min(520.0));
                    theme::body(ui, |ui| match tab {
                        SettingsTab::System => system_tab(ui, settings, &mut actions),
                        SettingsTab::Graphics => graphics_tab(ui, settings),
                        SettingsTab::Sound => sound_tab(ui, settings),
                        SettingsTab::Keybinds => self.keybinds_tab(ui, settings),
                        SettingsTab::Controller => self.controller_tab(ui, settings),
                        SettingsTab::Mods => mods_tab(ui, &self.mods, &mut actions),
                    })
                });
            });
            if !kept {
                open = false;
            }
        }
        self.settings_open = open;
        if !open {
            self.rebinding = None;
            self.rebinding_pad = None;
        }
        if *settings != before {
            actions.push(Action::Settings);
        }
        actions
    }

    /// The Keybinds tab of the settings.
    /// The bindings are listed two to a row, and a click on a key waits for the next key press.
    fn keybinds_tab(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        theme::commands(ui, |ui| {
            let mut keys = settings.keys.all_mut();
            for (pair, bindings) in keys.chunks_mut(2).enumerate() {
                theme::row(ui, |ui| {
                    for (offset, (name, key)) in bindings.iter_mut().enumerate() {
                        // The place of the binding in `Keys::all_mut`.
                        let i = pair * 2 + offset;
                        theme::cell(ui, 120.0, name);
                        let text = if self.rebinding == Some(i) { "press a key…".to_owned() } else { key_name(**key) };
                        if ui.add_sized([110.0, 24.0], egui::Button::new(text)).clicked() {
                            self.rebinding = Some(i);
                            self.rebinding_pad = None;
                        }
                        ui.add_space(12.0);
                    }
                });
            }
            theme::row(ui, |ui| {
                theme::cell(ui, 224.0, "Bottom screen over the top one");
                let text = if self.rebinding == Some(BOTTOM_OVER) { "press a key…".to_owned() } else { key_name(settings.bottom_over_key) };
                if ui.add_sized([110.0, 24.0], egui::Button::new(text)).clicked() {
                    self.rebinding = Some(BOTTOM_OVER);
                    self.rebinding_pad = None;
                }
            });
        });
        ui.add_space(8.0);
        theme::commands(ui, |ui| {
            if theme::command(ui, "\u{27F2}", "Default controls", true).clicked() {
                settings.keys = Keys::default();
                settings.bottom_over_key = crate::settings::BOTTOM_OVER_KEY;
                self.rebinding = None;
            }
        });
    }

    /// The Controller tab of the settings.
    /// It works like the Keybinds tab, but a click on a button waits for the next button press of the controller.
    fn controller_tab(&mut self, ui: &mut egui::Ui, settings: &mut Settings) {
        theme::commands(ui, |ui| {
            let mut buttons = settings.pad.all_mut();
            for (pair, bindings) in buttons.chunks_mut(2).enumerate() {
                theme::row(ui, |ui| {
                    for (offset, (name, button)) in bindings.iter_mut().enumerate() {
                        // The place of the binding in `PadButtons::all_mut`.
                        let i = pair * 2 + offset;
                        theme::cell(ui, 120.0, name);
                        let text = if self.rebinding_pad == Some(i) { "press a button…" } else { button_name(**button) };
                        if ui.add_sized([110.0, 24.0], egui::Button::new(text)).clicked() {
                            self.rebinding_pad = Some(i);
                            self.rebinding = None;
                        }
                        ui.add_space(12.0);
                    }
                });
            }
            theme::row(ui, |ui| {
                theme::cell(ui, 224.0, "Bottom screen over the top one");
                let text = if self.rebinding_pad == Some(BOTTOM_OVER) { "press a button…" } else { button_name(settings.bottom_over_button) };
                if ui.add_sized([110.0, 24.0], egui::Button::new(text)).clicked() {
                    self.rebinding_pad = Some(BOTTOM_OVER);
                    self.rebinding = None;
                }
            });
        });
        theme::hint(ui, "The circle pad is mapped to the left stick, and the Home button opens the in-game menu.");
        ui.add_space(8.0);
        theme::commands(ui, |ui| {
            if theme::command(ui, "\u{27F2}", "Default controller", true).clicked() {
                settings.pad = PadButtons::default();
                settings.bottom_over_button = crate::settings::BOTTOM_OVER_BUTTON;
                self.rebinding_pad = None;
            }
        });
    }

    /// a message box, when there is something to tell.
    /// the keyboard a game opened to have text typed in, it waits until a
    /// button closes it.
    pub fn keyboard(&mut self, ctx: &egui::Context, request: &Request) -> Option<Action> {
        let fresh = self.typing.as_ref().is_none_or(|(asked, _)| asked != request);
        if fresh {
            self.typing = Some((request.clone(), request.text.clone()));
        }
        let (_, text) = self.typing.as_mut()?;
        let confirm = request.buttons.len() - 1;
        let mut action = None;
        egui::Window::new("Keyboard")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let field = ui.add(
                    egui::TextEdit::singleline(text)
                        .hint_text(&request.hint)
                        .char_limit(request.max_length)
                        .desired_width(260.0),
                );
                if fresh {
                    field.request_focus();
                }
                let valid = request.accepts(text);
                ui.horizontal(|ui| {
                    for (button, label) in request.buttons.iter().enumerate() {
                        if ui.add_enabled(button != confirm || valid, egui::Button::new(label)).clicked() {
                            action = Some(Action::Keyboard(text.clone(), button));
                        }
                    }
                });
                // enter confirms, when the text will do
                if valid && field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    action = Some(Action::Keyboard(text.clone(), confirm));
                }
            });
        if action.is_some() {
            self.typing = None;
        }
        action
    }

    /// asks whether to download a compiler, there being none to recompile
    /// with.
    pub fn compiler_offer(&mut self, ctx: &egui::Context) -> Option<Action> {
        let index = self.compiler_offer?;
        let size = crate::zig::download_size().map_or_else(String::new, |size| format!(", {} MB to download and around 400 MB once unpacked", size / 1_000_000));
        let mut action = None;
        egui::Window::new("Recompile")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Recompiling needs a C compiler, and Zakuro didn't find one on this computer. It can download Zig, a compiler that needs no installing, into its own folder{size}."
                ));
                ui.horizontal(|ui| {
                    if ui.button("Download it and recompile").clicked() {
                        action = Some(Action::DownloadCompiler(index));
                        self.compiler_offer = None;
                    }
                    if ui.button("Not now").clicked() {
                        self.compiler_offer = None;
                    }
                });
            });
        action
    }

    /// Asks whether to install a mod with its optional part, which needs downloads, or without it.
    /// Returns the install action with the choice, and closes the question in every case.
    pub fn install_choice(&mut self, ctx: &egui::Context) -> Option<Action> {
        let InstallChoice { step: index, downloaded } = *self.install_choice.as_ref()?;
        let step = crate::recompile::MOD_STEPS.get(index)?;
        let need = step.needs.first()?;
        let size = need.download.size_text();
        let modal = egui::Modal::new(egui::Id::new("install choice")).frame(theme::frame()).show(ctx, |ui| {
            ui.set_width(420.0);
            theme::header(ui, &format!("Install the {} mod", step.label), None);
            theme::menu_body(ui, |ui| {
                ui.label(RichText::new(need.why).size(15.0));
                ui.add_space(4.0);
                let source = if downloaded { format!("The {} uses {}, already downloaded.", need.name, need.what) } else { format!("The {} needs {}: {size}, downloaded once from the pages of their authors.", need.name, need.what) };
                theme::hint(ui, &source);
                theme::hint(ui, &format!("Without the {}, the {} mod draws over the pictures as the game stores them.", need.name, step.label));
                ui.add_space(6.0);
                theme::commands(ui, |ui| {
                    let cancel = theme::command(ui, "\u{2716}", "Cancel", true);
                    let without = theme::command(ui, "\u{2192}", &format!("Without the {}", need.name), true);
                    let with_label = if downloaded { format!("With the {}", need.name) } else { format!("With the {}, {size}", need.name) };
                    let with = theme::command(ui, "\u{2193}", &with_label, true);
                    // Focus starts on the install with the optional part, so that Enter takes it.
                    if !cancel.has_focus() && !without.has_focus() && !with.has_focus() {
                        with.request_focus();
                    }
                    (cancel.clicked(), without.clicked(), with.clicked())
                })
            })
        });
        let (cancel, without, with) = modal.inner;
        if cancel || without || with || modal.should_close() {
            self.install_choice = None;
        }
        match (without, with) {
            (true, _) => Some(Action::InstallMod(index, Some(false))),
            (_, true) => Some(Action::InstallMod(index, Some(true))),
            _ => None,
        }
    }

    /// Shows the message to the user in a modal, with a button to copy the report that goes with it.
    pub fn message(&mut self, ctx: &egui::Context) {
        let Some(text) = self.message.clone() else { return };
        let report = self.report.as_ref().filter(|(about, _)| *about == text).map(|(_, report)| report.clone());
        let modal = egui::Modal::new(egui::Id::new("message")).frame(theme::frame()).show(ctx, |ui| {
            ui.set_width(360.0);
            theme::header(ui, "Zakuro", None);
            theme::menu_body(ui, |ui| {
                ui.label(RichText::new(text.as_str()).size(15.0));
                ui.add_space(8.0);
                theme::commands(ui, |ui| {
                    if let Some(report) = &report {
                        if theme::command(ui, "\u{1F4CB}", "Copy info for a report", true).clicked() {
                            ui.ctx().copy_text(report.clone());
                        }
                    }
                    theme::command(ui, "\u{2714}", "OK", true).clicked()
                })
            })
        });
        if modal.inner || modal.should_close() {
            self.message = None;
            self.report = None;
        }
    }
}

/// The System tab of the settings: the game and DLC files, the language and the console window.
fn system_tab(ui: &mut egui::Ui, settings: &mut Settings, actions: &mut Vec<Action>) {
    // The name of a chosen file without its folder.
    let name = |file: &Option<std::path::PathBuf>| {
        file.as_ref().and_then(|file| file.file_name()).map(|name| name.to_string_lossy().into_owned())
    };
    theme::commands(ui, |ui| {
        theme::row(ui, |ui| {
            theme::cell(ui, 70.0, "Game");
            if ui.button("Choose…").clicked() {
                actions.push(Action::ChooseGame);
            }
            match name(&settings.game_file) {
                Some(file) => ui.label(file),
                None => ui.label(RichText::new("none chosen").weak()),
            };
        });
        theme::row(ui, |ui| {
            theme::cell(ui, 70.0, "DLC");
            if ui.button("Choose…").clicked() {
                actions.push(Action::ChooseUpdate);
            }
            if settings.update_file.is_some() && ui.button("Remove").clicked() {
                actions.push(Action::ForgetUpdate);
            }
            match name(&settings.update_file) {
                Some(file) => ui.label(file),
                None => ui.label(RichText::new("none chosen").weak()),
            };
        });
    });
    theme::hint(ui, "The game is Fire Emblem Awakening's file (.3ds or .cci). The DLC is its DLC file (.cia), if you have one; the game runs with it from the next time it starts.");
    ui.add_space(8.0);
    theme::commands(ui, |ui| {
        theme::row(ui, |ui| {
            egui::ComboBox::from_label("Language").selected_text(settings.language.name()).show_ui(ui, |ui| {
                for language in Language::IN_THE_GAME {
                    ui.selectable_value(&mut settings.language, language, language.name());
                }
            })
            .response
            .on_hover_text("This is the console's system language. Fire Emblem Awakening shows its text in this language.");
        });
        theme::row(ui, |ui| {
            ui.checkbox(&mut settings.show_console, "Show the console")
                .on_hover_text("Shows a window with Zakuro's log, which helps when reporting a problem. Closing that window closes Zakuro too.");
        });
    });
    theme::hint(ui, "A new language takes effect after a reset, or the next time the game starts.");
}

/// Draws the cheats in two lists: the built in ones, then the codes.
/// Each cheat is a tuple of its name, whether it is on, its notes, whether it is built in, and its adjustable value if it has one.
fn cheat_lists(ui: &mut egui::Ui, cheats: &[(String, bool, String, bool, Option<ModSetting>)], actions: &mut Vec<Action>) {
    // Draws a checkbox for a cheat, with its notes as the hover text.
    let toggle = |ui: &mut egui::Ui, index: usize, name: &str, enabled: bool, notes: &str, actions: &mut Vec<Action>| {
        let mut on = enabled;
        let mut response = ui.checkbox(&mut on, name);
        if !notes.is_empty() {
            response = response.on_hover_text(notes);
        }
        if response.changed() {
            actions.push(Action::Cheat(index, on));
        }
    };
    // The changes of values are collected here and sent after the lists, because the toggle closure borrows the actions.
    let mut values = Vec::new();
    if cheats.iter().any(|cheat| cheat.3) {
        theme::section(ui, "Built in");
        theme::commands(ui, |ui| {
            for (index, (name, enabled, _, _, setting)) in cheats.iter().enumerate().filter(|(_, cheat)| cheat.3) {
                theme::row(ui, |ui| toggle(ui, index, name, *enabled, "", actions));
                // The slider shows only while the cheat is on, indented under it.
                if let (Some(setting), true) = (setting, *enabled) {
                    theme::row(ui, |ui| {
                        ui.add_space(24.0);
                        let mut value = setting.value;
                        let slider = egui::Slider::new(&mut value, setting.min..=setting.max).text(&setting.name).suffix(&setting.unit);
                        if ui.add(slider).changed() {
                            values.push(Action::CheatValue(index, value));
                        }
                    });
                }
            }
        });
        ui.add_space(10.0);
    }
    theme::section(ui, "Codes");
    if !cheats.iter().any(|cheat| !cheat.3) {
        theme::hint(ui, "No codes yet. Add one below, or put the cheat file Citra or Azahar has for this game in the cheats folder.");
    } else {
        egui::ScrollArea::vertical().id_salt("codes").max_height(260.0).show(ui, |ui| {
            theme::commands(ui, |ui| {
                for (index, (name, enabled, notes, _, _)) in cheats.iter().enumerate().filter(|(_, cheat)| !cheat.3) {
                    theme::row(ui, |ui| toggle(ui, index, name, *enabled, notes, actions));
                }
            });
        });
    }
    actions.extend(values);
}

/// The Graphics tab of the settings.
/// Original mode switches off the options that would change the picture of the console.
fn graphics_tab(ui: &mut egui::Ui, settings: &mut Settings) {
    let mut original = settings.original;
    theme::commands(ui, |ui| {
        theme::row(ui, |ui| {
            if ui
                .checkbox(&mut original, "Original mode")
                .on_hover_text("Plays the game as the console does: both screens, the top one 5:3, at the console's resolution with no smoothing and no frame interpolation, without mods, codes, enhancements or anything added over the game's screens. Your own choices come back when you turn it off.")
                .changed()
            {
                settings.set_original(original);
            }
        });
    });
    ui.add_space(8.0);
    // Options that Original mode locks are disabled when `free` is false.
    let free = !settings.original;
    // Frame interpolation works only with the Vulkan presenter.
    let interpolates = settings.renderer == Renderer::Vulkan;
    theme::commands(ui, |ui| {
        theme::row(ui, |ui| {
            egui::ComboBox::from_label("Presented with")
                .selected_text(match settings.renderer {
                    Renderer::Vulkan => "Vulkan",
                    Renderer::OpenGl => "OpenGL",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut settings.renderer, Renderer::Vulkan, "Vulkan");
                    ui.selectable_value(&mut settings.renderer, Renderer::OpenGl, "OpenGL");
                });
        });
        theme::row(ui, |ui| {
            ui.add_enabled(free, egui::Slider::new(&mut settings.resolution, 1..=8).text("Resolution").suffix("x"))
                .on_hover_text("Sets how many times the console's resolution the 3D is drawn at. Higher values look sharper but use much more GPU power.");
        });
        theme::row(ui, |ui| {
            ui.add_enabled(free, egui::Checkbox::new(&mut settings.texture_packs, "Texture packs"))
                .on_hover_text("Uses a texture pack made for Fire Emblem Awakening on Citra or Azahar. Put the pack in the textures folder inside the game's mods folder.");
        });
        theme::row(ui, |ui| {
            ui.add_enabled(free && interpolates, egui::Checkbox::new(&mut settings.frame_interpolation, "Frame interpolation"))
                .on_hover_text("Adds a generated frame between each pair of frames, so the battles and cutscenes, which run at 30 frames per second, are shown at 60. This adds one frame of delay. It requires the Vulkan presenter.");
        });
        theme::row(ui, |ui| {
            ui.add_enabled_ui(free, |ui| {
                egui::ComboBox::from_label("Battles UI style").selected_text(settings.battle_ui.name()).show_ui(ui, |ui| {
                    for choice in BattleUi::ALL {
                        ui.selectable_value(&mut settings.battle_ui, choice, choice.name());
                    }
                })
            })
            .inner
            .response
            .on_hover_text("With the top screen alone, what shows over a battle: the game's own panels from the bottom screen, the same information in the style of Fire Emblem: Three Houses, or nothing at all.");
        });
        theme::row(ui, |ui| {
            ui.checkbox(&mut settings.fullscreen, "Start games in fullscreen")
                .on_hover_text("Goes fullscreen when a game starts. F11 or Fullscreen in the menu switches back and forth while playing.");
        });
        theme::row(ui, |ui| {
            ui.checkbox(&mut settings.show_fps, "Show the frame rate")
                .on_hover_text("Shows the frame rate in a corner of the screen: the frames the game draws per second, then the frames shown per second.");
        });
    });
    ui.add_space(8.0);
    ui.add_enabled_ui(free, |ui| {
        theme::commands(ui, |ui| {
            theme::row(ui, |ui| {
                egui::ComboBox::from_label("Screens").selected_text(settings.layout.name()).show_ui(ui, |ui| {
                    for layout in Screens::ALL {
                        ui.selectable_value(&mut settings.layout, layout, layout.name());
                    }
                });
            });
            theme::row(ui, |ui| {
                egui::ComboBox::from_label("Filter")
                    .selected_text(settings.filter.name())
                    .show_ui(ui, |ui| {
                        for filter in Filter::ALL {
                            ui.selectable_value(&mut settings.filter, filter, filter.name());
                        }
                    })
                    .response
                    .on_hover_text("Sets how the screens look when scaled up: smooth, sharp with soft edges, or sharp square pixels.");
            });
            theme::row(ui, |ui| {
                egui::ComboBox::from_label("Top screen shape")
                    .selected_text(settings.top_shape.name())
                    .show_ui(ui, |ui| {
                        for shape in TopShape::ALL {
                            ui.selectable_value(&mut settings.top_shape, shape, shape.name());
                        }
                    })
                    .response
                    .on_hover_text("Shows the top screen wider than on the console. Use 16:9 with the game's 16:9 mod turned on in Cheats, otherwise the picture is stretched.");
            });
        });
    });
    ui.add_space(8.0);
    // The depth and the eye swap matter only when 3D is on.
    let stereo = settings.stereo != Stereo3d::Off;
    theme::commands(ui, |ui| {
        theme::row(ui, |ui| {
            egui::ComboBox::from_label("3D")
                .selected_text(settings.stereo.name())
                .show_ui(ui, |ui| {
                    for stereo in Stereo3d::ALL {
                        ui.selectable_value(&mut settings.stereo, stereo, stereo.name());
                    }
                })
                .response
                .on_hover_text("Shows the top screen in 3D. Red and cyan is for anaglyph glasses, side by side is for 3D TVs and headsets, and alternate rows is for passive 3D monitors. When off, the game is drawn flat.");
        });
        theme::row(ui, |ui| {
            ui.add_enabled(stereo, egui::Slider::new(&mut settings.depth_3d, 0..=100).text("Depth").suffix("%"))
                .on_hover_text("Works like the console's 3D slider and sets how deep the 3D looks.");
        });
        theme::row(ui, |ui| {
            ui.add_enabled(stereo, egui::Checkbox::new(&mut settings.swap_eyes, "Swap eyes"))
                .on_hover_text("Swaps the left and right pictures. Use this for cross-eyed viewing, or if the 3D looks inverted.");
        });
    });
    theme::hint(
        ui,
        "Press F9 while playing to switch between screen layouts, and F10 to switch between the top and bottom screen alone. When the top screen is shown alone, the key set in Keybinds shows the bottom screen in the middle of it, and you can use the mouse on it. The game's buttons wait until the same key, or the game's B button, closes it.",
    );
    theme::hint(ui, "Resolution changes apply right away. A change of presenter applies the next time Zakuro starts.");
}

/// The Mods tab of the settings: one entry per mod, in the state it is in.
/// A mod that is being built shows its progress, an installed mod shows its switch, and a missing mod offers to install.
fn mods_tab(ui: &mut egui::Ui, mods: &[ModSwitch], actions: &mut Vec<Action>) {
    for (index, switch) in mods.iter().enumerate() {
        match (switch.progress, switch.installed) {
            (Some((fraction, doing)), _) => {
                ui.horizontal(|ui| {
                    // The bar takes the room left by the Cancel button, up to a limit.
                    let cancel = Vec2::new(110.0, 30.0);
                    let width = (ui.available_width() - cancel.x - ui.spacing().item_spacing.x).min(420.0);
                    theme::progress(ui, fraction, &format!("{:.0}%", fraction * 100.0), width, 30.0);
                    if ui.add(egui::Button::new(RichText::new("Cancel").size(15.0)).min_size(cancel)).clicked() {
                        actions.push(Action::CancelRecompile(switch.program_id));
                    }
                });
                if !doing.is_empty() {
                    theme::hint(ui, doing);
                }
            }
            (None, true) => {
                theme::commands(ui, |ui| {
                    theme::row(ui, |ui| {
                        let mut on = switch.on;
                        if ui.checkbox(&mut on, switch.label).on_hover_text(switch.about).changed() {
                            actions.push(Action::ModEnabled(index, on));
                        }
                        if ui.button("Rebuild mod").on_hover_text(format!("Build the {} mod again from your copy of the game.", switch.label)).clicked() {
                            actions.push(Action::InstallMod(index, None));
                        }
                        if ui.button("Uninstall").on_hover_text(format!("Remove the {} mod's files. Install it again any time.", switch.label)).clicked() {
                            actions.push(Action::UninstallMod(index));
                        }
                    });
                });
            }
            (None, false) => {
                theme::commands(ui, |ui| {
                    if theme::command(ui, "\u{2193}", &format!("Install the {} mod", switch.label), true).on_hover_text(switch.about).clicked() {
                        actions.push(Action::InstallMod(index, None));
                    }
                });
            }
        }
        ui.add_space(6.0);
    }
    // Every mod belongs to the same game, so the first one gives the program id of the folder.
    if let Some(switch) = mods.first() {
        ui.add_space(6.0);
        theme::commands(ui, |ui| {
            if theme::command(ui, "\u{1F4C2}", "Open mods folder", true)
                .on_hover_text("Open the game's mods folder. Files put in its romfs folder replace the game's, the way Luma3DS and Citra take mods, and a texture pack made for Citra or Azahar goes in its textures folder.")
                .clicked()
            {
                actions.push(Action::Mods(switch.program_id));
            }
        });
    }
}

/// The Sound tab of the settings: volume and mute.
fn sound_tab(ui: &mut egui::Ui, settings: &mut Settings) {
    theme::commands(ui, |ui| {
        theme::row(ui, |ui| {
            ui.add(egui::Slider::new(&mut settings.volume, 0.0..=1.0).text("Volume").show_value(false));
        });
        theme::row(ui, |ui| {
            ui.checkbox(&mut settings.mute, "Mute");
        });
    });
}

/// a line a GameOverlay keeps over the game, in the bottom right corner.
pub fn corner(ctx: &egui::Context, text: &str) {
    egui::Area::new(egui::Id::new("overlay corner"))
        .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-8.0, -8.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(text);
            });
        });
}

/// The side of the icon of a game on its card.
const ICON: f32 = 150.0;

/// The widest a game card gets.
const CARD_WIDTH: f32 = 720.0;

/// Estimates the height of a game card, so that a single card can be placed a third of the way down the library.
/// The terms add up the header, the body margins, the status line, the Play command, the other commands and the line below them.
fn card_height(job: Option<&Job>) -> f32 {
    // The line under the commands is a progress bar while a job runs, or the error of a failed job.
    let below = match job.map(Job::stage) {
        Some(stage) if !stage.finished() => 46.0,
        Some(Stage::Failed(_)) => 30.0,
        _ => 0.0,
    };
    34.0 + 26.0 + 30.0 + 42.0 + 10.0 + 4.0 * 32.0 + below + 10.0
}

/// Draws the progress bar of a recompile job, with the current step and the time elapsed as its text.
fn progress(ui: &mut egui::Ui, job: &Job, width: f32, height: f32) {
    let stage = job.stage();
    let text = match &stage {
        Stage::Mods { .. } => "Preparing the mod".to_owned(),
        Stage::Downloading(arrived) => format!("Downloading the compiler, {:.0}%", arrived * 100.0),
        Stage::Generating => "Converting the game's code to C".to_owned(),
        Stage::Compiling { done, total } => format!("Compiling, {done} of {total} files"),
        Stage::Installing => "Installing".to_owned(),
        Stage::Done => "Done".to_owned(),
        Stage::Failed(error) => format!("Failed: {error}"),
    };
    let minutes = job.elapsed().as_secs() / 60;
    let seconds = job.elapsed().as_secs() % 60;
    theme::progress(ui, stage.fraction(), &format!("{text}, {minutes}:{seconds:02}"), width, height);
}

/// The place used in `rebinding` and `rebinding_pad` for the binding of the bottom screen over the top one.
/// It is outside the places of `Keys::all_mut` and `PadButtons::all_mut`.
pub const BOTTOM_OVER: usize = usize::MAX;

/// The state of the update button in the bar of the library.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateButton {
    /// No newer launcher is known, and the button looks for one.
    Idle,
    /// A newer launcher exists, and the button installs it.
    Available,
    /// The update is being installed, with the fraction done.
    Installing(f32),
}

/// Formats the frame rate for the corner of the screen.
/// `own` is the frames per second the game draws, and `between` is the generated frames per second added in between.
fn frame_rate((own, between): (f32, f32)) -> String {
    // Below half a frame per second the generated frames are not worth showing.
    if between >= 0.5 {
        format!("{own:.0}/{:.0} FPS", own + between)
    } else {
        format!("{own:.0} FPS")
    }
}

/// A cheat of the game that has a value to adjust with a slider.
pub struct ModSetting {
    pub name: String,
    /// The lowest value the slider allows.
    pub min: u16,
    /// The highest value the slider allows.
    pub max: u16,
    /// The value the cheat has now.
    pub value: u16,
    /// The text shown after the value, such as a percent sign.
    pub unit: String,
}

/// a key as a person would call it.
pub fn key_name(key: KeyCode) -> String {
    let name = format!("{key:?}");
    for prefix in ["Key", "Digit", "Arrow"] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return rest.to_owned();
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_background_fills_the_area_without_stretching() {
        // a square picture in a wide area loses its top and bottom
        let wide = cover(Vec2::new(200.0, 100.0), [100, 100]);
        assert_eq!((wide.min.x, wide.max.x), (0.0, 1.0));
        assert_eq!((wide.min.y, wide.max.y), (0.25, 0.75));
        // and in a tall one its sides
        let tall = cover(Vec2::new(100.0, 200.0), [100, 100]);
        assert_eq!((tall.min.x, tall.max.x), (0.25, 0.75));
        assert_eq!((tall.min.y, tall.max.y), (0.0, 1.0));
    }

    #[test]
    fn big_pictures_are_scaled_down_keeping_their_shape() {
        let path = std::env::temp_dir().join(format!("zakuro-background-{}.png", std::process::id()));
        image::RgbaImage::from_pixel(BACKGROUND_SIDE * 2, BACKGROUND_SIDE, image::Rgba([10, 20, 30, 255])).save(&path).unwrap();
        let decoded = decode(&path);
        std::fs::remove_file(&path).ok();
        let decoded = decoded.unwrap();
        assert_eq!(decoded.size, [BACKGROUND_SIDE as usize, BACKGROUND_SIDE as usize / 2]);
        assert_eq!(decoded.pixels[0].to_array(), [10, 20, 30, 255]);
        assert!(decode(Path::new("/nowhere/at/all.png")).is_err());
    }

    /// the settings stay inside a short screen, scrolling in it instead of
    /// running past its bottom.
    #[test]
    fn the_settings_fit_a_short_screen() {
        let ctx = crate::gui::context();
        let mut menus = Menus { settings_open: true, ..Menus::default() };
        let mut settings = Settings::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 360.0));
        for _ in 0..3 {
            let input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
            ctx.run_ui(input, |ui| {
                menus.settings(ui.ctx(), &mut settings);
            })
            .drop_without_applying_deltas();
        }
        let window = ctx.memory(|memory| memory.area_rect(egui::Id::new("settings"))).unwrap();
        assert!(screen.contains_rect(window), "{window:?} in {screen:?}");
    }

    /// The generated frames are added to the frames the game draws.
    #[test]
    fn the_frame_rate_counts_the_pictures_shown() {
        assert_eq!(frame_rate((60.0, 0.0)), "60 FPS", "nothing in between");
        assert_eq!(frame_rate((29.8, 29.6)), "30/59 FPS");
    }

    #[test]
    fn keys_have_short_names() {
        assert_eq!(key_name(KeyCode::KeyX), "X");
        assert_eq!(key_name(KeyCode::ArrowUp), "Up");
        assert_eq!(key_name(KeyCode::Digit1), "1");
        assert_eq!(key_name(KeyCode::Enter), "Enter");
    }
}
