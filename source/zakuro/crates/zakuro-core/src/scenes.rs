use std::collections::BTreeMap;

pub const ON_UNIT: u32 = 1;
pub const TALKING: u32 = 2;
pub const WORLD_MAP: u32 = 3;
pub const FORECAST: u32 = 4;
pub const BATTLE: u32 = 5;
pub const BRIEF_FORECAST: u32 = 6;
pub const TITLE: u32 = 7;
pub const EXP: u32 = 8;
pub const LEFT_BANNER: u32 = 9;
pub const RIGHT_BANNER: u32 = 10;
pub const UNIT_VIEW: u32 = 11;

const RECENT_FRAMES: u64 = 4;
const SPAN_LAG_FRAMES: u64 = 3;
const PICTURE_HISTORY_MASK: u32 = 0xFFFF;
const MOST_PICTURES_AT_HALF_RATE: u32 = 10;

#[derive(Default)]
pub(crate) struct Scenes {
    last_drawn: BTreeMap<u32, u64>,
    battle_over: bool,
    spans: Vec<(u32, [f32; 4], u64)>,
    selection: UnitSelection,
}

impl Scenes {
    pub(crate) fn end_frame(&mut self, frame: u64, drawn_markers: &[u32], new_spans: Vec<(u32, [f32; 4])>, a_held: bool, pictures_from_game: u64) {
        for &id in drawn_markers {
            self.last_drawn.insert(id, frame);
        }
        self.battle_over = self.shown(BATTLE, frame) && (self.battle_over || drawn_markers.contains(&EXP));
        self.spans.extend(new_spans.into_iter().map(|(id, span)| (id, span, frame)));
        self.spans.retain(|&(.., at)| frame - at <= 4 * SPAN_LAG_FRAMES);
        let cursor_on_unit = self.shown(ON_UNIT, frame);
        self.selection.update(a_held, pictures_from_game, cursor_on_unit);
    }

    pub(crate) fn shown(&self, marker: u32, frame: u64) -> bool {
        self.last_drawn.get(&marker).is_some_and(|&at| frame.saturating_sub(at) <= RECENT_FRAMES)
    }

    pub(crate) fn battle_shows(&self, frame: u64) -> bool {
        self.shown(BATTLE, frame) && !self.shown(TALKING, frame) && !self.battle_over
    }

    pub(crate) fn unit_selected(&self) -> bool {
        self.selection.state != SelectionState::NotSelected
    }

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SelectionState {
    #[default]
    NotSelected,
    OnUnit,
    CursorAway,
}

#[derive(Default)]
struct UnitSelection {
    state: SelectionState,
    a_was_held: bool,
    picture_history: u32,
    last_pictures_from_game: u64,
}

impl UnitSelection {
    fn update(&mut self, a_held: bool, pictures_from_game: u64, cursor_on_unit: bool) {
        let a_pressed = a_held && !self.a_was_held;
        self.a_was_held = a_held;
        let new_picture = pictures_from_game > self.last_pictures_from_game;
        self.picture_history = (self.picture_history << 1) | new_picture as u32;
        self.last_pictures_from_game = pictures_from_game;
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

    fn full_rate(selection: &mut UnitSelection, frames: u64, a_held: bool, cursor_on_unit: bool) {
        for _ in 0..frames {
            let pictures = selection.last_pictures_from_game + 1;
            selection.update(a_held, pictures, cursor_on_unit);
        }
    }

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

    #[test]
    fn a_marker_shows_for_four_frames_after_its_last_draw() {
        let mut scenes = Scenes::default();
        scenes.end_frame(10, &[TALKING], Vec::new(), false, 10);
        assert!(scenes.shown(TALKING, 14));
        assert!(!scenes.shown(TALKING, 15));
        assert!(!scenes.shown(BATTLE, 10));
    }
}
