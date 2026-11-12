//! Header sniffing for NES, Game Boy, Game Boy Color, Super NES, and Game Boy Advance.
//! Encrypted Switch, 3DS, and Wii U images are never decrypted.

use crate::codec::Codec;
use serde::{Deserialize, Serialize};

pub const MAX_ROM_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RomFormat {
    Nes,
    Gb,
    Gbc,
    Snes,
    Gba,
    Unknown,
}

impl RomFormat {
    pub fn label(self) -> &'static str {
        match self {
            RomFormat::Nes => "NES",
            RomFormat::Gb => "Game Boy",
            RomFormat::Gbc => "Game Boy Color",
            RomFormat::Snes => "Super NES",
            RomFormat::Gba => "Game Boy Advance",
            RomFormat::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RomInfo {
    pub format: RomFormat,
    pub title: String,
    pub codec: Codec,
    pub tile_offset: usize,
    pub trainer: bool,
    pub prg_start: usize,
    pub chr_start: usize,
    pub prg_len: usize,
    pub chr_len: usize,
    pub nes2: bool,
    pub details: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

fn ascii(bytes: &[u8], offset: usize, length: usize) -> String {
    let mut out = String::new();
    for i in 0..length {
        if offset + i >= bytes.len() {
            break;
        }
        let c = bytes[offset + i];
        if (32..127).contains(&c) {
            out.push(c as char);
        }
    }
    out.replace('\0', "").trim().to_string()
}

fn read16(bytes: &[u8], offset: usize) -> u16 {
    if offset + 1 >= bytes.len() {
        return 0;
    }
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn snes_score(bytes: &[u8], base: usize) -> i32 {
    if base + 0x20 > bytes.len() {
        return -1;
    }
    let mut printable = 0i32;
    for i in 0..21 {
        let c = bytes[base + i];
        if (32..127).contains(&c) {
            printable += 1;
        }
    }
    let complement = read16(bytes, base + 0x1c);
    let checksum = read16(bytes, base + 0x1e);
    let mut score = printable;
    if complement ^ checksum == 0xffff {
        score += 80;
    }
    let map = bytes[base + 0x15];
    if matches!(map, 0x20 | 0x21 | 0x30 | 0x31) {
        score += 10;
    }
    score
}

pub fn detect_rom(bytes: &[u8], filename: &str) -> RomInfo {
    let mut warnings = Vec::new();
    let mut details = vec![
        ("File".into(), filename.to_string()),
        ("Size".into(), format!("{} bytes", bytes.len())),
    ];
    let stem = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
    let title_from_name = stem.rsplit_once('.').map(|(s, _)| s).unwrap_or(stem).to_string();

    if bytes.len() >= 16 && bytes[0] == 0x4e && bytes[1] == 0x45 && bytes[2] == 0x53 && bytes[3] == 0x1a {
        let flags6 = bytes[6];
        let flags7 = bytes[7];
        let nes2 = flags7 & 0x0c == 0x08;
        let mut prg = bytes[4] as usize * 16384;
        let mut chr = bytes[5] as usize * 8192;
        if nes2 && bytes.len() >= 10 {
            prg = (((bytes[9] & 0x0f) as usize) << 8 | bytes[4] as usize) * 16384;
            chr = (((bytes[9] >> 4) as usize) << 8 | bytes[5] as usize) * 8192;
        }
        let trainer = flags6 & 0x04 != 0;
        let prg_start = 16 + if trainer { 512 } else { 0 };
        let chr_start = prg_start + prg;
        if chr == 0 {
            warnings.push(
                "This cartridge has no CHR ROM. Graphics are copied into CHR RAM by the program, so they are not a single sheet. Page the program as NES tiles, or use a tile viewer such as Mesen while the game builds its own patterns.".into(),
            );
        }
        let mapper = ((flags6 >> 4) as u16) | ((flags7 as u16) & 0xf0);
        details.push(("Format".into(), if nes2 { "NES 2.0" } else { "iNES" }.into()));
        details.push(("PRG".into(), format!("{} KB", prg / 1024)));
        details.push((
            "CHR".into(),
            if chr == 0 {
                "CHR RAM".into()
            } else {
                format!("{} KB", chr / 1024)
            },
        ));
        details.push(("Mapper".into(), mapper.to_string()));
        details.push(("Trainer".into(), if trainer { "512 bytes".into() } else { "none".into() }));
        return RomInfo {
            format: RomFormat::Nes,
            title: title_from_name,
            codec: Codec::Nes,
            tile_offset: if chr == 0 { prg_start } else { chr_start.min(bytes.len()) },
            trainer,
            prg_start,
            chr_start,
            prg_len: prg,
            chr_len: chr,
            nes2,
            details,
            warnings,
        };
    }

    if bytes.len() > 0x150 && bytes[0x104] == 0xce && bytes[0x105] == 0xed {
        let cgb = bytes[0x143];
        let format = if cgb == 0x80 || cgb == 0xc0 {
            RomFormat::Gbc
        } else {
            RomFormat::Gb
        };
        let title_len = if format == RomFormat::Gbc { 15 } else { 16 };
        let title = {
            let t = ascii(bytes, 0x134, title_len);
            if t.is_empty() { title_from_name } else { t }
        };
        details.push((
            "Format".into(),
            if format == RomFormat::Gbc { "Game Boy Color" } else { "Game Boy" }.into(),
        ));
        details.push(("Title".into(), title.clone()));
        details.push(("Cartridge type".into(), format!("${:02X}", bytes[0x147])));
        warnings.push(
            "Game Boy graphics are scattered through the ROM as 2bpp tiles, and many pictures are only assembled by code. Candidates are uncompressed tiles. A screenshot search such as gbgraphics helps when you already know which picture you want.".into(),
        );
        return RomInfo {
            format,
            title,
            codec: Codec::Gb,
            tile_offset: 0,
            trainer: false,
            prg_start: 0,
            chr_start: 0,
            prg_len: 0,
            chr_len: 0,
            nes2: false,
            details,
            warnings,
        };
    }

    if bytes.len() > 0xc0 && bytes[0xb2] == 0x96 {
        let title = {
            let t = ascii(bytes, 0xa0, 12);
            if t.is_empty() { title_from_name } else { t }
        };
        details.push(("Format".into(), "Game Boy Advance".into()));
        details.push(("Title".into(), title.clone()));
        warnings.push(
            "Game Boy Advance graphics are often compressed or stored as 4bpp or 8bpp bitmaps. This viewer pages uncompressed 4bpp tiles (low nibble first). Compressed sheets need a game-specific decoder.".into(),
        );
        return RomInfo {
            format: RomFormat::Gba,
            title,
            codec: Codec::Gba,
            tile_offset: 0,
            trainer: false,
            prg_start: 0,
            chr_start: 0,
            prg_len: 0,
            chr_len: 0,
            nes2: false,
            details,
            warnings,
        };
    }

    let mut smc = 0usize;
    if bytes.len() > 512 && bytes.len() % 1024 == 512 {
        smc = 512;
        warnings.push("A 512-byte copier header looks present. Tile offsets below include it.".into());
    }
    let lo = snes_score(bytes, smc + 0x7fc0);
    let hi = snes_score(bytes, smc + 0xffc0);
    if lo >= 40 || hi >= 40 {
        let hirom = hi > lo;
        let base = if hirom { smc + 0xffc0 } else { smc + 0x7fc0 };
        let title = {
            let t = ascii(bytes, base, 21);
            if t.is_empty() { title_from_name } else { t }
        };
        details.push((
            "Format".into(),
            if hirom { "Super NES HiROM" } else { "Super NES LoROM" }.into(),
        ));
        details.push(("Title".into(), title.clone()));
        let map = bytes.get(base + 0x15).copied().unwrap_or(0);
        details.push(("Map mode".into(), format!("${map:02X}")));
        warnings.push(
            "Many Super NES games compress graphics. Uncompressed 4bpp tiles show up in the pager. For compressed sheets, palettes, and scripts, RetroTool and Mesen go further than a universal scan.".into(),
        );
        return RomInfo {
            format: RomFormat::Snes,
            title,
            codec: Codec::Snes,
            tile_offset: smc,
            trainer: false,
            prg_start: 0,
            chr_start: 0,
            prg_len: 0,
            chr_len: 0,
            nes2: false,
            details,
            warnings,
        };
    }

    warnings.push(
        "No NES, Game Boy, Super NES, or Game Boy Advance header matched. You can still page the file as tiles. Encrypted Switch, 3DS, and Wii U images are not decrypted.".into(),
    );
    let ext = filename.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let codec = match ext.as_str() {
        "gb" | "gbc" => Codec::Gb,
        "gba" => Codec::Gba,
        "sfc" | "smc" => Codec::Snes,
        _ => Codec::Nes,
    };
    RomInfo {
        format: RomFormat::Unknown,
        title: filename.to_string(),
        codec,
        tile_offset: 0,
        trainer: false,
        prg_start: 0,
        chr_start: 0,
        prg_len: 0,
        chr_len: 0,
        nes2: false,
        details,
        warnings,
    }
}

pub fn find_strings(bytes: &[u8], limit: usize) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let scan = bytes.len().min(1024 * 1024);
    let mut start = None;
    let mut text = String::new();
    let flush = |found: &mut Vec<(usize, String)>, start: &mut Option<usize>, text: &mut String| {
        if text.len() >= 6 && text.chars().collect::<std::collections::HashSet<_>>().len() > 2 {
            if let Some(at) = *start {
                found.push((at, text.clone()));
            }
        }
        *start = None;
        text.clear();
    };
    for i in 0..scan {
        let c = bytes[i];
        if (32..127).contains(&c) {
            if start.is_none() {
                start = Some(i);
            }
            text.push(c as char);
            if text.len() > 80 {
                flush(&mut found, &mut start, &mut text);
            }
        } else if start.is_some() {
            flush(&mut found, &mut start, &mut text);
        }
        if found.len() >= limit {
            break;
        }
    }
    if start.is_some() {
        flush(&mut found, &mut start, &mut text);
    }
    found.truncate(limit);
    found
}
