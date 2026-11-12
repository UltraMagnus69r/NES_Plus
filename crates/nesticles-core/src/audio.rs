//! NES DPCM, Game Boy wave RAM, and Super NES BRR.
//!
//! DPCM is LSB-first, starts at level 64, steps by ±2, and clamps to 0–126.
//! BRR uses the integer path described for the workshop: high nibble first,
//! shift ≤ 12 means `(n << shift) >> 1`, otherwise `n & !0x7ff`, then the
//! filter, clamp, and `<< 1` into an i16.

#[derive(Clone, Debug)]
pub struct AudioHit {
    pub offset: usize,
    pub encoding: String,
    pub samples: Vec<f32>,
    pub rate: u32,
    pub score: f32,
    pub label: String,
}

pub fn decode_dpcm_levels(bytes: &[u8]) -> Vec<i32> {
    let mut level: i32 = 64;
    let mut out = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for bit in 0..8 {
            if byte & (1 << bit) != 0 {
                level = (level + 2).min(126);
            } else {
                level = (level - 2).max(0);
            }
            out.push(level);
        }
    }
    out
}

pub fn decode_dpcm_samples(bytes: &[u8]) -> Vec<f32> {
    decode_dpcm_levels(bytes)
        .into_iter()
        .map(|level| (level - 64) as f32 / 64.0)
        .collect()
}

/// 32 Game Boy wave samples packed as 16 bytes of nibbles, high nibble first.
pub fn decode_gb_wave(bytes: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(32);
    for i in 0..16 {
        let b = bytes.get(i).copied().unwrap_or(0);
        out.push(((b >> 4) as f32 - 7.5) / 7.5);
        out.push(((b & 0x0f) as f32 - 7.5) / 7.5);
    }
    out
}

/// Decode one BRR nibble. `p1` and `p2` are previous output samples (already shifted into i16 range).
pub fn decode_brr_sample(nibble: i32, shift: i32, filter: i32, p1: i32, p2: i32) -> i16 {
    let mut n = nibble & 0x0f;
    if n >= 8 {
        n -= 16;
    }
    let mut sample = n;
    if shift <= 12 {
        sample = (sample << shift) >> 1;
    } else {
        sample &= !0x7ff;
    }
    match filter {
        1 => {
            sample += p1 >> 1;
            sample += (-p1) >> 5;
        }
        2 => {
            sample += p1;
            sample -= p2;
            sample += p2 >> 4;
            sample += (-3 * p1) >> 6;
        }
        3 => {
            sample += p1;
            sample -= p2;
            sample += (-13 * p1) >> 7;
            sample += (3 * p2) >> 4;
        }
        _ => {}
    }
    if sample > 32767 {
        sample = 32767;
    }
    if sample < -32768 {
        sample = -32768;
    }
    let out = sample << 1;
    out as i16
}

pub fn decode_brr(bytes: &[u8], max_blocks: usize) -> Vec<i16> {
    let mut out = Vec::new();
    let mut p1: i32 = 0;
    let mut p2: i32 = 0;
    let blocks = (bytes.len() / 9).min(max_blocks);
    for b in 0..blocks {
        let header = bytes[b * 9];
        let shift = (header >> 4) as i32;
        let filter = ((header >> 2) & 3) as i32;
        let end = header & 1 != 0;
        for i in 0..8 {
            let byte = bytes.get(b * 9 + 1 + i).copied().unwrap_or(0);
            for nibble in [((byte >> 4) as i32), ((byte & 0x0f) as i32)] {
                let s = decode_brr_sample(nibble, shift, filter, p1, p2);
                p2 = p1;
                p1 = s as i32;
                out.push(s);
            }
        }
        if end {
            break;
        }
    }
    out
}

pub fn decode_brr_samples(bytes: &[u8]) -> Vec<f32> {
    decode_brr(bytes, 64)
        .into_iter()
        .map(|s| s as f32 / 32768.0)
        .collect()
}

fn score_signal(samples: &[f32]) -> f32 {
    if samples.len() < 32 {
        return 0.0;
    }
    let mut sum = 0.0f32;
    let mut cross = 0.0f32;
    let mut prev = samples[0];
    let mut min = 1.0f32;
    let mut max = -1.0f32;
    for &s in samples {
        sum += s * s;
        if (prev <= 0.0 && s > 0.0) || (prev >= 0.0 && s < 0.0) {
            cross += 1.0;
        }
        prev = s;
        min = min.min(s);
        max = max.max(s);
    }
    let rms = (sum / samples.len() as f32).sqrt();
    let zc = cross / samples.len() as f32;
    if max - min < 0.05 {
        return 0.0;
    }
    if !(0.002..0.45).contains(&zc) {
        return rms * 0.2;
    }
    rms * (1.0 - (zc - 0.05).abs())
}

fn top(mut hits: Vec<AudioHit>, count: usize) -> Vec<AudioHit> {
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(count);
    hits
}

pub fn scan_dpcm(bytes: &[u8], start: usize, end: usize) -> Vec<AudioHit> {
    let mut hits = Vec::new();
    let last = bytes.len().min(end);
    let mut off = start;
    while off + 256 < last {
        let slice_end = (off + 512).min(bytes.len());
        let samples = decode_dpcm_samples(&bytes[off..slice_end]);
        let score = score_signal(&samples);
        if score > 0.08 {
            hits.push(AudioHit {
                offset: off,
                encoding: "dpcm".into(),
                samples,
                rate: 8000,
                score,
                label: format!("DPCM @ ${off:X}"),
            });
        }
        off += 128;
        if hits.len() > 24 {
            break;
        }
    }
    top(hits, 6)
}

pub fn scan_gb_waves(bytes: &[u8]) -> Vec<AudioHit> {
    let mut hits = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let last = bytes.len().min(256 * 1024);
    let mut off = 0;
    while off + 16 <= last {
        let slice = &bytes[off..off + 16];
        if !seen.insert(slice.to_vec()) {
            off += 16;
            continue;
        }
        let wave = decode_gb_wave(slice);
        let mut distinct = std::collections::HashSet::new();
        for s in &wave {
            distinct.insert(s.to_bits());
        }
        if distinct.len() < 4 {
            off += 16;
            continue;
        }
        let mut repeated = Vec::with_capacity(wave.len() * 8);
        for _ in 0..8 {
            repeated.extend_from_slice(&wave);
        }
        let score = score_signal(&repeated);
        if score > 0.0 {
            let mut long = Vec::with_capacity(wave.len() * 16);
            for _ in 0..16 {
                long.extend_from_slice(&wave);
            }
            hits.push(AudioHit {
                offset: off,
                encoding: "wave".into(),
                samples: long,
                rate: 4000,
                score,
                label: format!("Wave @ ${off:X}"),
            });
        }
        if hits.len() > 40 {
            break;
        }
        off += 16;
    }
    top(hits, 8)
}

pub fn scan_brr(bytes: &[u8]) -> Vec<AudioHit> {
    let mut hits = Vec::new();
    let last = bytes.len().min(256 * 1024);
    let mut off = 0;
    while off + 72 < last {
        let header = bytes[off];
        let shift = header >> 4;
        let filter = (header >> 2) & 3;
        if shift > 12 || filter > 3 {
            off += 36;
            continue;
        }
        let mut blocks = 0usize;
        let mut ended = false;
        for b in 0..48 {
            if off + (b + 1) * 9 > bytes.len() {
                break;
            }
            blocks += 1;
            if bytes[off + b * 9] & 1 != 0 {
                ended = true;
                break;
            }
        }
        if ended && blocks >= 8 {
            let samples = decode_brr_samples(&bytes[off..off + blocks * 9]);
            let score = score_signal(&samples);
            if score >= 0.05 {
                hits.push(AudioHit {
                    offset: off,
                    encoding: "brr".into(),
                    samples,
                    rate: 16000,
                    score,
                    label: format!("BRR @ ${off:X}"),
                });
            }
        }
        if hits.len() > 30 {
            break;
        }
        off += 36;
    }
    top(hits, 6)
}
