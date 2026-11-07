use zakuro_gpu::{Anchor, BannerSide, Banners, Ground, Marker, Piece, Place, Rounded};

use crate::scenes::{BATTLE, BRIEF_FORECAST, EXP, FORECAST, LEFT_BANNER, ON_UNIT, RIGHT_BANNER, TALKING, TITLE, UNIT_VIEW, WORLD_MAP};

pub const FIRE_EMBLEM_AWAKENING: u64 = 0x0004_0000_0009_F100;

pub struct GameUi {
    pub markers: &'static [Marker],
    pub battle_panels: &'static [Piece],
    pub battle_buttons: &'static [Piece],
    pub battle_banners: &'static Banners,
    pub title_pieces: &'static [Piece],
    pub world_map_pieces: &'static [Piece],
}

pub fn for_game(program_id: u64) -> Option<&'static GameUi> {
    (program_id == FIRE_EMBLEM_AWAKENING).then_some(&AWAKENING)
}

static AWAKENING: GameUi = GameUi {
    markers: MARKERS,
    battle_panels: BATTLE_PANELS,
    battle_buttons: BATTLE_BUTTONS,
    battle_banners: &BATTLE_BANNERS,
    title_pieces: TITLE_PIECES,
    world_map_pieces: WORLD_MAP_PIECES,
};

const ETC1A4: u32 = 0xD;
const RGBA4: u32 = 0x4;
const LA8: u32 = 0x5;
const L8: u32 = 0x7;

const fn marker(id: u32, format: u32, width: u32, height: u32, bottom: bool, place: Place) -> Marker {
    Marker { id, format, width, height, bottom, place }
}

const fn place(left: [f32; 2], top: [f32; 2], width: [f32; 2], height: [f32; 2]) -> Place {
    Place { left, top, width, height }
}

const TOP_SCREEN: bool = false;
const BOTTOM_SCREEN: bool = true;

const MARKERS: &[Marker] = &[
    marker(ON_UNIT, ETC1A4, 256, 256, TOP_SCREEN, place([0.55, 0.98], [0.0, 0.06], [0.3, 0.42], [0.12, 0.21])),
    marker(TALKING, Marker::UNTEXTURED, 0, 0, TOP_SCREEN, place([-0.005, 0.005], [-0.005, 0.005], [0.99, 1.01], [0.99, 1.01])),
    marker(WORLD_MAP, RGBA4, 256, 512, TOP_SCREEN, place([0.1, 0.26], [0.8, 0.94], [0.5, 0.75], [0.08, 0.2])),
    marker(TALKING, ETC1A4, 256, 512, TOP_SCREEN, place([-0.05, 0.75], [-0.05, 0.9], [0.3, 0.35], [0.12, 0.16])),
    marker(UNIT_VIEW, ETC1A4, 256, 256, BOTTOM_SCREEN, place([-0.25, -0.05], [-0.05, 0.05], [0.7, 0.9], [1.0, 1.15])),
    marker(FORECAST, ETC1A4, 256, 512, TOP_SCREEN, place([-0.03, 0.03], [0.03, 0.12], [0.95, 1.05], [0.7, 0.85])),
    marker(BRIEF_FORECAST, ETC1A4, 256, 512, TOP_SCREEN, place([-0.03, 0.03], [0.67, 0.695], [0.95, 1.05], [0.31, 0.335])),
    marker(BATTLE, ETC1A4, 256, 512, BOTTOM_SCREEN, place([-0.1, 0.1], [0.0, 0.14], [0.95, 1.05], [0.5, 0.65])),
    marker(EXP, ETC1A4, 256, 512, TOP_SCREEN, place([0.18, 0.31], [0.4, 0.47], [0.4, 0.6], [0.11, 0.16])),
    marker(LEFT_BANNER, ETC1A4, 256, 512, BOTTOM_SCREEN, place([0.0, 0.15], [-0.05, 0.08], [0.33, 0.37], [0.08, 0.12])),
    marker(RIGHT_BANNER, ETC1A4, 256, 512, BOTTOM_SCREEN, place([0.5, 0.65], [-0.05, 0.08], [0.33, 0.37], [0.08, 0.12])),
    marker(TITLE, LA8, 256, 32, BOTTOM_SCREEN, place([0.05, 0.15], [0.38, 0.48], [0.75, 0.85], [0.1, 0.17])),
    marker(TITLE, L8, 512, 32, BOTTOM_SCREEN, place([-0.3, -0.2], [0.83, 0.9], [1.45, 1.55], [0.1, 0.16])),
];

const fn rounded(rect: [u16; 4], corners: [u8; 4]) -> Rounded {
    Rounded { rect, shift: [0, 0], mirror: false, corners, soft: 0, light: false, shade: 255, shadow: true, outline: &[], cut: None }
}

const fn light(rect: [u16; 4], corners: [u8; 4], soft: u8) -> Rounded {
    Rounded { soft, light: true, shadow: false, ..rounded(rect, corners) }
}

const fn button(x: u16, width: u16) -> Rounded {
    Rounded { shadow: false, ..rounded([x, 191, width, 42], [4; 4]) }
}

const TITLE_PIECES: &[Piece] = &[
    Piece { from: [30, 106, 260, 28], across: Anchor::Middle(0), down: Anchor::End(30), size: 100, shapes: PROMPT, follows: None },
    Piece { from: [12, 212, 296, 26], across: Anchor::Middle(0), down: Anchor::End(0), size: 100, shapes: COPYRIGHT, follows: None },
];
const PROMPT: &[Rounded] = &[light([30, 106, 260, 28], [14; 4], 12)];
const COPYRIGHT: &[Rounded] = &[light([12, 212, 296, 26], [13; 4], 10)];

const WORLD_MAP_PIECES: &[Piece] = &[Piece { from: [2, 0, 316, 43], across: Anchor::Middle(0), down: Anchor::Start(2), size: 70, shapes: PLACE_NAME, follows: None }];
const PLACE_NAME: &[Rounded] = &[
    Rounded { outline: PLACE_FRAME, ..rounded([15, 10, 290, 28], [1; 4]) },
    Rounded { outline: PLACE_FRAME, ..rounded([8, 14, 10, 20], [6, 0, 6, 0]) },
    Rounded { outline: PLACE_FRAME, ..rounded([302, 14, 10, 20], [0, 6, 0, 6]) },
    Rounded { cut: Some(Ground { redder: 3, darker: 90 }), ..rounded([22, 0, 46, 11], [6; 4]) },
];
const PLACE_FRAME: &[[u16; 2]] = &[[174, 100], [3026, 100], [3036, 103], [3046, 115], [3048, 129], [3052, 139], [3066, 143], [3080, 151], [3096, 163], [3102, 171], [3108, 175], [3110, 181], [3116, 187], [3120, 195], [3122, 207], [3126, 211], [3126, 265], [3122, 271], [3116, 291], [3108, 299], [3106, 305], [3102, 307], [3096, 315], [3084, 325], [3054, 339], [3050, 343], [3048, 347], [3048, 359], [3046, 361], [3046, 365], [3038, 373], [3036, 378], [164, 378], [162, 373], [154, 365], [150, 343], [146, 339], [134, 335], [116, 325], [104, 315], [98, 307], [94, 305], [92, 299], [84, 291], [78, 271], [74, 265], [74, 211], [78, 207], [78, 203], [80, 201], [80, 193], [92, 175], [116, 153], [134, 143], [146, 139], [150, 135], [150, 127], [152, 125], [154, 115], [164, 103]];

const LEFT_PANEL_REST: [u16; 2] = [24, 4];
const RIGHT_PANEL_REST: [u16; 2] = [184, 4];

const BATTLE_PANELS: &[Piece] = &[
    Piece { from: [68, 85, 107, 59], across: Anchor::Start(0), down: Anchor::End(0), size: 50, shapes: STATS_LEFT, follows: None },
    Piece { from: [0, 6, 162, 70], across: Anchor::Start(53), down: Anchor::End(0), size: 50, shapes: HP_LEFT, follows: Some((LEFT_BANNER, LEFT_PANEL_REST)) },
    Piece { from: [145, 85, 107, 59], across: Anchor::End(0), down: Anchor::End(0), size: 50, shapes: STATS_RIGHT, follows: None },
    Piece { from: [162, 6, 162, 70], across: Anchor::End(53), down: Anchor::End(0), size: 50, shapes: HP_RIGHT, follows: Some((RIGHT_BANNER, RIGHT_PANEL_REST)) },
];
const HP_LEFT: &[Rounded] = &[
    Rounded { shift: [4, 0], ..rounded([0, 17, 158, 59], [0, 6, 0, 0]) },
    Rounded { shift: [-153, 0], mirror: true, shadow: false, ..rounded([153, 24, 5, 52], [0; 4]) },
    Rounded { shift: [-153, -7], mirror: true, shadow: false, ..rounded([153, 24, 5, 8], [0; 4]) },
    Rounded { shadow: false, ..rounded([0, 17, 6, 1], [0; 4]) },
    Rounded { shift: [4, 0], ..rounded([24, 6, 112, 22], [8; 4]) },
];
const HP_RIGHT: &[Rounded] = &[
    rounded([162, 17, 158, 59], [6, 0, 0, 0]),
    Rounded { shift: [157, 0], mirror: true, shadow: false, ..rounded([162, 24, 5, 52], [0; 4]) },
    Rounded { shift: [157, -7], mirror: true, shadow: false, ..rounded([162, 24, 5, 8], [0; 4]) },
    Rounded { shift: [4, 0], shadow: false, ..rounded([314, 17, 6, 1], [0; 4]) },
    rounded([184, 6, 112, 22], [8; 4]),
];
const STATS_LEFT: &[Rounded] = &[
    Rounded { shade: 205, ..rounded([68, 85, 107, 59], [0; 4]) },
    Rounded { shade: 205, shift: [0, -53], shadow: false, ..rounded([68, 138, 107, 6], [0; 4]) },
];
const STATS_RIGHT: &[Rounded] = &[
    Rounded { shade: 205, ..rounded([145, 85, 107, 59], [0; 4]) },
    Rounded { shade: 205, shift: [0, -53], shadow: false, ..rounded([145, 138, 107, 6], [0; 4]) },
];

const BATTLE_BUTTONS: &[Piece] = &[Piece { from: [6, 191, 307, 42], across: Anchor::Middle(0), down: Anchor::End(4), size: 28, shapes: BUTTONS, follows: None }];
const BUTTONS: &[Rounded] = &[button(6, 43), button(54, 43), button(102, 43), button(175, 42), button(222, 43), button(270, 43)];

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
