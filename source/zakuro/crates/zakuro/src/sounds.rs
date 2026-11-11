//! Reads the sounds for the bottom screen from the sound archive of the game.
//! The archive is a BCSAR file, and the waves in it are DSP ADPCM.

use std::sync::Arc;

/// A decoded sound.
pub struct Sound {
    /// The samples as left and right pairs, already scaled by the gain.
    pub frames: Arc<[[f32; 2]]>,
    /// The playback rate in frames per second.
    pub rate: f64,
}

/// A sound in the archive, found by its name.
struct SoundEffect {
    /// The name of the sound in the string table of the archive.
    name: &'static str,
    /// The volume factor that is applied to the samples.
    gain: f32,
}

/// The sounds of the bottom screen for one game.
struct BottomScreenSounds {
    /// The program id of the game that the sounds belong to.
    program_id: u64,
    /// The path of the sound archive in the romfs.
    archive: &'static str,
    /// The sound that plays when the bottom screen opens.
    open: SoundEffect,
    /// The sound that plays when the bottom screen closes.
    close: SoundEffect,
}

/// The games that have sounds for the bottom screen.
/// Each gain is written as a volume out of 127.
const BOTTOM_SCREEN_SOUNDS: &[BottomScreenSounds] = &[BottomScreenSounds {
    program_id: zakuro_core::game_ui::FIRE_EMBLEM_AWAKENING,
    archive: "sound/14Iron_sound_US_EN.bcsar",
    open: SoundEffect { name: "SE_SYS_SELECT1", gain: 42.0 / 127.0 },
    close: SoundEffect { name: "SE_SYS_CANCEL1", gain: 66.0 / 127.0 },
}];

/// The type of the block with the strings of the archive.
const STRING_BLOCK: u16 = 0x2000;
/// The type of the block with the info tables of the archive.
const INFO_BLOCK: u16 = 0x2001;
/// The type of the block with the files of the archive.
const FILE_BLOCK: u16 = 0x2002;
/// The type of the info table of the sounds.
const SOUND_INFO_TABLE: u16 = 0x2100;
/// The type of the info table of the wave archives.
const WAVE_ARCHIVE_INFO_TABLE: u16 = 0x2103;
/// The type of the info table of the files.
const FILE_INFO_TABLE: u16 = 0x2106;
/// The type of a file entry that is stored inside the archive.
const INTERNAL_FILE: u16 = 0x220C;
/// The type of the sound info of a wave sound.
const WAVE_SOUND_INFO: u16 = 0x2202;
/// The type of the info block inside a wave sound file or a wave archive.
const INNER_INFO_BLOCK: u16 = 0x6800;
/// The type of the file block inside a wave archive.
const INNER_FILE_BLOCK: u16 = 0x6801;
/// The type of the info block of a wave file.
const WAVE_INFO_BLOCK: u16 = 0x7000;
/// The type of the data block of a wave file.
const WAVE_DATA_BLOCK: u16 = 0x7001;
/// The size of the header in front of the body of a block.
const BLOCK_HEADER_BYTES: usize = 8;
/// The kind in the top byte of an item id that marks a sound.
const SOUND_ITEM_KIND: u32 = 1;
/// The mask for the index in the low three bytes of an item id.
const ITEM_INDEX_MASK: u32 = 0xFF_FFFF;
/// The encoding value of DSP ADPCM in a wave file.
const DSP_ADPCM: u8 = 2;
/// The size of one frame of DSP ADPCM in bytes.
const ADPCM_FRAME_BYTES: usize = 8;
/// The number of samples in one frame of DSP ADPCM.
const ADPCM_FRAME_SAMPLES: usize = 14;

/// The sounds of the bottom screen, where a sound is missing when it can not be read.
#[derive(Default)]
pub struct BottomScreen {
    /// The sound for opening the bottom screen.
    pub open: Option<Sound>,
    /// The sound for closing the bottom screen.
    pub close: Option<Sound>,
}

/// Reads the sounds of the bottom screen from the game of the title.
/// A game without sounds, or an archive that can not be read, gives empty sounds and a warning in the log.
pub fn bottom_screen(title: &zakuro_fs::Title) -> BottomScreen {
    let Some(sounds) = BOTTOM_SCREEN_SOUNDS.iter().find(|sounds| sounds.program_id == title.program_id()) else {
        return BottomScreen::default();
    };
    let archive = sounds.archive;
    let Some(data) = title.romfs.as_ref().and_then(|romfs| romfs.lookup(archive).ok()).and_then(|file| title.read_romfs(&file, 0, file.data_size as usize)) else {
        log::warn!("{archive} can't be read");
        return BottomScreen::default();
    };
    let read = |effect: &SoundEffect| match sound(&data, effect.name, effect.gain) {
        Ok(sound) => Some(sound),
        Err(error) => {
            log::warn!("{} in {archive} can't be read, {error}", effect.name);
            None
        }
    };
    BottomScreen { open: read(&sounds.open), close: read(&sounds.close) }
}

/// Decodes the sound with the given name from the data of the archive.
/// The path goes from the name through the item and the wave archive to the wave file.
fn sound(data: &[u8], name: &str, gain: f32) -> Result<Sound, String> {
    let archive = SoundArchive::open(data)?;
    let item = archive.sound_item(name)?;
    let wave = archive.wave_of(item, name)?;
    let wave_file = archive.wave_file(&wave)?;
    decode_wave(&archive.reader, wave_file, gain, wave.pitch)
}

/// A reader of the little endian data of the archive, where every read is checked against the end of the data.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    /// Returns a slice of the data, or an error when it reaches past the end.
    fn bytes(&self, at: usize, count: usize) -> Result<&[u8], String> {
        self.0.get(at..at.checked_add(count).ok_or("an offset out of range")?).ok_or_else(|| format!("nothing at 0x{at:X}"))
    }

    /// Reads one byte.
    fn u8(&self, at: usize) -> Result<u8, String> {
        Ok(self.bytes(at, 1)?[0])
    }

    /// Reads a little endian u16.
    fn u16(&self, at: usize) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.bytes(at, 2)?.try_into().unwrap()))
    }

    /// Reads a little endian u32.
    fn u32(&self, at: usize) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.bytes(at, 4)?.try_into().unwrap()))
    }

    /// Reads a little endian f32.
    fn f32(&self, at: usize) -> Result<f32, String> {
        Ok(f32::from_bits(self.u32(at)?))
    }

    /// Follows a reference, which is a type and a signed offset that counts from `base`.
    /// It returns the position that the reference points to.
    fn reference(&self, at: usize, base: usize) -> Result<usize, String> {
        // The offset sits in the second four bytes of the reference.
        let offset = self.u32(at + 4)? as i32;
        base.checked_add_signed(offset as isize).ok_or_else(|| "a reference out of range".into())
    }

    /// Finds the block of a file by its type and returns its position.
    /// The header of the file lists the blocks, and each entry is twelve bytes with a type and an offset.
    fn block(&self, file: usize, kind: u16) -> Result<usize, String> {
        let count = self.u16(file + 0x10)? as usize;
        (0..count)
            .find_map(|i| {
                let entry = file + 0x14 + 12 * i;
                (self.u16(entry).ok()? == kind).then(|| self.u32(entry + 4).ok().map(|offset| file + offset as usize)).flatten()
            })
            .ok_or_else(|| format!("no block 0x{kind:04X}"))
    }

    /// Returns the position of one entry of a table.
    /// A table is a count followed by references of eight bytes, each relative to the start of the table.
    fn table_entry(&self, table: usize, index: usize) -> Result<usize, String> {
        if index >= self.u32(table)? as usize {
            return Err(format!("no entry {index}"));
        }
        self.reference(table + 4 + 8 * index, table)
    }
}

/// An opened sound archive, with the positions of its three main blocks.
struct SoundArchive<'a> {
    /// The reader of the data of the archive.
    reader: Reader<'a>,
    /// The position of the body of the string block.
    strings: usize,
    /// The position of the body of the info block.
    info: usize,
    /// The position of the body of the file block.
    files: usize,
}

/// The wave that a sound plays, as found in the info of the sound.
struct WaveReference {
    /// The id of the wave archive that holds the wave.
    archive_id: u32,
    /// The index of the wave inside its wave archive.
    index: usize,
    /// The factor for the playback rate of the sound.
    pitch: f32,
}

impl<'a> SoundArchive<'a> {
    /// Opens an archive by checking its magic and finding its blocks.
    fn open(data: &'a [u8]) -> Result<SoundArchive<'a>, String> {
        let reader = Reader(data);
        if reader.bytes(0, 4)? != b"CSAR" {
            return Err("not a sound archive".into());
        }
        let strings = reader.block(0, STRING_BLOCK)? + BLOCK_HEADER_BYTES;
        let info = reader.block(0, INFO_BLOCK)? + BLOCK_HEADER_BYTES;
        let files = reader.block(0, FILE_BLOCK)? + BLOCK_HEADER_BYTES;
        Ok(SoundArchive { reader, strings, info, files })
    }

    /// Looks up the item id of a sound by its name.
    /// The name is found in the string table first, and then its index is found in the tree that holds the item ids.
    fn sound_item(&self, name: &str) -> Result<u32, String> {
        let reader = &self.reader;
        let string_table = reader.reference(self.strings, self.strings)?;
        let index = (0..reader.u32(string_table)? as usize)
            .find(|&i| {
                // Each string entry is twelve bytes, with an offset and a size in the last eight.
                let entry = string_table + 4 + 12 * i;
                let (Ok(at), Ok(size)) = (reader.u32(entry + 4), reader.u32(entry + 8)) else { return false };
                reader.bytes(string_table + at as usize, size as usize).is_ok_and(|text| text.strip_suffix(&[0]).unwrap_or(text) == name.as_bytes())
            })
            .ok_or_else(|| format!("no sound named {name}"))?;
        let tree = reader.reference(self.strings + 8, self.strings)?;
        let item = (0..reader.u32(tree + 4)? as usize)
            .find_map(|n| {
                // Each node of the tree is twenty bytes. The string index is at 12 and the item id is at 16.
                let node = tree + 8 + 20 * n;
                (reader.u32(node + 12).ok()? == index as u32).then(|| reader.u32(node + 16).ok()).flatten()
            })
            .ok_or("its name in no item")?;
        // The top byte of the item id is the kind of the item.
        if item >> 24 != SOUND_ITEM_KIND {
            return Err(format!("{name} is not a sound"));
        }
        Ok(item)
    }

    /// Finds an info table by its type and returns its position.
    /// The info block starts with a short list of references, and eight of them are searched.
    fn table(&self, kind: u16) -> Result<usize, String> {
        let reader = &self.reader;
        (0..8).map(|i| self.info + 8 * i).find(|&at| reader.u16(at).ok() == Some(kind)).ok_or(format!("no table 0x{kind:04X}")).and_then(|at| reader.reference(at, self.info))
    }

    /// Returns the position of a file of the archive by its file id.
    /// A file that is stored outside the archive is an error.
    fn file(&self, id: u32) -> Result<usize, String> {
        let reader = &self.reader;
        let entry = reader.table_entry(self.table(FILE_INFO_TABLE)?, id as usize)?;
        if reader.u16(entry)? != INTERNAL_FILE {
            return Err(format!("file {id} is outside the archive"));
        }
        Ok(self.files + reader.u32(reader.reference(entry, entry)? + 4)? as usize)
    }

    /// Finds the wave that a sound item plays.
    /// It reads the sound info, then the wave sound data file, and takes the wave of the first note.
    fn wave_of(&self, item: u32, name: &str) -> Result<WaveReference, String> {
        let reader = &self.reader;
        let sound_info = reader.table_entry(self.table(SOUND_INFO_TABLE)?, (item & ITEM_INDEX_MASK) as usize)?;
        if reader.u16(sound_info + 0x0C)? != WAVE_SOUND_INFO {
            return Err(format!("{name} is not a wave sound"));
        }
        let wave_sound_data = self.file(reader.u32(sound_info)?)?;
        let index_in_data = reader.u32(reader.reference(sound_info + 0x0C, sound_info)?)? as usize;
        let body = reader.block(wave_sound_data, INNER_INFO_BLOCK)? + BLOCK_HEADER_BYTES;
        let waves = reader.reference(body, body)?;
        let wave_sound = reader.table_entry(reader.reference(body + 8, body)?, index_in_data)?;
        let pitch = reader.f32(reader.reference(wave_sound, wave_sound)? + 8)?;
        let notes = reader.reference(wave_sound + 0x10, wave_sound)?;
        let wave = reader.u32(reader.table_entry(notes, 0)?)? as usize;
        if wave >= reader.u32(waves)? as usize {
            return Err("a note of no wave".into());
        }
        Ok(WaveReference { archive_id: reader.u32(waves + 4 + 8 * wave)?, index: reader.u32(waves + 8 + 8 * wave)? as usize, pitch })
    }

    /// Returns the position of the wave file of a wave, inside the file block of its wave archive.
    fn wave_file(&self, wave: &WaveReference) -> Result<usize, String> {
        let reader = &self.reader;
        let wave_archive = self.file(reader.u32(reader.table_entry(self.table(WAVE_ARCHIVE_INFO_TABLE)?, (wave.archive_id & ITEM_INDEX_MASK) as usize)?)?)?;
        let info = reader.block(wave_archive, INNER_INFO_BLOCK)? + BLOCK_HEADER_BYTES;
        let files = reader.block(wave_archive, INNER_FILE_BLOCK)? + BLOCK_HEADER_BYTES;
        if wave.index >= reader.u32(info)? as usize {
            return Err("no such wave in its archive".into());
        }
        Ok(files + reader.u32(info + 4 + 12 * wave.index + 4)? as usize)
    }
}

/// Decodes a wave file of DSP ADPCM into stereo frames.
/// The gain scales the samples, and the pitch scales the rate that is stored in the file.
fn decode_wave(reader: &Reader, wave_file: usize, gain: f32, pitch: f32) -> Result<Sound, String> {
    let info = reader.block(wave_file, WAVE_INFO_BLOCK)? + BLOCK_HEADER_BYTES;
    let data = reader.block(wave_file, WAVE_DATA_BLOCK)? + BLOCK_HEADER_BYTES;
    let (encoding, rate, length) = (reader.u8(info)?, reader.u32(info + 4)?, reader.u32(info + 12)? as usize);
    if encoding != DSP_ADPCM {
        return Err(format!("a wave in encoding {encoding}, not DSP ADPCM"));
    }
    // The channel table sits at offset 0x14 of the info, and at most two channels are decoded.
    let channels = info + 0x14;
    let mut decoded = Vec::new();
    for channel_index in 0..reader.u32(channels)?.min(2) as usize {
        let channel = reader.table_entry(channels, channel_index)?;
        let samples = data + reader.u32(channel + 4)? as usize;
        let adpcm = reader.reference(channel + 8, channel)?;
        // The predictor has eight pairs of coefficients, which are sixteen values of i16.
        let coefficients: [i16; 16] = std::array::from_fn(|k| reader.u16(adpcm + 2 * k).unwrap_or(0) as i16);
        let encoded = reader.bytes(samples, length.div_ceil(ADPCM_FRAME_SAMPLES) * ADPCM_FRAME_BYTES)?;
        decoded.push(decode_adpcm(encoded, length, &coefficients));
    }
    // The samples are i16, so dividing by 32768 brings them into the range of minus one to one. A mono wave is played on both sides.
    let scale = gain / 32768.0;
    let frames: Vec<[f32; 2]> = (0..length)
        .map(|i| match decoded.as_slice() {
            [mono] => [mono[i] as f32 * scale; 2],
            [left, right, ..] => [left[i] as f32 * scale, right[i] as f32 * scale],
            [] => [0.0; 2],
        })
        .collect();
    Ok(Sound { frames: frames.into(), rate: rate as f64 * pitch as f64 })
}

/// Decodes DSP ADPCM into `count` samples.
/// Each frame is eight bytes: a header byte and seven bytes with two four bit nibbles each, which is fourteen samples.
fn decode_adpcm(data: &[u8], count: usize, coefficients: &[i16; 16]) -> Vec<i16> {
    let mut samples = Vec::with_capacity(count);
    let (mut previous, mut before_previous) = (0i32, 0i32);
    for frame in data.chunks_exact(ADPCM_FRAME_BYTES) {
        // The low four bits of the header are the scale as a shift, and the next three bits pick the pair of coefficients.
        let header = frame[0];
        let scale = 1i32 << (header & 0xF);
        let predictor = ((header >> 4) & 7) as usize;
        let (first_coefficient, second_coefficient) = (coefficients[predictor * 2] as i32, coefficients[predictor * 2 + 1] as i32);
        for &byte in &frame[1..] {
            for nibble in [byte >> 4, byte & 0xF] {
                if samples.len() == count {
                    return samples;
                }
                // The nibble is a signed four bit number. The shift pair extends its sign.
                // The new sample is predicted from the two samples before it, with a fixed point of eleven bits, plus the difference.
                let difference = (((nibble as i32) << 28) >> 28) * scale;
                let value = (((difference << 11) + 0x400 + first_coefficient * previous + second_coefficient * before_previous) >> 11).clamp(-32768, 32767);
                (before_previous, previous) = (previous, value);
                samples.push(value as i16);
            }
        }
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sample is predicted from the samples before it.
    #[test]
    fn adpcm_predicts_from_the_samples_before() {
        let mut coefficients = [0i16; 16];
        coefficients[2] = 1 << 11;
        let frame = [0x11, 0x1F, 0, 0, 0, 0, 0, 0];
        let samples = decode_adpcm(&frame, 4, &coefficients);
        assert_eq!(samples, [2, 0, 0, 0]);
        let frame = [0x11, 0x12, 0, 0, 0, 0, 0, 0];
        assert_eq!(decode_adpcm(&frame, 3, &coefficients), [2, 6, 6]);
    }

    /// Reads the real sounds from a rom, which is named by the environment variable `ZAKURO_SOUND_ROM`.
    /// The test is ignored by default because it needs the game.
    #[test]
    #[ignore]
    fn the_bottom_screens_sounds_read_from_the_game() {
        let Some(rom) = std::env::var_os("ZAKURO_SOUND_ROM") else { panic!("set ZAKURO_SOUND_ROM") };
        let title = zakuro_fs::Title::load(rom).unwrap();
        let sounds = bottom_screen(&title);
        let check = |sound: Option<Sound>, length: usize, rate: f64, level: f32| {
            let sound = sound.unwrap();
            assert_eq!(sound.frames.len(), length);
            assert!((sound.rate - rate).abs() < 0.01, "{}", sound.rate);
            let peak = sound.frames.iter().map(|f| f[0].abs().max(f[1].abs())).fold(0.0, f32::max);
            assert!(peak > level * 0.99 && peak <= level, "{peak}");
        };
        check(sounds.open, 13970, 32728.0, 42.0 / 127.0);
        check(sounds.close, 23637, 44100.0, 66.0 / 127.0);
    }
}
