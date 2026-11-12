//! The description of the screens of Fire Emblem Awakening that the renderer needs.
//!
//! It lists the markers that find the draws of the game by their texture and place, and the pieces of the bottom screen that are shown over the top screen.
//! The numbers are positions in the pictures of the game, which are 320 by 240 pixels for the bottom screen.

use zakuro_gpu::{Anchor, BannerSide, Banners, Ground, Marker, Piece, Place, Rounded};

use crate::scenes::{BATTLE, BRIEF_FORECAST, EXP, FORECAST, LEFT_BANNER, ON_UNIT, PREPARATIONS, RIGHT_BANNER, TALKING, TITLE, UNIT_VIEW, WORLD_MAP};

/// The program id of Fire Emblem Awakening.
pub const FIRE_EMBLEM_AWAKENING: u64 = 0x0004_0000_0009_F100;

/// The markers and the pieces of the screens of one game.
pub struct GameUi {
    /// The draws of the game to watch for.
    pub markers: &'static [Marker],
    /// The pieces of the bottom screen shown over the top screen during a battle.
    pub battle_panels: &'static [Piece],
    /// The buttons of a battle.
    pub battle_buttons: &'static [Piece],
    /// The buttons of a battle, placed higher so that they sit over the panels.
    pub battle_buttons_over_panels: &'static [Piece],
    /// The places of the text of the banners of a battle.
    pub battle_banners: &'static Banners,
    /// The pieces of the title screen.
    pub title_pieces: &'static [Piece],
    /// The pieces of the world map.
    pub world_map_pieces: &'static [Piece],
}

/// The description of the screens of the game with the program id, or `None` for a game that has none.
pub fn for_game(program_id: u64) -> Option<&'static GameUi> {
    (program_id == FIRE_EMBLEM_AWAKENING).then_some(&AWAKENING)
}

/// The screens of Fire Emblem Awakening.
static AWAKENING: GameUi = GameUi {
    markers: MARKERS,
    battle_panels: BATTLE_PANELS,
    battle_buttons: BATTLE_BUTTONS,
    battle_buttons_over_panels: BATTLE_BUTTONS_OVER_PANELS,
    battle_banners: &BATTLE_BANNERS,
    title_pieces: TITLE_PIECES,
    world_map_pieces: WORLD_MAP_PIECES,
};

// The codes of the texture formats of the PICA200 that the markers use.
/// The ETC1 format with a 4-bit alpha channel.
const ETC1A4: u32 = 0xD;
/// The format with four bits for each of red, green, blue and alpha.
const RGBA4: u32 = 0x4;
/// The format with eight bits of luminance and eight bits of alpha.
const LA8: u32 = 0x5;
/// The format with eight bits of luminance.
const L8: u32 = 0x7;

/// Builds a marker for a draw with the texture format and size, on the top or the bottom screen, within the place.
const fn marker(id: u32, format: u32, width: u32, height: u32, bottom: bool, place: Place) -> Marker {
    Marker { id, format, width, height, bottom, place }
}

/// Builds the place of a marker from the ranges that the left, top, width and height of the draw may lie in.
const fn place(left: [f32; 2], top: [f32; 2], width: [f32; 2], height: [f32; 2]) -> Place {
    Place { left, top, width, height }
}

/// The value of the `bottom` argument of `marker` for the top screen.
const TOP_SCREEN: bool = false;
/// The value of the `bottom` argument of `marker` for the bottom screen.
const BOTTOM_SCREEN: bool = true;

/// The draws of the game that are watched for.
const MARKERS: &[Marker] = &[
    marker(ON_UNIT, ETC1A4, 256, 256, TOP_SCREEN, place([0.55, 0.98], [0.0, 0.06], [0.3, 0.42], [0.12, 0.21])),
    // This draw has no texture and covers the whole top screen.
    marker(TALKING, Marker::UNTEXTURED, 0, 0, TOP_SCREEN, place([-0.005, 0.005], [-0.005, 0.005], [0.99, 1.01], [0.99, 1.01])),
    marker(WORLD_MAP, RGBA4, 256, 512, TOP_SCREEN, place([0.1, 0.26], [0.8, 0.94], [0.5, 0.75], [0.08, 0.2])),
    marker(TALKING, ETC1A4, 256, 512, TOP_SCREEN, place([-0.05, 0.75], [-0.05, 0.9], [0.3, 0.35], [0.12, 0.16])),
    marker(UNIT_VIEW, ETC1A4, 256, 256, BOTTOM_SCREEN, place([-0.25, -0.05], [-0.05, 0.05], [0.7, 0.9], [1.0, 1.15])),
    marker(FORECAST, ETC1A4, 256, 512, TOP_SCREEN, place([-0.03, 0.03], [0.03, 0.12], [0.95, 1.05], [0.7, 0.85])),
    marker(BRIEF_FORECAST, ETC1A4, 256, 512, TOP_SCREEN, place([-0.03, 0.03], [0.67, 0.695], [0.95, 1.05], [0.31, 0.335])),
    marker(BATTLE, ETC1A4, 256, 512, BOTTOM_SCREEN, place([-0.1, 0.1], [0.0, 0.14], [0.95, 1.05], [0.5, 0.65])),
    // The menus of the preparations draw a bar across the top of the screen, which no other scene draws.
    marker(PREPARATIONS, ETC1A4, 256, 512, TOP_SCREEN, place([-0.02, 0.02], [-0.02, 0.02], [0.97, 1.03], [0.12, 0.15])),
    // A duo shows two gauges, one above and one below the place of a single gauge.
    marker(EXP, ETC1A4, 256, 512, TOP_SCREEN, place([0.18, 0.31], [0.33, 0.51], [0.4, 0.6], [0.11, 0.16])),
    marker(LEFT_BANNER, ETC1A4, 256, 512, BOTTOM_SCREEN, place([0.0, 0.15], [-0.05, 0.08], [0.33, 0.37], [0.08, 0.12])),
    marker(RIGHT_BANNER, ETC1A4, 256, 512, BOTTOM_SCREEN, place([0.5, 0.65], [-0.05, 0.08], [0.33, 0.37], [0.08, 0.12])),
    marker(TITLE, LA8, 256, 32, BOTTOM_SCREEN, place([0.05, 0.15], [0.38, 0.48], [0.75, 0.85], [0.1, 0.17])),
    marker(TITLE, L8, 512, 32, BOTTOM_SCREEN, place([-0.3, -0.2], [0.83, 0.9], [1.45, 1.55], [0.1, 0.16])),
];

/// A rounded rectangle with a shadow, taken from the rectangle in the picture of the bottom screen, with no shift and no outline.
const fn rounded(rect: [u16; 4], corners: [u8; 4]) -> Rounded {
    Rounded { rect, shift: [0, 0], mirror: false, corners, soft: 0, light: false, shade: 255, shadow: true, outline: &[], cut: None }
}

/// A rectangle with a blurred edge that is drawn without alpha and has no shadow.
const fn light(rect: [u16; 4], corners: [u8; 4], soft: u8) -> Rounded {
    Rounded { soft, light: true, shadow: false, ..rounded(rect, corners) }
}

/// A battle button at the horizontal position and width, in the row at 191 that is 42 pixels high.
const fn button(x: u16, width: u16) -> Rounded {
    Rounded { shadow: false, ..rounded([x, 191, width, 42], [4; 4]) }
}

/// The prompt and the copyright line of the title screen, shown at the bottom of the top screen.
const TITLE_PIECES: &[Piece] = &[
    Piece { from: [30, 106, 260, 28], across: Anchor::Middle(0), down: Anchor::End(30), size: 100, shapes: PROMPT, follows: None },
    Piece { from: [12, 212, 296, 26], across: Anchor::Middle(0), down: Anchor::End(0), size: 100, shapes: COPYRIGHT, follows: None },
];
/// The shape of the prompt of the title screen.
const PROMPT: &[Rounded] = &[light([30, 106, 260, 28], [14; 4], 12)];
/// The shape of the copyright line of the title screen.
const COPYRIGHT: &[Rounded] = &[light([12, 212, 296, 26], [13; 4], 10)];

/// The name of the place on the world map, shown at the top of the top screen at 70 percent.
const WORLD_MAP_PIECES: &[Piece] = &[Piece { from: [2, 0, 316, 43], across: Anchor::Middle(0), down: Anchor::Start(2), size: 70, shapes: PLACE_NAME, follows: None }];
/// The shapes of the frame of the name of a place, made of a body, two ends and a small piece at the top whose ground is cut away.
const PLACE_NAME: &[Rounded] = &[
    Rounded { outline: PLACE_FRAME, ..rounded([15, 10, 290, 28], [1; 4]) },
    Rounded { outline: PLACE_FRAME, ..rounded([8, 14, 10, 20], [6, 0, 6, 0]) },
    Rounded { outline: PLACE_FRAME, ..rounded([302, 14, 10, 20], [0, 6, 0, 6]) },
    Rounded { cut: Some(Ground { redder: 3, darker: 90 }), ..rounded([22, 0, 46, 11], [6; 4]) },
];
/// The outline of the frame of the name of a place, as points in tenths of a pixel of the picture.
const PLACE_FRAME: &[[u16; 2]] = &[[174, 100], [3026, 100], [3036, 103], [3046, 115], [3048, 129], [3052, 139], [3066, 143], [3080, 151], [3096, 163], [3102, 171], [3108, 175], [3110, 181], [3116, 187], [3120, 195], [3122, 207], [3126, 211], [3126, 265], [3122, 271], [3116, 291], [3108, 299], [3106, 305], [3102, 307], [3096, 315], [3084, 325], [3054, 339], [3050, 343], [3048, 347], [3048, 359], [3046, 361], [3046, 365], [3038, 373], [3036, 378], [164, 378], [162, 373], [154, 365], [150, 343], [146, 339], [134, 335], [116, 325], [104, 315], [98, 307], [94, 305], [92, 299], [84, 291], [78, 271], [74, 265], [74, 211], [78, 207], [78, 203], [80, 201], [80, 193], [92, 175], [116, 153], [134, 143], [146, 139], [150, 135], [150, 127], [152, 125], [154, 115], [164, 103]];

/// The position in the picture of the left banner at rest, which the left panel follows.
const LEFT_PANEL_REST: [u16; 2] = [24, 4];
/// The position in the picture of the right banner at rest, which the right panel follows.
const RIGHT_PANEL_REST: [u16; 2] = [184, 4];

/// The stats panel and the two hit point panels of a battle.
const BATTLE_PANELS: &[Piece] = &[
    Piece { from: [70, 88, 180, 52], across: Anchor::Middle(0), down: Anchor::End(4), size: 49, shapes: STATS, follows: None },
    Piece { from: [0, 6, 162, 70], across: Anchor::Middle(-88), down: Anchor::End(4), size: 49, shapes: HP_LEFT, follows: Some((LEFT_BANNER, LEFT_PANEL_REST)) },
    Piece { from: [162, 6, 162, 70], across: Anchor::Middle(88), down: Anchor::End(4), size: 49, shapes: HP_RIGHT, follows: Some((RIGHT_BANNER, RIGHT_PANEL_REST)) },
];
/// The shapes of the hit point panel on the left. Rectangles that are one pixel wide are copied side by side to stretch the panel.
const HP_LEFT: &[Rounded] = &[
    Rounded { shift: [4, 0], ..rounded([0, 17, 152, 59], [0; 4]) },
    Rounded { shift: [10, 0], ..rounded([146, 17, 12, 59], [0, 6, 0, 6]) },
    Rounded { shift: [-27, 0], mirror: true, shadow: false, ..rounded([32, 28, 123, 22], [0; 4]) },
    Rounded { shift: [98, 0], shadow: false, ..rounded([30, 28, 1, 22], [0; 4]) },
    Rounded { shift: [129, 0], shadow: false, ..rounded([0, 28, 31, 22], [0; 4]) },
    Rounded { shift: [5, 0], shadow: false, ..rounded([155, 28, 1, 22], [0; 4]) },
    Rounded { shift: [6, 0], shadow: false, ..rounded([155, 28, 1, 22], [0; 4]) },
    Rounded { shift: [7, 0], shadow: false, ..rounded([155, 28, 1, 22], [0; 4]) },
    Rounded { shift: [8, 0], shadow: false, ..rounded([155, 28, 1, 22], [0; 4]) },
    Rounded { shift: [9, 0], shadow: false, ..rounded([155, 28, 1, 22], [0; 4]) },
    Rounded { shift: [-154, 0], mirror: true, shadow: false, ..rounded([154, 24, 4, 44], [0; 4]) },
    Rounded { shift: [-148, 0], mirror: true, shadow: false, ..rounded([148, 68, 10, 8], [0, 0, 6, 0]) },
    Rounded { shift: [-148, 0], mirror: true, shadow: false, ..rounded([148, 17, 10, 10], [6, 0, 0, 0]) },
    Rounded { shift: [4, 0], ..rounded([24, 6, 112, 22], [8; 4]) },
];
/// The shapes of the hit point panel on the right. Rectangles that are one pixel wide are copied side by side to stretch the panel.
const HP_RIGHT: &[Rounded] = &[
    rounded([168, 17, 152, 59], [0; 4]),
    Rounded { shift: [-6, 0], ..rounded([162, 17, 12, 59], [6, 0, 6, 0]) },
    Rounded { shift: [-5, 0], shadow: false, ..rounded([164, 28, 1, 22], [0; 4]) },
    Rounded { shift: [-4, 0], shadow: false, ..rounded([164, 28, 1, 22], [0; 4]) },
    Rounded { shift: [-3, 0], shadow: false, ..rounded([164, 28, 1, 22], [0; 4]) },
    Rounded { shift: [-2, 0], shadow: false, ..rounded([164, 28, 1, 22], [0; 4]) },
    Rounded { shift: [-127, 0], shadow: false, ..rounded([290, 28, 30, 22], [0; 4]) },
    Rounded { shift: [-96, 0], shadow: false, ..rounded([289, 28, 1, 22], [0; 4]) },
    Rounded { shift: [-95, 0], shadow: false, ..rounded([289, 28, 1, 22], [0; 4]) },
    Rounded { shift: [30, 0], mirror: true, shadow: false, ..rounded([165, 28, 125, 22], [0; 4]) },
    Rounded { shift: [157, 0], mirror: true, shadow: false, ..rounded([162, 24, 5, 4], [0; 4]) },
    Rounded { shift: [158, 0], mirror: true, shadow: false, ..rounded([162, 28, 4, 22], [0; 4]) },
    Rounded { shift: [157, 0], mirror: true, shadow: false, ..rounded([162, 50, 5, 18], [0; 4]) },
    Rounded { shift: [152, 0], mirror: true, shadow: false, ..rounded([162, 68, 10, 8], [0, 0, 0, 6]) },
    Rounded { shift: [152, 0], mirror: true, shadow: false, ..rounded([162, 17, 10, 10], [0, 6, 0, 0]) },
    rounded([184, 6, 112, 22], [8; 4]),
];
/// The shapes of the stats panel of a battle.
/// The strip left of the stat labels is mirrored over the shaded strip right of them, which would look like a seam.
/// The row of the attack is left out, because the badge of a double attack sits there on either side and would be copied or hidden.
const STATS: &[Rounded] = &[
    rounded([70, 88, 180, 52], [4; 4]),
    Rounded { shift: [39, 0], mirror: true, shadow: false, ..rounded([136, 88, 9, 3], [0; 4]) },
    Rounded { shift: [39, 0], mirror: true, shadow: false, ..rounded([136, 105, 9, 35], [0; 4]) },
];

/// The row of buttons of a battle, at the bottom of the top screen.
const BATTLE_BUTTONS: &[Piece] = &[Piece { from: [6, 191, 307, 42], across: Anchor::Middle(0), down: Anchor::End(4), size: 28, shapes: BUTTONS, follows: None }];
/// The same row of buttons, 27 pixels higher so that it clears the panels.
const BATTLE_BUTTONS_OVER_PANELS: &[Piece] = &[Piece { from: [6, 191, 307, 42], across: Anchor::Middle(0), down: Anchor::End(31), size: 28, shapes: BUTTONS, follows: None }];
/// The six buttons of the row.
const BUTTONS: &[Rounded] = &[button(6, 43), button(54, 43), button(102, 43), button(175, 42), button(222, 43), button(270, 43)];

/// The places in the picture of the bottom screen that hold the text of the banners of a battle.
const BATTLE_BANNERS: Banners = Banners {
    sides: [
        BannerSide {
            name: [40, 8, 80, 18],
            hp: [0, 30, 30, 20],
            gauge: [32, 28, 122, 20],
            weapon: [36, 54, 120, 22],
            values: [[74, 92, 69, 14], [74, 108, 69, 14], [74, 124, 69, 14]],
            follows: Some((LEFT_BANNER, LEFT_PANEL_REST)),
        },
        BannerSide {
            name: [200, 8, 80, 18],
            hp: [290, 30, 30, 20],
            gauge: [166, 28, 124, 20],
            weapon: [178, 54, 124, 22],
            values: [[177, 92, 69, 14], [177, 108, 69, 14], [177, 124, 69, 14]],
            follows: Some((RIGHT_BANNER, RIGHT_PANEL_REST)),
        },
    ],
    labels: [[146, 92, 28, 14], [146, 108, 28, 14], [146, 124, 28, 14]],
};
