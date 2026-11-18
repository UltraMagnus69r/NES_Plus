//! Turn a verified SMB1 Goomba candidate into sprite frames.
//!
//! The public disassembly's `EnemyGraphicsTable` stores one walk row,
//! `FC FC 70 71 72 73`, and one defeat row, `FC FC FC FC EF EF`. `FC` is an
//! empty slot. The walk cycle is not a second row of tiles. This payload's
//! graphics handler mirrors that metasprite while `FrameCounter` bit 3 is
//! clear, which lasts 8 frames. Defeat draws the same tile twice and flips
//! only the right column.
//!
//! Sprite patterns for this payload sit in the first 4 KB of CHR. Reset stores
//! PPUCTRL `$10`, so bit 3 is clear and sprites use `$0000`.
//!
//! Colors passed in here are a display palette. CHR does not contain the PPU
//! palette, and this module does not claim those colors are the original ones.

use crate::anim::{self, Playback};
use crate::codec::{decode_tile, Codec};
use crate::detect::{self, RomFormat};
use crate::pipeline::{Candidate, FrameDraft, TileSpot, SMB1_PAYLOAD_SHA256};
use crate::pngutil::b64_encode;
use crate::project::{nid, Asset, AssetKind, Project};

pub const TAG: &str = "r2-goomba";
pub const PALETTE_NOTE: &str = "Review colors only — original palette not yet verified.";
pub const WALK_TICKS: u32 = 8;

const TIMING_VERIFIED: &str = "Walk holds each pose for 8 frames. This cartridge's graphics handler loads FrameCounter ($09), masks bit 3, and mirrors the metasprite while that bit is clear. Defeat plays once and does not loop.";
const TIMING_PROVISIONAL: &str = "Timing is provisional. The 8-frame walk interval was not found in this payload, so it is not claimed as original NES timing.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BakedFrame {
    pub state: String,
    pub ticks: u32,
    pub playback: Playback,
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
    pub indices: Vec<u8>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
    pub frames: Vec<BakedFrame>,
    pub palette_note: String,
    pub timing_note: String,
    pub timing_verified: bool,
    pub palette_verified: bool,
    pub payload_sha256: String,
    /// Source rows that were not turned into frames.
    pub omitted: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Published {
    pub sprite_id: String,
    pub set_id: String,
    pub walk_clip_id: String,
    pub frame_ids: Vec<String>,
    pub message: String,
}

/// Assemble the Goomba when the payload hash and the documented rows agree.
/// Any other candidate, or a different payload, is refused.
pub fn assemble(
    rom: &[u8],
    payload_sha256: &str,
    candidate: &Candidate,
    palette: &[[u8; 3]; 4],
) -> Result<Assembly, String> {
    if candidate.character_id != "goomba" {
        return Err(
            "Only the verified Goomba is assembled in this version. Nothing was invented for this candidate."
                .into(),
        );
    }
    if payload_sha256 != SMB1_PAYLOAD_SHA256 {
        return Err(
            "This payload is not the verified SMB1 profile. Goomba was not assembled.".into(),
        );
    }
    let info = detect::detect_rom(rom, "reconstruct");
    if info.format != RomFormat::Nes || info.chr_len < 16 {
        return Err("This file has no NES CHR to assemble.".into());
    }
    let (timing_verified, sprites_at_0000) = handler_marks(rom, &info);
    if !sprites_at_0000 {
        return Err(
            "This payload does not store the expected sprite-pattern setting. Goomba was not assembled from a guessed table."
                .into(),
        );
    }
    let (frames, omitted) = goomba_frames(rom, info.chr_start, candidate, palette)?;
    Ok(Assembly {
        frames,
        palette_note: PALETTE_NOTE.into(),
        timing_note: if timing_verified {
            TIMING_VERIFIED.into()
        } else {
            TIMING_PROVISIONAL.into()
        },
        timing_verified,
        palette_verified: false,
        payload_sha256: payload_sha256.into(),
        omitted,
    })
}

pub fn pose_frame<'a>(
    assembly: &'a Assembly,
    pose: &str,
    elapsed: u32,
) -> Option<(usize, &'a BakedFrame)> {
    let indexes: Vec<usize> = assembly
        .frames
        .iter()
        .enumerate()
        .filter(|(_, frame)| frame.state.eq_ignore_ascii_case(pose))
        .map(|(index, _)| index)
        .collect();
    if indexes.is_empty() {
        return None;
    }
    let durations: Vec<u32> = indexes
        .iter()
        .map(|index| assembly.frames[*index].ticks.max(1))
        .collect();
    let local = match assembly.frames[indexes[0]].playback {
        Playback::Loop => walk_loop(&durations, elapsed),
        Playback::Once | Playback::Hold | Playback::PingPong => walk_hold(&durations, elapsed),
    };
    Some((local, &assembly.frames[indexes[local]]))
}

/// Write the assembled Goomba into the project. The ROM is not copied.
/// The candidate status is left unchanged so the caller can record approval
/// only after the project file is actually written.
pub fn publish_goomba(
    project: &mut Project,
    rom: &[u8],
    candidate: &Candidate,
    palette: &[[u8; 3]; 4],
) -> Result<Published, String> {
    let report = crate::pipeline::inspect_nes(rom, "publish")?;
    let assembly = assemble(rom, &report.payload_sha256, candidate, palette)?;
    let payload = report.payload_sha256;
    let walks = assembly
        .frames
        .iter()
        .filter(|frame| frame.state == "walk")
        .count();
    if walks < 2 {
        return Err("Goomba walk does not have two poses. Nothing was saved.".into());
    }
    retire_previous(project);
    let max_w = assembly
        .frames
        .iter()
        .map(|frame| frame.w)
        .max()
        .unwrap_or(16);
    let max_h = assembly
        .frames
        .iter()
        .map(|frame| frame.h)
        .max()
        .unwrap_or(16);
    let sprite_id = anim::create_sprite(project, "Goomba", max_w, max_h)?;
    let mut frame_ids = Vec::new();
    for frame in &assembly.frames {
        let asset_id = nid("asset-");
        project.assets.push(picture_asset(
            &asset_id,
            &format!("Goomba {}", frame.state),
            frame.w,
            frame.h,
            &frame.rgba,
        ));
        let frame_id = anim::append_picture_frame(project, &sprite_id, &asset_id, frame.ticks)?;
        frame_ids.push(frame_id);
    }
    if let Some(sprite) = project
        .sprites
        .iter_mut()
        .find(|sprite| sprite.id == sprite_id)
    {
        sprite.tags = vec![
            TAG.into(),
            format!("payload:{payload}"),
            "palette:provisional-review".into(),
            if assembly.timing_verified {
                "timing:framecounter-bit3-8".into()
            } else {
                "timing:provisional-8".into()
            },
        ];
        sprite.palette = palette.to_vec();
    }
    let set_id = anim::create_set(project, "Goomba", &sprite_id)?;
    let mut walk_clip_id = String::new();
    let mut cursor = 0usize;
    let mut states: Vec<(String, Playback)> = Vec::new();
    for frame in &assembly.frames {
        if states.last().map(|(name, _)| name == &frame.state) != Some(true) {
            states.push((frame.state.clone(), frame.playback));
        }
    }
    for (state, playback) in &states {
        let count = assembly
            .frames
            .iter()
            .filter(|frame| frame.state == *state)
            .count();
        let clip_id = anim::add_clip(project, &set_id, state)?;
        for offset in 0..count {
            let baked = &assembly.frames[cursor + offset];
            let frame_id = &frame_ids[cursor + offset];
            anim::append_frame_to_clip(project, &set_id, &clip_id, frame_id, baked.ticks)?;
        }
        anim::set_playback(project, &set_id, &clip_id, *playback)?;
        if state == "walk" {
            walk_clip_id = clip_id;
        }
        cursor += count;
    }
    if walk_clip_id.is_empty() {
        retire_previous(project);
        return Err(
            "Goomba walk was not attached to the animation. The partial sprite was removed.".into(),
        );
    }
    let text = serde_json::to_string(project).unwrap_or_default();
    if text.contains('\u{1a}') || text.contains("sk-") || text.len() > 200_000 {
        retire_previous(project);
        return Err(
            "The project did not pass the save check. Goomba was removed and the cartridge was not stored."
                .into(),
        );
    }
    let defeat = assembly.frames.iter().any(|frame| frame.state == "defeat");
    let message = if defeat {
        format!(
            "Saved Goomba into the project. Walk loops two different poses. Defeat plays once. {} {}",
            assembly.timing_note, assembly.palette_note
        )
    } else {
        format!(
            "Saved the walking Goomba. Defeat was not published: {}. {}",
            assembly.omitted.join(" "),
            assembly.palette_note
        )
    };
    Ok(Published {
        sprite_id,
        set_id,
        walk_clip_id,
        frame_ids,
        message,
    })
}

/// One baked picture per source row. The Goomba path adds the mirrored walk.
pub fn bake_candidate(
    rom: &[u8],
    payload_sha256: &str,
    candidate: &Candidate,
    palette: &[[u8; 3]; 4],
) -> Result<Vec<BakedFrame>, String> {
    if candidate.character_id == "goomba" && payload_sha256 == SMB1_PAYLOAD_SHA256 {
        let assembly = assemble(rom, payload_sha256, candidate, palette)?;
        return Ok(assembly.frames);
    }
    let mut frames = Vec::new();
    for draft in &candidate.frames {
        frames.push(bake_draft(rom, draft, &GRAY, Playback::Loop, WALK_TICKS)?);
    }
    if frames.is_empty() {
        return Err("There is no frame to bake.".into());
    }
    Ok(frames)
}

const GRAY: [[u8; 3]; 4] = [[0, 0, 0], [96, 96, 96], [168, 168, 168], [248, 248, 248]];

fn goomba_frames(
    rom: &[u8],
    chr_start: usize,
    candidate: &Candidate,
    palette: &[[u8; 3]; 4],
) -> Result<(Vec<BakedFrame>, Vec<String>), String> {
    let walk = candidate
        .frames
        .iter()
        .find(|frame| frame.state == "walk")
        .ok_or_else(|| "The Goomba profile has no walk row. Nothing was assembled.".to_string())?;
    for tile in [0x70u32, 0x71, 0x72, 0x73] {
        if walk.spots.iter().all(|spot| spot.tile != tile) {
            return Err(format!(
                "Walk is missing tile 0x{tile:02X}. Goomba was not assembled."
            ));
        }
    }
    let (w, h, first) = paint_spots(rom, chr_start, &walk.spots)?;
    let second = mirror(&first, w, h);
    if first == second {
        return Err(
            "The mirrored walk pose is the same picture. Goomba was not saved as a fake animation."
                .into(),
        );
    }
    if !opaque(&first) || !opaque(&second) {
        return Err("A walk pose decoded with no visible pixels. Nothing was saved.".into());
    }
    let mut frames = vec![
        baked(
            "walk",
            Playback::Loop,
            w,
            h,
            first,
            palette,
            "Walk pose from tiles 0x70-0x73.",
        ),
        baked(
            "walk",
            Playback::Loop,
            w,
            h,
            second,
            palette,
            "Walk pose mirrored the way the graphics handler flips the metasprite.",
        ),
    ];
    let mut omitted = Vec::new();
    match defeat_indices(rom, chr_start, candidate) {
        Ok((dw, dh, indices)) if opaque(&indices) => frames.push(baked(
            "defeat",
            Playback::Once,
            dw,
            dh,
            indices,
            palette,
            "Defeat uses tile 0xEF twice. The right copy is flipped horizontally.",
        )),
        Ok(_) => omitted.push("The defeat tiles decoded empty. Defeat was not invented.".into()),
        Err(err) => omitted.push(err),
    }
    Ok((frames, omitted))
}

fn defeat_indices(
    rom: &[u8],
    chr_start: usize,
    candidate: &Candidate,
) -> Result<(u32, u32, Vec<u8>), String> {
    let Some(defeat) = candidate
        .frames
        .iter()
        .find(|frame| frame.state == "defeat")
    else {
        return Err(
            "The profile has no defeat row. A defeat pose was not invented, and the walk was not saved without reporting that."
                .into(),
        );
    };
    let copies = defeat.spots.iter().filter(|spot| spot.tile == 0xEF).count();
    if copies < 2 {
        return Err(
            "Defeat does not contain the two documented 0xEF tiles. It was not invented.".into(),
        );
    }
    let max_x = defeat.spots.iter().map(|spot| spot.x).max().unwrap_or(0);
    let spots = defeat
        .spots
        .iter()
        .cloned()
        .map(|mut spot| {
            if spot.tile == 0xEF && spot.x == max_x {
                spot.flip_x = true;
            }
            spot
        })
        .collect::<Vec<_>>();
    paint_spots(rom, chr_start, &spots)
}

fn baked(
    state: &str,
    playback: Playback,
    w: u32,
    h: u32,
    indices: Vec<u8>,
    palette: &[[u8; 3]; 4],
    note: &str,
) -> BakedFrame {
    let rgba = colorize(&indices, palette);
    BakedFrame {
        state: state.into(),
        ticks: WALK_TICKS,
        playback,
        w,
        h,
        rgba,
        indices,
        note: note.into(),
    }
}

fn bake_draft(
    rom: &[u8],
    draft: &FrameDraft,
    palette: &[[u8; 3]; 4],
    playback: Playback,
    ticks: u32,
) -> Result<BakedFrame, String> {
    let info = detect::detect_rom(rom, "bake");
    let (w, h, indices) = paint_spots(rom, info.chr_start, &draft.spots)?;
    let rgba = colorize(&indices, palette);
    Ok(BakedFrame {
        state: draft.state.clone(),
        ticks,
        playback,
        w,
        h,
        rgba,
        indices,
        note: draft.origin.clone(),
    })
}

fn paint_spots(
    rom: &[u8],
    chr_start: usize,
    spots: &[TileSpot],
) -> Result<(u32, u32, Vec<u8>), String> {
    if spots.is_empty() {
        return Err("A frame has no tiles.".into());
    }
    let mut max_x = 8i32;
    let mut max_y = 8i32;
    for spot in spots {
        max_x = max_x.max(spot.x.saturating_add(8));
        max_y = max_y.max(spot.y.saturating_add(8));
    }
    if max_x <= 0 || max_y <= 0 || max_x > 256 || max_y > 256 {
        return Err("That frame is outside the sprite limit.".into());
    }
    let w = max_x as u32;
    let h = max_y as u32;
    let mut indices = vec![0u8; w as usize * h as usize];
    for spot in spots {
        let tile = read_tile(rom, chr_start, spot.tile)?;
        for row in 0..8i32 {
            for col in 0..8i32 {
                let sx = if spot.flip_x { 7 - col } else { col };
                let sy = if spot.flip_y { 7 - row } else { row };
                let value = tile[(sy * 8 + sx) as usize];
                let dx = spot.x + col;
                let dy = spot.y + row;
                if dx < 0 || dy < 0 || dx >= w as i32 || dy >= h as i32 {
                    continue;
                }
                let at = dy as usize * w as usize + dx as usize;
                if value != 0 {
                    indices[at] = value;
                }
            }
        }
    }
    Ok((w, h, indices))
}

fn read_tile(rom: &[u8], chr_start: usize, tile: u32) -> Result<[u8; 64], String> {
    let offset = chr_start
        .checked_add(tile as usize * 16)
        .ok_or_else(|| format!("Tile 0x{tile:02X} is outside CHR."))?;
    let decoded = decode_tile(rom, offset, Codec::Nes)
        .ok_or_else(|| format!("Tile 0x{tile:02X} could not be decoded."))?;
    let mut out = [0u8; 64];
    out.copy_from_slice(&decoded[..64]);
    Ok(out)
}

fn mirror(indices: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; indices.len()];
    for y in 0..h {
        for x in 0..w {
            let src = (y * w + x) as usize;
            let dst = (y * w + (w - 1 - x)) as usize;
            out[dst] = indices[src];
        }
    }
    out
}

fn colorize(indices: &[u8], palette: &[[u8; 3]; 4]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(indices.len() * 4);
    for value in indices {
        let slot = (*value).min(3) as usize;
        let color = palette[slot];
        let alpha = if slot == 0 { 0 } else { 255 };
        rgba.extend_from_slice(&[color[0], color[1], color[2], alpha]);
    }
    rgba
}

fn opaque(indices: &[u8]) -> bool {
    indices.iter().any(|value| *value != 0)
}

fn handler_marks(rom: &[u8], info: &crate::detect::RomInfo) -> (bool, bool) {
    let end = info.prg_start.saturating_add(info.prg_len).min(rom.len());
    let start = info.prg_start.min(end);
    let prg = &rom[start..end];
    let timing = prg
        .windows(10)
        .any(|window| window == [0xA5, 0x09, 0x29, 0x08, 0xD0, 0x06, 0xA5, 0x03, 0x49, 0x03]);
    let sprites_at_0000 = prg
        .windows(5)
        .any(|window| window == [0xA9, 0x10, 0x8D, 0x00, 0x20]);
    (timing, sprites_at_0000)
}

fn picture_asset(id: &str, name: &str, w: u32, h: u32, rgba: &[u8]) -> Asset {
    Asset {
        id: id.into(),
        name: name.into(),
        kind: AssetKind::Enemy,
        style_id: String::new(),
        w,
        h,
        rgba_base64: b64_encode(rgba),
        png_base64: String::new(),
        wav_base64: String::new(),
        solid: false,
        cue: String::new(),
        item_tag: Default::default(),
        block_tag: Default::default(),
        hazard_tag: Default::default(),
        warp: None,
    }
}

fn retire_previous(project: &mut Project) {
    let ids: Vec<String> = project
        .sprites
        .iter()
        .filter(|sprite| sprite.tags.iter().any(|tag| tag == TAG))
        .map(|sprite| sprite.id.clone())
        .collect();
    for id in ids {
        let assets: Vec<String> = project
            .sprites
            .iter()
            .filter(|sprite| sprite.id == id)
            .flat_map(|sprite| sprite.frames.iter())
            .flat_map(|frame| frame.parts.iter())
            .map(|part| part.asset_id.clone())
            .collect();
        project.anim_sets.retain(|set| set.sprite_id != id);
        project.sprites.retain(|sprite| sprite.id != id);
        for asset in assets {
            let used = project.sprites.iter().any(|sprite| {
                sprite
                    .frames
                    .iter()
                    .any(|frame| frame.parts.iter().any(|part| part.asset_id == asset))
            });
            if !used {
                project.assets.retain(|item| item.id != asset);
            }
        }
    }
}

fn walk_loop(durations: &[u32], elapsed: u32) -> usize {
    let total: u32 = durations.iter().copied().sum();
    if total == 0 {
        return 0;
    }
    let mut time = elapsed % total;
    for (index, duration) in durations.iter().enumerate() {
        if time < *duration {
            return index;
        }
        time -= *duration;
    }
    0
}

fn walk_hold(durations: &[u32], elapsed: u32) -> usize {
    let mut time = elapsed;
    for (index, duration) in durations.iter().enumerate() {
        if time < *duration || index + 1 == durations.len() {
            return index;
        }
        time -= *duration;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{
        import_package, package_character, propose_session, synthetic_fixture, ReviewStatus,
    };

    const WARM: [[u8; 3]; 4] = [[0, 0, 0], [80, 40, 16], [216, 112, 48], [248, 224, 176]];

    #[test]
    fn an_asymmetric_tile_mirrors_into_a_different_picture() {
        let mut tile = [0u8; 16];
        tile[0] = 0b1000_0000;
        tile[8] = 0b1000_0000;
        let mut rom = vec![0x4E, 0x45, 0x53, 0x1A, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        rom.extend(std::iter::repeat(0).take(16_384));
        rom.extend_from_slice(&tile);
        rom.extend(std::iter::repeat(0).take(8192 - 16));
        let spots = vec![TileSpot {
            tile: 0,
            x: 0,
            y: 0,
            flip_x: false,
            flip_y: false,
        }];
        let (w, h, indices) = paint_spots(&rom, 16 + 16_384, &spots).unwrap();
        let mirrored = mirror(&indices, w, h);
        assert_ne!(indices, mirrored);
        assert_eq!(indices[0], 3);
        assert_eq!(mirrored[7], 3);
        assert!(indices.contains(&0));
    }

    #[test]
    fn a_different_payload_is_not_published_as_goomba() {
        let rom = synthetic_fixture();
        let session = propose_session(&rom, "synthetic-fixture.nes", &[], &[]).unwrap();
        assert!(session
            .candidates
            .iter()
            .all(|candidate| candidate.character_id != "goomba"));
        let profile = crate::pipeline::smb1_profile();
        let goomba = profile
            .candidates
            .iter()
            .find(|candidate| candidate.character_id == "goomba")
            .unwrap();
        let mut project = Project::blank("synthetic");
        let err = publish_goomba(&mut project, &rom, goomba, &WARM).unwrap_err();
        assert!(err.contains("not the verified"));
        assert!(project.sprites.is_empty());
    }

    #[test]
    fn verified_smb1_goomba_survives_project_and_package_round_trip() {
        let bytes = std::fs::read("/home/g/Desktop/Roms/smb1.nes").expect("owner ROM");
        assert_eq!(bytes.len(), 40_976);
        assert_eq!(
            crate::pipeline::sha256(&bytes),
            "0b3d9e1f01ed1668205bab34d6c82b0e281456e137352e4f36a9b2cfa3b66dea"
        );
        let session = propose_session(&bytes, "cartridge.nes", &[], &[]).unwrap();
        assert_eq!(session.report.payload_sha256, SMB1_PAYLOAD_SHA256);
        assert_eq!(session.report.mapper, 0);
        assert_eq!(session.report.chr_bytes, 8192);
        assert_eq!(session.report.mirroring, "vertical");
        let goomba = session
            .candidates
            .iter()
            .find(|candidate| candidate.character_id == "goomba")
            .unwrap()
            .clone();
        let assembly = assemble(&bytes, &session.report.payload_sha256, &goomba, &WARM).unwrap();
        assert!(assembly.timing_verified);
        assert!(!assembly.palette_verified);
        assert!(assembly
            .palette_note
            .contains("original palette not yet verified"));
        let walks: Vec<_> = assembly
            .frames
            .iter()
            .filter(|frame| frame.state == "walk")
            .collect();
        assert_eq!(walks.len(), 2);
        assert_ne!(walks[0].indices, walks[1].indices);
        assert!(walks[0].indices.contains(&0));
        assert_eq!((walks[0].w, walks[0].h), (16, 24));
        let defeat = assembly
            .frames
            .iter()
            .find(|frame| frame.state == "defeat")
            .unwrap();
        assert_eq!(defeat.playback, Playback::Once);
        assert_ne!(defeat.indices, walks[0].indices);
        let mut project = Project::blank("Goomba");
        let published = publish_goomba(&mut project, &bytes, &goomba, &WARM).unwrap();
        let text = serde_json::to_string(&project).unwrap();
        assert!(text.len() < 200_000);
        assert!(!text.contains('\u{1a}'));
        assert!(!text.contains("sk-"));
        let loaded = crate::blueprint::load_project_json(&text).unwrap();
        let pose_a = anim::present(&loaded, &published.set_id, "walk", 1, 0);
        let pose_b = anim::present(&loaded, &published.set_id, "walk", 1, 8);
        assert!(!pose_a.missing);
        assert_ne!(pose_a.rgba, pose_b.rgba);
        assert_eq!((pose_a.w, pose_a.h), (16, 24));
        let squashed = anim::present(&loaded, &published.set_id, "defeat", 1, 0);
        assert_ne!(squashed.rgba, pose_a.rgba);
        let mut packaging = session;
        packaging
            .candidates
            .iter_mut()
            .find(|candidate| candidate.character_id == "goomba")
            .unwrap()
            .status = ReviewStatus::Approved;
        let package = package_character(&packaging, &bytes, "goomba").unwrap();
        assert!(package.forms[0].pictures.len() >= 3);
        let mut imported = Project::blank("imported");
        import_package(&mut imported, &package).unwrap();
        let set = &imported.anim_sets[0];
        let walk = set.clips.iter().find(|clip| clip.name == "walk").unwrap();
        assert_eq!(walk.frame_ids.len(), 2);
        assert_ne!(walk.frame_ids[0], walk.frame_ids[1]);
        assert_eq!(walk.playback, Playback::Loop);
        assert_eq!(walk.ticks, vec![8, 8]);
        let defeat_clip = set.clips.iter().find(|clip| clip.name == "defeat").unwrap();
        assert_eq!(defeat_clip.playback, Playback::Once);
        assert_eq!(defeat_clip.frame_ids.len(), 1);
        let imported_a = anim::present(&imported, &set.id, "walk", 1, 0);
        let imported_b = anim::present(&imported, &set.id, "walk", 1, 8);
        assert_ne!(imported_a.rgba, imported_b.rgba);
        let package_text = crate::pipeline::package_json(&package).unwrap();
        assert!(!package_text.contains("sk-"));
        assert!(package_text.len() < 200_000);
    }
}
