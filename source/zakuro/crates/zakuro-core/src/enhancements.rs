//! enhancements, the community's codes that make games run at 60 FPS and
//! the like, for the builds of games they were tested on, which a player
//! turns on by name instead of finding and pasting the code.

use std::collections::BTreeMap;

use crate::cheats::{Cheat, CheatSetting};

const LIST: &str = include_str!("enhancements.txt");

const DEFAULT_SOURCE: &str = "the 60 FPS thread on GBAtemp";

/// one enhancement for a game.
#[derive(Debug, Clone, PartialEq)]
pub struct Enhancement {
    pub name: String,
    pub author: String,
    /// what a player should know before turning it on, empty if nothing.
    pub note: String,
    pub source: String,
    pub lines: Vec<String>,
    pub setting: Option<Setting>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    pub name: String,
    pub min: u16,
    pub max: u16,
    pub default: u16,
    pub unit: String,
}

impl Setting {
    fn parse(line: &str) -> Option<Setting> {
        let (declaration, default_and_unit) = line.strip_prefix('~')?.split_once('=')?;
        let (name, range) = declaration.trim().rsplit_once(' ')?;
        let (min, max) = range.split_once("..")?;
        let mut default_and_unit = default_and_unit.split_whitespace();
        let default = default_and_unit.next()?.parse().ok()?;
        let unit = default_and_unit.collect::<Vec<_>>().join(" ");
        Some(Setting { name: name.trim().to_owned(), min: min.parse().ok()?, max: max.parse().ok()?, default, unit })
    }
}

pub(crate) fn code_with_value(lines: &[String], value: u16) -> Vec<String> {
    let float = |number: f32| format!("{:08X}", number.to_bits());
    let (fraction, inverse) = (float(value as f32 / 100.0), float(100.0 / value.max(1) as f32));
    lines.iter().map(|line| line.replace("{x/100}", &fraction).replace("{100/x}", &inverse)).collect()
}

/// the build a title runs, which an enhancement's code has to be made for:
/// its update's version, or else its cartridge's revision.
pub fn build(title: &zakuro_fs::Title) -> String {
    match title.update() {
        Some(update) => format!("update {}.{}", update.version >> 10, update.version >> 4 & 0x3F),
        None => format!("rev{}", title.exheader.remaster_version),
    }
}

/// the enhancements for a game of program_id running build.
pub fn for_title(program_id: u64, build: &str) -> Vec<Enhancement> {
    let mut found: Vec<Enhancement> = Vec::new();
    let (mut game, mut taking) = (false, false);
    for line in LIST.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with('#')) {
        if let Some(header) = line.strip_prefix('=') {
            let id = header.split_whitespace().next().and_then(|id| u64::from_str_radix(id, 16).ok());
            game = id == Some(program_id);
            taking = false;
        } else if let Some((name, rest)) = line.split_once('|').filter(|_| game) {
            let mut fields = rest.split('|').map(str::trim);
            let made_for = fields.next().unwrap_or_default();
            taking = made_for == "any" || made_for == build;
            if taking {
                found.push(Enhancement {
                    name: name.trim().to_owned(),
                    author: fields.next().unwrap_or_default().to_owned(),
                    note: fields.next().unwrap_or_default().to_owned(),
                    source: fields.next().filter(|source| !source.is_empty()).unwrap_or(DEFAULT_SOURCE).to_owned(),
                    lines: Vec::new(),
                    setting: None,
                });
            }
        } else if let Some(setting) = Setting::parse(line).filter(|_| game && taking) {
            if let Some(enhancement) = found.last_mut() {
                enhancement.setting = Some(setting);
            }
        } else if game && taking {
            if let Some(enhancement) = found.last_mut() {
                enhancement.lines.push(line.to_owned());
            }
        }
    }
    found
}

pub fn cheats(program_id: u64, build: &str, on: &[String], values: &BTreeMap<String, u16>) -> Vec<Cheat> {
    for_title(program_id, build).into_iter().map(|enhancement| cheat_for(enhancement, on, values)).collect()
}

fn cheat_for(enhancement: Enhancement, on: &[String], values: &BTreeMap<String, u16>) -> Cheat {
    let credit = format!("Code by {}, from {}.", enhancement.author, enhancement.source);
    let notes = [enhancement.note, credit].into_iter().filter(|line| !line.is_empty()).collect();
    let setting = enhancement.setting.map(|setting| {
        let value = values.get(&enhancement.name).copied().unwrap_or(setting.default).clamp(setting.min, setting.max);
        CheatSetting::new(setting, value, enhancement.lines.clone())
    });
    let lines = match &setting {
        Some(setting) => code_with_value(&enhancement.lines, setting.value),
        None => enhancement.lines,
    };
    let mut cheat = Cheat::new(&enhancement.name, lines, notes);
    cheat.builtin = true;
    cheat.enabled = on.contains(&enhancement.name);
    cheat.setting = setting;
    cheat
}

#[cfg(test)]
mod tests {
    use super::*;

    /// each enhancement is for its own game and build, with its note and
    /// who made it, and only its own code.
    #[test]
    fn a_game_gets_the_enhancements_for_its_build() {
        let enhancements = for_title(0x0004_0000_0008_C300, "rev2");
        assert_eq!(enhancements.len(), 1);
        let fps = &enhancements[0];
        assert_eq!((fps.name.as_str(), fps.author.as_str()), ("60 FPS", "Hazerou"));
        assert!(!fps.note.is_empty());
        assert_eq!(fps.lines, ["D3000000 08000000", "2020A748 0000003C", "D2000000 00000000"]);
        assert!(for_title(0x0004_0000_0008_C300, "rev0").is_empty());
        assert!(for_title(0x0004_0000_0008_C300, "update 1.1").is_empty());

        let cheats = cheats(0x0004_0000_0008_C300, "rev2", &["60 FPS".to_owned()], &Default::default());
        assert!(cheats[0].enabled && cheats[0].builtin);
        assert_eq!(cheats[0].notes.len(), 2);
        assert_eq!(fps.source, DEFAULT_SOURCE, "the thread, when the list names no other place");
    }

    #[test]
    fn a_code_says_where_it_came_from() {
        let enhancements = for_title(0x0004_0000_0009_F100, "rev0");
        assert_eq!(enhancements.len(), 3);
        let cheats = cheats(0x0004_0000_0009_F100, "rev0", &[], &Default::default());
        assert!(cheats[0].notes.iter().any(|note| note.contains("recomp's notes")));

        let wide = &enhancements[0];
        assert_eq!(wide.name, "16:9");
        assert!(wide.source.contains("recomp"));
        assert!(wide.lines.iter().all(|line| line.starts_with('0')), "32-bit writes alone");
        assert_eq!(wide.lines.len(), 15 + 28);
        assert_eq!(wide.lines[0], "002FE310 EB0572FA");
        assert_eq!(wide.lines[13], "0045AF30 3F888889");
        assert_eq!(wide.lines[14], "003D57F4 3FE38E39");
        let glow = &wide.lines[15..];
        assert_eq!(glow[0], "002EA610 EB05C258");
        assert_eq!(glow[9], "002D1984 EB062583");
        let routines = glow.iter().filter(|line| line.starts_with("0045AF")).map(|line| u32::from_str_radix(&line[..8], 16).unwrap());
        assert!(routines.clone().all(|addr| (0x0045_AF78..0x0045_B000).contains(&addr)), "past the words of Faster battles");
        assert_eq!(routines.count(), 26);

        let smooth = &enhancements[1];
        assert_eq!(smooth.name, "Smooth map sprites");
        assert!(smooth.source.contains("recomp"));
        assert_eq!(smooth.lines[..2], ["003D1B18 E1A00000", "003D1B24 E1A00000"]);
        assert_eq!(smooth.lines.len(), 11);

        let faster = &enhancements[2];
        assert_eq!(faster.name, "Faster battles");
        let speed = Setting { name: "Speed".into(), min: 100, max: 200, default: 100, unit: "%".into() };
        assert_eq!(faster.setting, Some(speed));
        assert_eq!(faster.lines.len(), 14);
        assert_eq!(faster.lines[10..12], ["0045AF70 {100/x}", "0045AF74 {x/100}"]);
        assert_eq!(faster.lines[12..], ["001D6168 0045AF40", "001D61B4 0045AF54"]);
        assert_eq!(code_with_value(&faster.lines, 150)[10..12], ["0045AF70 3F2AAAAB", "0045AF74 3FC00000"]);
        assert_eq!(code_with_value(&faster.lines, 130)[10..12], ["0045AF70 3F44EC4F", "0045AF74 3FA66666"]);
        let start = |values: &[(&str, u16)]| {
            let values = values.iter().map(|&(name, value)| (name.to_owned(), value)).collect();
            super::cheats(0x0004_0000_0009_F100, "rev0", &[], &values)[2].setting.as_ref().map(|setting| setting.value)
        };
        assert_eq!((start(&[]), start(&[("Faster battles", 120)]), start(&[("Faster battles", 900)])), (Some(100), Some(120), Some(200)));

    }

    #[test]
    fn a_new_value_rebuilds_the_code() {
        let mut faster = cheats(0x0004_0000_0009_F100, "rev0", &[], &Default::default()).remove(2);
        assert_eq!(faster.set_value(150), Some(150));
        assert_eq!(faster.lines()[10..12], ["0045AF70 3F2AAAAB", "0045AF74 3FC00000"]);
        assert_eq!(faster.set_value(900), Some(200), "the most the setting allows");
        let mut wide = cheats(0x0004_0000_0009_F100, "rev0", &[], &Default::default()).remove(0);
        assert_eq!(wide.set_value(150), None, "16:9 has no setting");
    }

    #[test]
    fn every_listed_code_is_code() {
        let code = LIST.lines().map(str::trim).filter(|line| !line.is_empty() && !line.starts_with(['#', '=', '~']) && !line.contains('|'));
        for line in code_with_value(&code.map(str::to_owned).collect::<Vec<_>>(), 150) {
            assert!(crate::cheats::is_code(&line), "{line}");
        }
    }
}
