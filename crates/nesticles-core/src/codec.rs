//! Tile codecs for uncompressed console patterns.
//! NES 2bpp keeps the two planes in separate 8-byte halves.
//! Game Boy 2bpp interleaves the two planes on every row.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    Nes,
    Gb,
    Snes,
    Gba,
    OneBpp,
}

impl Codec {
    pub fn label(self) -> &'static str {
        match self {
            Codec::Nes => "NES 2bpp",
            Codec::Gb => "Game Boy 2bpp",
            Codec::Snes => "Super NES 4bpp",
            Codec::Gba => "Game Boy Advance 4bpp",
            Codec::OneBpp => "1bpp",
        }
    }
}

pub fn tile_bytes(codec: Codec) -> usize {
    match codec {
        Codec::Snes | Codec::Gba => 32,
        Codec::OneBpp => 8,
        Codec::Nes | Codec::Gb => 16,
    }
}

/// Decode one 8×8 tile into 64 palette indices. Returns None if the slice is short.
pub fn decode_tile(data: &[u8], offset: usize, codec: Codec) -> Option<Vec<u8>> {
    let size = tile_bytes(codec);
    if offset + size > data.len() {
        return None;
    }
    let src = &data[offset..offset + size];
    let mut out = vec![0u8; 64];
    match codec {
        Codec::Nes => {
            for y in 0..8 {
                let p0 = src[y];
                let p1 = src[y + 8];
                for x in 0..8 {
                    let bit = 7 - x;
                    out[y * 8 + x] = ((p0 >> bit) & 1) | (((p1 >> bit) & 1) << 1);
                }
            }
        }
        Codec::Gb => {
            for y in 0..8 {
                let p0 = src[y * 2];
                let p1 = src[y * 2 + 1];
                for x in 0..8 {
                    let bit = 7 - x;
                    out[y * 8 + x] = ((p0 >> bit) & 1) | (((p1 >> bit) & 1) << 1);
                }
            }
        }
        Codec::OneBpp => {
            for y in 0..8 {
                let p0 = src[y];
                for x in 0..8 {
                    out[y * 8 + x] = (p0 >> (7 - x)) & 1;
                }
            }
        }
        Codec::Snes => {
            for y in 0..8 {
                let p0 = src[y];
                let p1 = src[y + 8];
                let p2 = src[y + 16];
                let p3 = src[y + 24];
                for x in 0..8 {
                    let bit = 7 - x;
                    out[y * 8 + x] = ((p0 >> bit) & 1)
                        | (((p1 >> bit) & 1) << 1)
                        | (((p2 >> bit) & 1) << 2)
                        | (((p3 >> bit) & 1) << 3);
                }
            }
        }
        Codec::Gba => {
            for y in 0..8 {
                for x in 0..4 {
                    let byte = src[y * 4 + x];
                    out[y * 8 + x * 2] = byte & 0x0f;
                    out[y * 8 + x * 2 + 1] = (byte >> 4) & 0x0f;
                }
            }
        }
    }
    Some(out)
}

pub fn encode_tile(indices: &[u8], codec: Codec) -> Vec<u8> {
    match codec {
        Codec::Nes => encode_nes(indices),
        Codec::Gb => encode_gb(indices),
        Codec::OneBpp => encode_1bpp(indices),
        Codec::Snes => encode_snes(indices),
        Codec::Gba => encode_gba(indices),
    }
}

fn sample(indices: &[u8], i: usize) -> u8 {
    indices.get(i).copied().unwrap_or(0)
}

fn encode_nes(indices: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    for y in 0..8 {
        let mut p0 = 0u8;
        let mut p1 = 0u8;
        for x in 0..8 {
            let v = sample(indices, y * 8 + x) & 3;
            let bit = 7 - x;
            if v & 1 != 0 {
                p0 |= 1 << bit;
            }
            if v & 2 != 0 {
                p1 |= 1 << bit;
            }
        }
        out[y] = p0;
        out[y + 8] = p1;
    }
    out
}

fn encode_gb(indices: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    for y in 0..8 {
        let mut p0 = 0u8;
        let mut p1 = 0u8;
        for x in 0..8 {
            let v = sample(indices, y * 8 + x) & 3;
            let bit = 7 - x;
            if v & 1 != 0 {
                p0 |= 1 << bit;
            }
            if v & 2 != 0 {
                p1 |= 1 << bit;
            }
        }
        out[y * 2] = p0;
        out[y * 2 + 1] = p1;
    }
    out
}

fn encode_1bpp(indices: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 8];
    for y in 0..8 {
        let mut p0 = 0u8;
        for x in 0..8 {
            if sample(indices, y * 8 + x) & 1 != 0 {
                p0 |= 1 << (7 - x);
            }
        }
        out[y] = p0;
    }
    out
}

fn encode_snes(indices: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 32];
    for y in 0..8 {
        let mut planes = [0u8; 4];
        for x in 0..8 {
            let v = sample(indices, y * 8 + x) & 0x0f;
            let bit = 7 - x;
            for p in 0..4 {
                if v & (1 << p) != 0 {
                    planes[p] |= 1 << bit;
                }
            }
        }
        out[y] = planes[0];
        out[y + 8] = planes[1];
        out[y + 16] = planes[2];
        out[y + 24] = planes[3];
    }
    out
}

fn encode_gba(indices: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 32];
    for y in 0..8 {
        for x in 0..4 {
            let lo = sample(indices, y * 8 + x * 2) & 0x0f;
            let hi = sample(indices, y * 8 + x * 2 + 1) & 0x0f;
            out[y * 4 + x] = lo | (hi << 4);
        }
    }
    out
}

pub fn default_palette(codec: Codec) -> Vec<[u8; 3]> {
    match codec {
        Codec::Gb => vec![
            [15, 56, 15],
            [48, 98, 48],
            [139, 172, 15],
            [155, 188, 15],
        ],
        Codec::Snes | Codec::Gba => vec![
            [20, 16, 24],
            [248, 244, 236],
            [224, 112, 80],
            [240, 200, 120],
            [112, 160, 208],
            [56, 96, 144],
            [104, 176, 104],
            [48, 104, 56],
            [216, 104, 168],
            [136, 56, 104],
            [160, 120, 80],
            [96, 64, 48],
            [192, 192, 200],
            [104, 104, 120],
            [240, 224, 128],
            [32, 32, 40],
        ],
        Codec::Nes | Codec::OneBpp => vec![
            [32, 24, 16],
            [240, 208, 176],
            [200, 72, 48],
            [248, 244, 236],
        ],
    }
}

/// Paint `tiles_x` by `tiles_y` 8×8 tiles. Index 0 can be transparent.
pub fn indices_to_rgba(
    indices: &[u8],
    tiles_x: usize,
    tiles_y: usize,
    palette: &[[u8; 3]],
    transparent_zero: bool,
) -> (u32, u32, Vec<u8>) {
    let width = tiles_x * 8;
    let height = tiles_y * 8;
    let mut rgba = vec![0u8; width * height * 4];
    if tiles_x == 0 || tiles_y == 0 || palette.is_empty() {
        return (width as u32, height as u32, rgba);
    }
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let tile_index = ty * tiles_x + tx;
            let base = tile_index * 64;
            for y in 0..8 {
                for x in 0..8 {
                    let v = indices.get(base + y * 8 + x).copied().unwrap_or(0) as usize;
                    let color = palette.get(v).or_else(|| palette.last()).copied().unwrap_or([0, 0, 0]);
                    let alpha = if transparent_zero && v == 0 { 0 } else { 255 };
                    let px = tx * 8 + x;
                    let py = ty * 8 + y;
                    let o = (py * width + px) * 4;
                    rgba[o] = color[0];
                    rgba[o + 1] = color[1];
                    rgba[o + 2] = color[2];
                    rgba[o + 3] = alpha;
                }
            }
        }
    }
    (width as u32, height as u32, rgba)
}

#[derive(Clone, Debug)]
pub struct TileCandidate {
    pub offset: usize,
    pub suggestion: crate::classify::Suggestion,
    pub indices: Vec<u8>,
}

/// Deduped scan of uncompressed tiles. Empty tiles are omitted.
pub fn scan_tiles(data: &[u8], codec: Codec, start: usize, max_tiles: usize) -> Vec<TileCandidate> {
    let size = tile_bytes(codec);
    let mut seen = std::collections::HashSet::<Vec<u8>>::new();
    let mut out = Vec::new();
    let end = data.len().min(start.saturating_add(max_tiles.saturating_mul(size)));
    let mut off = start;
    while off + size <= end {
        if let Some(indices) = decode_tile(data, off, codec) {
            if !seen.contains(&indices) {
                let suggestion = crate::classify::suggest_kind(&indices, 8, 8);
                seen.insert(indices.clone());
                if suggestion != crate::classify::Suggestion::Empty {
                    out.push(TileCandidate {
                        offset: off,
                        suggestion,
                        indices,
                    });
                }
            }
        }
        off += size;
        if out.len() >= 64 {
            break;
        }
    }
    out.sort_by_key(|c| match c.suggestion {
        crate::classify::Suggestion::Sprite => 0,
        crate::classify::Suggestion::Text => 1,
        crate::classify::Suggestion::Scenery => 2,
        crate::classify::Suggestion::Noise => 3,
        crate::classify::Suggestion::Empty => 4,
    });
    out
}
