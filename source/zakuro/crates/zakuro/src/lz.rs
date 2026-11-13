//! Packs and unpacks the LZ compression of the files of the game.
//! The game uses the LZ10 and LZ11 formats, and the LZ13 wrapper around LZ11.

use std::collections::HashMap;

/// Unpacks a file of the game by the first byte of its header.
/// Data that does not start with a known compression byte is returned unchanged.
pub fn unpack(data: &[u8]) -> Result<Vec<u8>, String> {
    match data.first() {
        // LZ13 is a four byte header in front of an LZ11 stream.
        Some(0x13) => lz11_unpack(&data[4..]),
        Some(0x10 | 0x11) => lz11_unpack(data),
        _ => Ok(data.to_vec()),
    }
}

/// Unpacks one LZ10 or LZ11 stream, where the first byte of the stream names the format.
fn lz11_unpack(src: &[u8]) -> Result<Vec<u8>, String> {
    let short = || "an LZ stream ends early".to_owned();
    let kind = src[0];
    // The unpacked size is three bytes, little endian.
    let (mut length, mut at) = (src[1] as usize | (src[2] as usize) << 8 | (src[3] as usize) << 16, 4);
    // A size of zero means that the real size follows as four more bytes.
    if length == 0 {
        length = u32::from_le_bytes(src.get(4..8).ok_or_else(short)?.try_into().unwrap()) as usize;
        at = 8;
    }
    let mut out = Vec::with_capacity(length);
    let byte = |at: usize| src.get(at).copied().ok_or_else(short);
    while out.len() < length {
        // One flag byte covers eight items, from the highest bit down. A clear bit is a plain byte, and a set bit is a copy.
        let flags = byte(at)?;
        at += 1;
        for bit in 0..8 {
            if out.len() >= length {
                break;
            }
            if flags & (0x80 >> bit) == 0 {
                out.push(byte(at)?);
                at += 1;
                continue;
            }
            let (count, disp);
            let b0 = byte(at)? as usize;
            if kind == 0x10 {
                // LZ10 copies are two bytes: four bits of length and twelve bits of distance.
                (count, disp) = ((b0 >> 4) + 3, ((b0 & 0xF) << 8 | byte(at + 1)? as usize) + 1);
                at += 2;
            } else {
                // LZ11 picks the size of the copy by the top four bits of the first byte.
                match b0 >> 4 {
                    // Zero: three bytes, eight bits of length.
                    0 => {
                        let (b1, b2) = (byte(at + 1)? as usize, byte(at + 2)? as usize);
                        (count, disp) = (((b0 & 0xF) << 4 | b1 >> 4) + 0x11, ((b1 & 0xF) << 8 | b2) + 1);
                        at += 3;
                    }
                    // One: four bytes, sixteen bits of length.
                    1 => {
                        let (b1, b2, b3) = (byte(at + 1)? as usize, byte(at + 2)? as usize, byte(at + 3)? as usize);
                        (count, disp) = (((b0 & 0xF) << 12 | b1 << 4 | b2 >> 4) + 0x111, ((b2 & 0xF) << 8 | b3) + 1);
                        at += 4;
                    }
                    // Two or more: two bytes, and the top bits are the length minus one.
                    top => {
                        (count, disp) = (top + 1, ((b0 & 0xF) << 8 | byte(at + 1)? as usize) + 1);
                        at += 2;
                    }
                }
            }
            if disp > out.len() {
                return Err("an LZ stream reaches before its start".into());
            }
            // The copy goes byte by byte, so that it can overlap its own output.
            for _ in 0..count {
                out.push(out[out.len() - disp]);
            }
        }
    }
    Ok(out)
}

/// Packs data as an LZ11 stream.
/// The `chain` argument is the most earlier matches that are tried for each position, which trades speed for size.
pub fn lz11(data: &[u8], chain: usize) -> Vec<u8> {
    let n = data.len();
    let mut out = vec![0x11, n as u8, (n >> 8) as u8, (n >> 16) as u8];
    // The latest position of each three byte sequence, and for each position the earlier one with the same sequence.
    let mut heads: HashMap<[u8; 3], i64> = HashMap::new();
    let mut previous = vec![-1i64; n];
    let remember = |i: usize, heads: &mut HashMap<[u8; 3], i64>, previous: &mut Vec<i64>| {
        if i + 3 <= n {
            let key = [data[i], data[i + 1], data[i + 2]];
            previous[i] = heads.get(&key).copied().unwrap_or(-1);
            heads.insert(key, i as i64);
        }
    };
    let mut at = 0;
    while at < n {
        // The flag byte is a placeholder until its eight items are known.
        let flag_at = out.len();
        out.push(0);
        let mut flags = 0u8;
        for bit in 0..8 {
            if at >= n {
                break;
            }
            let (mut best_len, mut best_disp) = (0usize, 0usize);
            if at + 3 <= n {
                let mut cand = heads.get(&[data[at], data[at + 1], data[at + 2]]).copied().unwrap_or(-1);
                // The longest copy that LZ11 can express is 0x10110 bytes, and the farthest distance is 0x1000.
                let limit = (n - at).min(0x10110);
                let mut tries = 0;
                while cand >= 0 && at - cand as usize <= 0x1000 && tries < chain {
                    let c = cand as usize;
                    let mut length = 3;
                    while length < limit && data[c + length] == data[at + length] {
                        length += 1;
                    }
                    if length > best_len {
                        (best_len, best_disp) = (length, at - c);
                        if length == limit {
                            break;
                        }
                    }
                    cand = previous[c];
                    tries += 1;
                }
            }
            if best_len >= 3 {
                flags |= 0x80 >> bit;
                // The stored distance is one less than the real one. The length picks the two, three or four byte form.
                let d = best_disp - 1;
                if best_len <= 16 {
                    out.extend_from_slice(&[((best_len - 1) << 4 | d >> 8) as u8, d as u8]);
                } else if best_len <= 0x110 {
                    let m = best_len - 0x11;
                    out.extend_from_slice(&[(m >> 4) as u8, ((m & 0xF) << 4 | d >> 8) as u8, d as u8]);
                } else {
                    let m = best_len - 0x111;
                    out.extend_from_slice(&[(0x10 | m >> 12) as u8, (m >> 4) as u8, ((m & 0xF) << 4 | d >> 8) as u8, d as u8]);
                }
                for i in at..at + best_len {
                    remember(i, &mut heads, &mut previous);
                }
                at += best_len;
            } else {
                out.push(data[at]);
                remember(at, &mut heads, &mut previous);
                at += 1;
            }
        }
        out[flag_at] = flags;
    }
    // The stream is padded to a multiple of four bytes.
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

/// Packs data as an LZ13 file, which is an LZ11 stream behind a header with the unpacked size.
pub fn lz13(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let mut out = vec![0x13, n as u8, (n >> 8) as u8, (n >> 16) as u8];
    out.extend(lz11(data, 64));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packed data unpacks to the original and is much smaller for repetitive input.
    #[test]
    fn lz11_unpacks_to_what_it_packed() {
        let data: Vec<u8> = (0..5000u32).map(|i| ((i * 7) % 13) as u8).chain(std::iter::repeat_n(9, 300)).collect();
        let packed = lz13(&data);
        assert!(packed.len() < data.len() / 4);
        assert_eq!(unpack(&packed).unwrap(), data);
    }
}
