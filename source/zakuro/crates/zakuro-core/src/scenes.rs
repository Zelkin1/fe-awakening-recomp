//! Tracks which screens of the game are showing, from the markers that the renderer reports each frame.
//!
//! It also follows whether a unit is selected on the map, from the button A and the pictures of the game.

use std::collections::BTreeMap;

// The ids of the markers, which `game_ui` ties to draws of the game.
/// The cursor over a unit on the map.
pub const ON_UNIT: u32 = 1;
/// A conversation.
pub const TALKING: u32 = 2;
/// The world map.
pub const WORLD_MAP: u32 = 3;
/// The forecast of a battle.
pub const FORECAST: u32 = 4;
/// The battle.
pub const BATTLE: u32 = 5;
/// The short forecast of a battle.
pub const BRIEF_FORECAST: u32 = 6;
/// The title screen.
pub const TITLE: u32 = 7;
/// The experience gauge shown after a battle.
pub const EXP: u32 = 8;
/// The banner on the left side of a battle.
pub const LEFT_BANNER: u32 = 9;
/// The banner on the right side of a battle.
pub const RIGHT_BANNER: u32 = 10;
/// The view of a unit.
pub const UNIT_VIEW: u32 = 11;

/// The number of frames for which a marker still counts as shown after its last draw.
const RECENT_FRAMES: u64 = 4;
/// The number of frames by which the spans of the markers are read behind the current frame.
const SPAN_LAG_FRAMES: u64 = 3;
/// The mask that keeps the last 16 frames of the picture history.
const PICTURE_HISTORY_MASK: u32 = 0xFFFF;
/// The most frames with a new picture, out of 16, that still count as a game that runs at half rate.
const MOST_PICTURES_AT_HALF_RATE: u32 = 10;

/// The state of the screens, updated at the end of each frame.
#[derive(Default)]
pub(crate) struct Scenes {
    /// The last frame in which each marker was drawn.
    last_drawn: BTreeMap<u32, u64>,
    /// Whether the battle on the screen has ended, which is the case once the experience gauge is drawn.
    battle_over: bool,
    /// The areas of the markers with the frame in which each was seen, kept for a few frames.
    spans: Vec<(u32, [f32; 4], u64)>,
    /// The selection of a unit on the map.
    selection: UnitSelection,
}

impl Scenes {
    /// Records the markers and the areas drawn in the frame, and updates the battle state and the selection.
    pub(crate) fn end_frame(&mut self, frame: u64, drawn_markers: &[u32], new_spans: Vec<(u32, [f32; 4])>, a_held: bool, pictures_from_game: u64) {
        for &id in drawn_markers {
            self.last_drawn.insert(id, frame);
        }
        // The battle is over once the experience gauge is drawn while the battle is still shown, and stays over until the battle goes away.
        self.battle_over = self.shown(BATTLE, frame) && (self.battle_over || drawn_markers.contains(&EXP));
        self.spans.extend(new_spans.into_iter().map(|(id, span)| (id, span, frame)));
        // Older areas are dropped, as no frame reads them any more.
        self.spans.retain(|&(.., at)| frame - at <= 4 * SPAN_LAG_FRAMES);
        let cursor_on_unit = self.shown(ON_UNIT, frame);
        self.selection.update(a_held, pictures_from_game, cursor_on_unit);
    }

    /// Whether the marker was drawn within the last few frames.
    pub(crate) fn shown(&self, marker: u32, frame: u64) -> bool {
        self.last_drawn.get(&marker).is_some_and(|&at| frame.saturating_sub(at) <= RECENT_FRAMES)
    }

    /// Whether a battle is on the screen, which is not the case during a conversation or after the battle has ended.
    pub(crate) fn battle_shows(&self, frame: u64) -> bool {
        self.shown(BATTLE, frame) && !self.shown(TALKING, frame) && !self.battle_over
    }

    /// Whether a unit is selected on the map.
    pub(crate) fn unit_selected(&self) -> bool {
        self.selection.state != SelectionState::NotSelected
    }

    /// The latest area of each marker, read a few frames behind the current one.
    pub(crate) fn marker_spans(&self, frame: u64) -> Vec<(u32, [f32; 4])> {
        let shown_frame = frame.saturating_sub(SPAN_LAG_FRAMES);
        let mut latest: Vec<(u32, [f32; 4], u64)> = Vec::new();
        for &(id, span, at) in self.spans.iter().filter(|&&(.., at)| at <= shown_frame && shown_frame - at <= SPAN_LAG_FRAMES) {
            match latest.iter_mut().find(|(other, ..)| *other == id) {
                Some(entry) if entry.2 <= at => *entry = (id, span, at),
                Some(_) => {}
                None => latest.push((id, span, at)),
            }
        }
        latest.into_iter().map(|(id, span, _)| (id, span)).collect()
    }
}

/// Whether a unit is selected on the map.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SelectionState {
    /// No unit is selected.
    #[default]
    NotSelected,
    /// A unit is selected and the cursor is on a unit.
    OnUnit,
    /// A unit is selected and the cursor has moved off the units.
    CursorAway,
}

/// Follows the selection of a unit from the button A, the cursor and the pictures of the game.
#[derive(Default)]
struct UnitSelection {
    /// The current state.
    state: SelectionState,
    /// Whether the button A was held in the previous frame, to find the frame in which it is pressed.
    a_was_held: bool,
    /// One bit per frame, the lowest for the latest, set when the game produced a new picture in that frame.
    picture_history: u32,
    /// The number of pictures of the game at the previous frame.
    last_pictures_from_game: u64,
}

impl UnitSelection {
    /// Moves the state on by one frame.
    fn update(&mut self, a_held: bool, pictures_from_game: u64, cursor_on_unit: bool) {
        let a_pressed = a_held && !self.a_was_held;
        self.a_was_held = a_held;
        let new_picture = pictures_from_game > self.last_pictures_from_game;
        self.picture_history = (self.picture_history << 1) | new_picture as u32;
        self.last_pictures_from_game = pictures_from_game;
        // A game that makes a new picture in only some of the last 16 frames runs at half rate, as in a battle, and no unit is selected then.
        let half_rate = (self.picture_history & PICTURE_HISTORY_MASK).count_ones() <= MOST_PICTURES_AT_HALF_RATE;
        self.state = match self.state {
            _ if half_rate => SelectionState::NotSelected,
            SelectionState::NotSelected if a_pressed && cursor_on_unit => SelectionState::OnUnit,
            SelectionState::NotSelected => SelectionState::NotSelected,
            SelectionState::CursorAway if cursor_on_unit => SelectionState::NotSelected,
            SelectionState::CursorAway => SelectionState::CursorAway,
            SelectionState::OnUnit if cursor_on_unit => SelectionState::OnUnit,
            SelectionState::OnUnit => SelectionState::CursorAway,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the frames in which the game makes a new picture every time.
    fn full_rate(selection: &mut UnitSelection, frames: u64, a_held: bool, cursor_on_unit: bool) {
        for _ in 0..frames {
            let pictures = selection.last_pictures_from_game + 1;
            selection.update(a_held, pictures, cursor_on_unit);
        }
    }

    /// A selection starts with the button A on a unit, and ends when the cursor comes back to a unit after it was away.
    #[test]
    fn a_selection_lasts_until_the_cursor_comes_back_to_a_unit() {
        let mut selection = UnitSelection::default();
        full_rate(&mut selection, 16, false, true);
        assert_eq!(selection.state, SelectionState::NotSelected);
        full_rate(&mut selection, 1, true, true);
        assert_eq!(selection.state, SelectionState::OnUnit);
        full_rate(&mut selection, 3, false, false);
        assert_eq!(selection.state, SelectionState::CursorAway);
        full_rate(&mut selection, 1, false, true);
        assert_eq!(selection.state, SelectionState::NotSelected);
    }

    /// A game that runs at half rate resets the selection.
    #[test]
    fn a_battle_at_half_rate_ends_the_selection() {
        let mut selection = UnitSelection::default();
        full_rate(&mut selection, 16, false, true);
        full_rate(&mut selection, 1, true, true);
        for frame in 0..16 {
            let pictures = selection.last_pictures_from_game + frame % 2;
            selection.update(false, pictures, true);
        }
        assert_eq!(selection.state, SelectionState::NotSelected);
    }

    /// A marker counts as shown for the four frames after its last draw.
    #[test]
    fn a_marker_shows_for_four_frames_after_its_last_draw() {
        let mut scenes = Scenes::default();
        scenes.end_frame(10, &[TALKING], Vec::new(), false, 10);
        assert!(scenes.shown(TALKING, 14));
        assert!(!scenes.shown(TALKING, 15));
        assert!(!scenes.shown(BATTLE, 10));
    }
}
