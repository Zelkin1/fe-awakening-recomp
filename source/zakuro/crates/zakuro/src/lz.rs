use std::collections::HashMap;

pub fn unpack(data: &[u8]) -> Result<Vec<u8>, String> {
    match data.first() {
        Some(0x13) => lz11_unpack(&data[4..]),
        Some(0x10 | 0x11) => lz11_unpack(data),
        _ => Ok(data.to_vec()),
    }
}

fn lz11_unpack(src: &[u8]) -> Result<Vec<u8>, String> {
    let short = || "an LZ stream ends early".to_owned();
    let kind = src[0];
    let (mut length, mut at) = (src[1] as usize | (src[2] as usize) << 8 | (src[3] as usize) << 16, 4);
    if length == 0 {
        length = u32::from_le_bytes(src.get(4..8).ok_or_else(short)?.try_into().unwrap()) as usize;
        at = 8;
    }
    let mut out = Vec::with_capacity(length);
    let byte = |at: usize| src.get(at).copied().ok_or_else(short);
    while out.len() < length {
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
                (count, disp) = ((b0 >> 4) + 3, ((b0 & 0xF) << 8 | byte(at + 1)? as usize) + 1);
                at += 2;
            } else {
                match b0 >> 4 {
                    0 => {
                        let (b1, b2) = (byte(at + 1)? as usize, byte(at + 2)? as usize);
                        (count, disp) = (((b0 & 0xF) << 4 | b1 >> 4) + 0x11, ((b1 & 0xF) << 8 | b2) + 1);
                        at += 3;
                    }
                    1 => {
                        let (b1, b2, b3) = (byte(at + 1)? as usize, byte(at + 2)? as usize, byte(at + 3)? as usize);
                        (count, disp) = (((b0 & 0xF) << 12 | b1 << 4 | b2 >> 4) + 0x111, ((b2 & 0xF) << 8 | b3) + 1);
                        at += 4;
                    }
                    top => {
                        (count, disp) = (top + 1, ((b0 & 0xF) << 8 | byte(at + 1)? as usize) + 1);
                        at += 2;
                    }
                }
            }
            if disp > out.len() {
                return Err("an LZ stream reaches before its start".into());
            }
            for _ in 0..count {
                out.push(out[out.len() - disp]);
            }
        }
    }
    Ok(out)
}

pub fn lz11(data: &[u8], chain: usize) -> Vec<u8> {
    let n = data.len();
    let mut out = vec![0x11, n as u8, (n >> 8) as u8, (n >> 16) as u8];
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
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

pub fn lz13(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let mut out = vec![0x13, n as u8, (n >> 8) as u8, (n >> 16) as u8];
    out.extend(lz11(data, 64));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lz11_unpacks_to_what_it_packed() {
        let data: Vec<u8> = (0..5000u32).map(|i| ((i * 7) % 13) as u8).chain(std::iter::repeat_n(9, 300)).collect();
        let packed = lz13(&data);
        assert!(packed.len() < data.len() / 4);
        assert_eq!(unpack(&packed).unwrap(), data);
    }
}
