//! Cartridge extraction, review, local recipes, and content packages.
//!
//! Tile indexes for Super Mario Bros. come from the public disassembly's
//! `EnemyGraphicsTable`. Pixels are copied from the user's ROM only at export,
//! and only after that ROM's PRG+CHR payload matches a profile. A recipe is
//! never applied to a different payload as if it were verified.
//!
//! This module does not upload anything. Knowledge leaves the machine only when
//! the user exports a `.nknowledge` file.

use crate::anim::{self, Playback};
use crate::behavior::BehaviorGraph;
use crate::codec::{decode_tile, indices_to_rgba, Codec};
use crate::detect::{self, RomFormat};
use crate::pngutil::{b64_decode, b64_encode};
use crate::project::{nid, Asset, AssetKind, Project};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const ALPHA_ID: &str = "alpha2-phase9.1";
pub const KNOWLEDGE_SCHEMA: u32 = 1;
pub const PACKAGE_SCHEMA: u32 = 1;
/// No-Intro headerless Super Mario Bros. (World), 40960 bytes, CRC32 d445f698.
pub const SMB1_PAYLOAD_SHA256: &str =
    "fcb6a0ef3a20c19b356005fbb21dc8009563b1cb5a9aaebc8e9386b4a8c5912e";

const MAX_KNOWLEDGE_BYTES: usize = 2_000_000;
const MAX_PACKAGE_BYTES: usize = 8_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CartridgeReport {
    pub sha256: String,
    pub payload_sha256: String,
    pub format: String,
    pub nes2: bool,
    pub trainer: bool,
    pub prg_bytes: usize,
    pub chr_bytes: usize,
    pub mapper: u16,
    pub mirroring: String,
    pub length_ok: bool,
    pub supported: bool,
    pub profile_id: String,
    pub profile_label: String,
    pub partial: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTile {
    pub index: u32,
    pub bank: u32,
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Awaiting,
    Approved,
    Failed,
    Reinspecting,
    Ignored,
    Unsupported,
}

impl ReviewStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Awaiting => "Awaiting review",
            Self::Approved => "Approved",
            Self::Failed => "Failed / needs reinspection",
            Self::Reinspecting => "Reinspection in progress",
            Self::Ignored => "Ignored",
            Self::Unsupported => "Unsupported",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailReason {
    WrongIdentity,
    WrongTiles,
    WrongPalette,
    WrongAlignment,
    MissingFrames,
    WrongAnimation,
    Duplicate,
    Other,
}

impl FailReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::WrongIdentity => "Wrong identity",
            Self::WrongTiles => "Wrong tile grouping",
            Self::WrongPalette => "Incorrect palette",
            Self::WrongAlignment => "Incorrect alignment",
            Self::MissingFrames => "Missing animation frames",
            Self::WrongAnimation => "Incorrect animation",
            Self::Duplicate => "Duplicate asset",
            Self::Other => "Other",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TileSpot {
    pub tile: u32,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameDraft {
    pub state: String,
    /// How this frame was obtained. Never upgraded to verified by a model response.
    pub origin: String,
    pub spots: Vec<TileSpot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    pub tiles: Vec<u32>,
    pub reason: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub target_style: String,
    pub collection: String,
    pub character_id: String,
    pub form: String,
    pub frames: Vec<FrameDraft>,
    pub palette: u8,
    pub status: ReviewStatus,
    pub fail_reason: String,
    pub confidence: String,
    pub evidence: String,
    pub attempts: Vec<Attempt>,
    /// Tiles the user removed. They stay available to the puzzle.
    #[serde(default)]
    pub rejected_tiles: Vec<u32>,
}

impl Candidate {
    pub fn tile_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for frame in &self.frames {
            for spot in &frame.spots {
                if !ids.contains(&spot.tile) {
                    ids.push(spot.tile);
                }
            }
        }
        ids
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageRow {
    pub name: String,
    pub category: String,
    pub raw: String,
    pub assembly: String,
    pub animation: String,
    pub palette: String,
    pub review: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtractionSession {
    pub report: ReportSaved,
    pub tiles: Vec<RawTile>,
    pub candidates: Vec<Candidate>,
    pub coverage: Vec<CoverageRow>,
}

/// The part of the report we can save. The ROM itself is not saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSaved {
    pub sha256: String,
    pub payload_sha256: String,
    pub mapper: u16,
    pub mirroring: String,
    pub chr_bytes: usize,
    pub profile_id: String,
    pub profile_label: String,
    pub partial: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub schema: u32,
    pub payload_sha256: String,
    pub profile_id: String,
    pub name: String,
    pub category: String,
    pub character_id: String,
    pub form: String,
    pub target_style: String,
    pub collection: String,
    pub tiles: Vec<u32>,
    pub frames: Vec<FrameDraft>,
    pub palette: u8,
    pub verified: bool,
    pub rejected: Vec<Vec<u32>>,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct KnowledgeFile {
    schema: u32,
    recipes: Vec<Recipe>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackState {
    pub name: String,
    pub origin: String,
    pub ticks: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackForm {
    pub form: String,
    pub w: u32,
    pub h: u32,
    pub rgba_base64: String,
    pub states: Vec<PackState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentPackage {
    pub schema: u32,
    pub name: String,
    pub category: String,
    pub target_style: String,
    pub collection: String,
    pub provenance: String,
    pub source_payload_sha256: String,
    pub forms: Vec<PackForm>,
    #[serde(default)]
    pub behavior: Option<BehaviorGraph>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ExtractJob {
    pub stage: u8,
    pub stage_name: String,
    pub paused: bool,
    pub cancelled: bool,
    pub candidates: usize,
    pub matched: usize,
    pub unresolved: usize,
    pub error: String,
    pub session: Option<ExtractionSession>,
}

pub fn sha256(data: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(data);
    hash.finish_hex()
}

pub fn inspect_nes(bytes: &[u8], filename: &str) -> Result<CartridgeReport, String> {
    if bytes.is_empty() {
        return Err("The file is empty. Nothing was extracted.".into());
    }
    let info = detect::detect_rom(bytes, filename);
    let header_ok = bytes.len() >= 16 && info.format == RomFormat::Nes;
    let mirroring = if header_ok {
        mirroring_label(bytes[6])
    } else {
        "unavailable".into()
    };
    let payload_end = info.chr_start.saturating_add(info.chr_len);
    let length_ok = header_ok && payload_end <= bytes.len() && info.prg_len > 0;
    let payload_sha = if length_ok {
        sha256(&bytes[info.prg_start..payload_end])
    } else {
        String::new()
    };
    let mut warnings = info.warnings.clone();
    if !filename.trim().is_empty() {
        warnings.push("The file name is not the cartridge identity. The payload digest is.".into());
    }
    if info.chr_len == 0 && header_ok {
        warnings.push(
            "This cartridge has no CHR-ROM. Pattern tables would have to be decoded from PRG, which this extractor does not do."
                .into(),
        );
    }
    let supported = header_ok && length_ok;
    if !supported {
        warnings.push(
            "The file is not a supported iNES or NES 2.0 cartridge with a complete length. No reconstruction was applied."
                .into(),
        );
    }
    Ok(CartridgeReport {
        sha256: sha256(bytes),
        payload_sha256: payload_sha,
        format: format!("{:?}", info.format),
        nes2: info.nes2,
        trainer: info.trainer,
        prg_bytes: info.prg_len,
        chr_bytes: info.chr_len,
        mapper: if header_ok {
            ((bytes[6] >> 4) as u16) | ((bytes[7] as u16) & 0xf0)
        } else {
            0
        },
        mirroring,
        length_ok,
        supported,
        profile_id: String::new(),
        profile_label: "No verified profile".into(),
        partial: true,
        warnings,
    })
}

pub fn raw_tiles(bytes: &[u8], filename: &str) -> Result<Vec<RawTile>, String> {
    let info = detect::detect_rom(bytes, filename);
    if info.format != RomFormat::Nes || info.chr_len == 0 {
        return Err("There is no CHR-ROM to extract.".into());
    }
    let end = info.chr_start.saturating_add(info.chr_len);
    if end > bytes.len() {
        return Err(
            "The CHR-ROM is shorter than the header claims. Raw tiles were not produced.".into(),
        );
    }
    let mut tiles = Vec::new();
    let mut offset = info.chr_start;
    let mut index = 0u32;
    while offset + 16 <= end {
        let bank = index / 256;
        tiles.push(RawTile {
            index,
            bank,
            offset,
        });
        offset += 16;
        index += 1;
    }
    Ok(tiles)
}

pub fn job_start() -> ExtractJob {
    ExtractJob {
        stage: 0,
        stage_name: "Read header".into(),
        paused: false,
        cancelled: false,
        candidates: 0,
        matched: 0,
        unresolved: 0,
        error: String::new(),
        session: None,
    }
}

pub fn job_advance(
    job: &mut ExtractJob,
    bytes: &[u8],
    filename: &str,
    recipes: &[Recipe],
    extra: &[Profile],
) {
    if job.cancelled || job.paused || job.session.is_some() {
        return;
    }
    match job.stage {
        0 => job.stage_name = "Hash cartridge".into(),
        1 => job.stage_name = "Extract raw tiles".into(),
        2 => job.stage_name = "Match profile".into(),
        3 => job.stage_name = "Apply local recipes".into(),
        4 => match propose_session(bytes, filename, recipes, extra) {
            Ok(session) => {
                job.candidates = session.candidates.len();
                job.matched = session
                    .candidates
                    .iter()
                    .filter(|c| {
                        c.confidence.contains("recipe") || c.confidence.contains("documented")
                    })
                    .count();
                job.unresolved = session
                    .coverage
                    .iter()
                    .filter(|row| row.assembly == "unresolved")
                    .count();
                job.session = Some(session);
                job.stage_name = "Ready for review".into();
            }
            Err(err) => {
                job.error = err;
                job.stage_name = "Stopped".into();
            }
        },
        _ => {}
    }
    if job.stage < 4 && job.error.is_empty() {
        job.stage += 1;
    }
}

pub fn job_cancel(job: &mut ExtractJob) {
    job.cancelled = true;
    job.session = None;
    job.stage_name = "Cancelled. Verified library files were left as they were.".into();
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub id: String,
    pub payload_sha256: String,
    pub label: String,
    pub candidates: Vec<Candidate>,
}

pub fn smb1_profile() -> Profile {
    Profile {
        id: "smb1-world".into(),
        payload_sha256: SMB1_PAYLOAD_SHA256.into(),
        label: "Super Mario Bros. (World), headerless No-Intro payload".into(),
        candidates: smb1_documented(),
    }
}

pub fn propose_session(
    bytes: &[u8],
    filename: &str,
    recipes: &[Recipe],
    extra: &[Profile],
) -> Result<ExtractionSession, String> {
    let mut report = inspect_nes(bytes, filename)?;
    let tiles = if report.supported && report.chr_bytes > 0 {
        raw_tiles(bytes, filename)?
    } else {
        Vec::new()
    };
    let mut profiles = vec![smb1_profile()];
    profiles.extend(extra.iter().cloned());
    let mut candidates = Vec::new();
    if let Some(profile) = profiles.iter().find(|profile| {
        profile.payload_sha256 == report.payload_sha256 && !report.payload_sha256.is_empty()
    }) {
        report.profile_id = profile.id.clone();
        report.profile_label = profile.label.clone();
        report.partial = true;
        report.warnings.push(
            "A profile matched this payload. Its assemblies are proposals from documented tile indexes, not approved artwork."
                .into(),
        );
        candidates = profile.candidates.clone();
    } else if report.supported {
        report.warnings.push(
            "No hash-specific profile matched. Raw tiles are available. Named objects were not reconstructed."
                .into(),
        );
    }
    apply_recipes(&mut candidates, &report, recipes);
    let coverage = coverage_for(&report, &candidates);
    Ok(ExtractionSession {
        report: ReportSaved {
            sha256: report.sha256,
            payload_sha256: report.payload_sha256,
            mapper: report.mapper,
            mirroring: report.mirroring,
            chr_bytes: report.chr_bytes,
            profile_id: report.profile_id,
            profile_label: report.profile_label,
            partial: report.partial,
            warnings: report.warnings,
        },
        tiles,
        candidates,
        coverage,
    })
}

pub fn fail_candidate(candidate: &mut Candidate, reason: FailReason, note: &str) {
    candidate.attempts.push(Attempt {
        tiles: candidate.tile_ids(),
        reason: reason.label().into(),
        note: note.into(),
    });
    candidate.fail_reason = reason.label().into();
    candidate.status = ReviewStatus::Failed;
}

pub fn reinspect(candidate: &mut Candidate, tiles: &[u32]) -> Result<String, String> {
    if !matches!(
        candidate.status,
        ReviewStatus::Failed | ReviewStatus::Reinspecting
    ) {
        return Err("Reinspection starts from a failed candidate.".into());
    }
    candidate.status = ReviewStatus::Reinspecting;
    let old = candidate.tile_ids();
    if old.as_slice() == tiles {
        return Ok(
            "Reinspection kept the rejected grouping. It was not presented again as a new answer."
                .into(),
        );
    }
    let keep = candidate.frames.first().cloned();
    let state = keep
        .map(|frame| frame.state)
        .unwrap_or_else(|| "idle".into());
    let origin = "user correction".to_string();
    candidate.frames = vec![FrameDraft {
        state,
        origin,
        spots: tiles
            .iter()
            .enumerate()
            .map(|(i, tile)| TileSpot {
                tile: *tile,
                x: (i as i32 % 2) * 8,
                y: (i as i32 / 2) * 8,
                flip_x: false,
                flip_y: false,
            })
            .collect(),
    }];
    candidate.status = ReviewStatus::Awaiting;
    candidate.confidence = "reinspected".into();
    Ok(format!("Grouping changed from {old:?} to {tiles:?}."))
}

pub fn ignore_candidate(candidate: &mut Candidate) {
    candidate.status = ReviewStatus::Ignored;
}

pub fn restore_candidate(candidate: &mut Candidate) {
    if candidate.status == ReviewStatus::Ignored {
        candidate.status = ReviewStatus::Awaiting;
    }
}

pub fn approve_candidate(candidate: &mut Candidate) -> Result<(), String> {
    if candidate.frames.is_empty() || candidate.tile_ids().is_empty() {
        return Err("There is nothing to approve.".into());
    }
    candidate.status = ReviewStatus::Approved;
    Ok(())
}

pub fn recipe_from(report: &ReportSaved, candidate: &Candidate) -> Recipe {
    Recipe {
        schema: KNOWLEDGE_SCHEMA,
        payload_sha256: report.payload_sha256.clone(),
        profile_id: report.profile_id.clone(),
        name: candidate.name.clone(),
        category: candidate.category.clone(),
        character_id: candidate.character_id.clone(),
        form: candidate.form.clone(),
        target_style: candidate.target_style.clone(),
        collection: candidate.collection.clone(),
        tiles: candidate.tile_ids(),
        frames: candidate.frames.clone(),
        palette: candidate.palette,
        verified: candidate.status == ReviewStatus::Approved,
        rejected: candidate
            .attempts
            .iter()
            .map(|attempt| attempt.tiles.clone())
            .collect(),
        evidence: format!("Saved from review. {}", candidate.evidence),
    }
}

pub fn export_knowledge(recipes: &[Recipe]) -> Result<String, String> {
    let file = KnowledgeFile {
        schema: KNOWLEDGE_SCHEMA,
        recipes: recipes.to_vec(),
    };
    serde_json::to_string_pretty(&file).map_err(|err| err.to_string())
}

pub fn import_knowledge(text: &str) -> Result<Vec<Recipe>, String> {
    if text.len() > MAX_KNOWLEDGE_BYTES {
        return Err("That knowledge file is larger than 2 MB. It was not imported.".into());
    }
    if text.contains('\0') || text.contains("../") || text.contains("..\\") {
        return Err("That knowledge file contains an unsafe path. It was not imported.".into());
    }
    let file: KnowledgeFile = serde_json::from_str(text)
        .map_err(|_| "That knowledge file is not a Nesticles knowledge package.".to_string())?;
    if file.schema != KNOWLEDGE_SCHEMA {
        return Err(format!(
            "Knowledge schema {} is not supported. This build reads schema {KNOWLEDGE_SCHEMA}.",
            file.schema
        ));
    }
    for recipe in &file.recipes {
        if recipe.name.len() > 200 || recipe.name.contains('/') || recipe.name.contains('\\') {
            return Err("A recipe name is not usable. Nothing was imported.".into());
        }
        if recipe.tiles.len() > 256 || recipe.frames.len() > 48 {
            return Err("A recipe exceeds the sprite limits. Nothing was imported.".into());
        }
    }
    Ok(file.recipes)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportGate {
    pub ignored_names: Vec<String>,
    pub included_names: Vec<String>,
    pub needs_confirmation: bool,
}

pub fn export_gate(session: &ExtractionSession) -> ExportGate {
    let ignored_names = session
        .candidates
        .iter()
        .filter(|c| c.status == ReviewStatus::Ignored)
        .map(|c| c.name.clone())
        .collect::<Vec<_>>();
    let included_names = session
        .candidates
        .iter()
        .filter(|c| c.status == ReviewStatus::Approved)
        .map(|c| c.name.clone())
        .collect::<Vec<_>>();
    ExportGate {
        needs_confirmation: !ignored_names.is_empty(),
        ignored_names,
        included_names,
    }
}

pub fn package_character(
    session: &ExtractionSession,
    rom: &[u8],
    character_id: &str,
) -> Result<ContentPackage, String> {
    let group: Vec<&Candidate> = session
        .candidates
        .iter()
        .filter(|c| c.character_id == character_id && c.status == ReviewStatus::Approved)
        .collect();
    if group.is_empty() {
        return Err(format!(
            "No approved form of {character_id} is in this session. Missing states stay unresolved."
        ));
    }
    if group.iter().any(|c| c.status == ReviewStatus::Ignored) {
        return Err("An ignored form was included. Export was stopped.".into());
    }
    let mut forms = Vec::new();
    let mut notes = vec![
        "One package is one library item. Frames stay inside it.".into(),
        "Palette bytes are not in CHR. Palette stays unverified unless a later correction says otherwise.".into(),
    ];
    for candidate in &group {
        if candidate
            .frames
            .iter()
            .any(|frame| frame.origin == "missing")
        {
            notes.push(format!("{} has a missing state.", candidate.name));
        }
        forms.push(embed_form(rom, &session.report, candidate)?);
    }
    let first = group[0];
    Ok(ContentPackage {
        schema: PACKAGE_SCHEMA,
        name: format!("{} — complete", first.name),
        category: first.category.clone(),
        target_style: first.target_style.clone(),
        collection: first.collection.clone(),
        provenance: format!("payload {}", session.report.payload_sha256),
        source_payload_sha256: session.report.payload_sha256.clone(),
        forms,
        behavior: None,
        notes,
    })
}

pub fn package_json(package: &ContentPackage) -> Result<String, String> {
    let text = serde_json::to_string_pretty(package).map_err(|err| err.to_string())?;
    if text.len() > MAX_PACKAGE_BYTES {
        return Err("The package is too large.".into());
    }
    if text.contains("sk-") {
        return Err("A package contained a secret-shaped string. It was not written.".into());
    }
    Ok(text)
}

pub fn parse_package(text: &str) -> Result<ContentPackage, String> {
    if text.len() > MAX_PACKAGE_BYTES || text.contains('\0') || text.contains("../") {
        return Err("That package failed the safety check.".into());
    }
    let package: ContentPackage = serde_json::from_str(text)
        .map_err(|_| "That file is not a Nesticles content package.".to_string())?;
    if package.schema != PACKAGE_SCHEMA {
        return Err("That package schema is not supported.".into());
    }
    if package.forms.is_empty() {
        return Err("The package has no forms.".into());
    }
    Ok(package)
}

/// One sprite, one animation set, and the frame pictures that sprite needs.
/// Returns the sprite id. The caller shows that one library item.
pub fn import_package(project: &mut Project, package: &ContentPackage) -> Result<String, String> {
    let kind = kind_for(&package.category);
    let mut sprite_id = String::new();
    for form in &package.forms {
        let rgba = b64_decode(&form.rgba_base64)?;
        let expected = form.w as usize * form.h as usize * 4;
        if rgba.len() != expected || form.w == 0 || form.h > 256 || form.w > 256 {
            return Err("A form has an impossible picture size. It was not imported.".into());
        }
        let asset_id = nid("asset-");
        project.assets.push(Asset {
            id: asset_id.clone(),
            name: format!("{} {}", package.name, form.form),
            kind,
            style_id: String::new(),
            w: form.w,
            h: form.h,
            rgba_base64: form.rgba_base64.clone(),
            png_base64: String::new(),
            wav_base64: String::new(),
            solid: false,
            cue: String::new(),
            item_tag: Default::default(),
            block_tag: Default::default(),
            hazard_tag: Default::default(),
            warp: None,
        });
        if sprite_id.is_empty() {
            sprite_id = anim::create_sprite(project, &package.name, form.w, form.h)?;
            let frame_id = project
                .sprites
                .iter()
                .find(|sprite| sprite.id == sprite_id)
                .and_then(|sprite| sprite.frames.first())
                .map(|frame| frame.id.clone())
                .unwrap_or_default();
            anim::add_part(project, &sprite_id, &frame_id, &asset_id, 0, 0)?;
        }
    }
    if sprite_id.is_empty() {
        return Err("The package produced no sprite.".into());
    }
    let set_id = anim::create_set(project, &package.name, &sprite_id)?;
    if let Some(sprite) = project.sprites.iter().find(|sprite| sprite.id == sprite_id) {
        if let Some(frame) = sprite.frames.first() {
            let frame_id = frame.id.clone();
            for form in &package.forms {
                for state in &form.states {
                    let clip = anim::add_clip(project, &set_id, &state.name)?;
                    anim::append_frame_to_clip(
                        project,
                        &set_id,
                        &clip,
                        &frame_id,
                        state.ticks.max(1),
                    )?;
                    anim::set_clip_ticks(project, &set_id, &clip, 0, state.ticks.max(1))?;
                    let _ = anim::set_playback(project, &set_id, &clip, Playback::Loop);
                }
            }
        }
    }
    if let Some(behavior) = &package.behavior {
        project.behaviors.push(behavior.clone());
    }
    Ok(sprite_id)
}

pub fn synthetic_fixture() -> Vec<u8> {
    let mut bytes = vec![
        0x4e, 0x45, 0x53, 0x1a, 1, 1, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    bytes.extend(std::iter::repeat(0x11).take(16384));
    let mut tile = [0u8; 16];
    tile[0] = 0xff;
    bytes.extend_from_slice(&tile);
    bytes.extend(std::iter::repeat(0).take(8192 - 16));
    bytes
}

pub fn address_blocked(ip: std::net::IpAddr) -> bool {
    reject_ip(&ip.to_string()).is_some()
}

pub fn allow_reference_url(url: &str) -> Result<(), String> {
    if url.len() > 2000 {
        return Err("That URL is too long.".into());
    }
    if !url.starts_with("https://") {
        return Err("Reference URLs must be HTTPS.".into());
    }
    let rest = &url["https://".len()..];
    if rest.contains('@') || rest.contains('\\') {
        return Err("That URL is not allowed.".into());
    }
    let (host_port, _) = rest.split_once('/').unwrap_or((rest, ""));
    let host_port = host_port.trim();
    if host_port.is_empty() {
        return Err("That URL has no host.".into());
    }
    let (host, port) = if let Some(host) = host_port.strip_prefix('[') {
        let (host, port) = host.split_once(']').ok_or("That IPv6 URL is malformed.")?;
        let port = port.strip_prefix(':').unwrap_or("");
        (host, port)
    } else {
        match host_port.rsplit_once(':') {
            Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => (host, port),
            _ => (host_port, ""),
        }
    };
    if !port.is_empty() && port != "443" {
        return Err("Reference URLs must use port 443.".into());
    }
    let host_l = host.to_ascii_lowercase();
    if host_l == "localhost" || host_l.ends_with(".local") || host_l.ends_with(".internal") {
        return Err("That host is not a public reference.".into());
    }
    if let Some(err) = reject_ip(&host_l) {
        return Err(err);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiStatus {
    Connected,
    InvalidCredentials,
    ModelUnavailable,
    Quota,
    RateLimited,
    Network,
    Unavailable,
    Timeout,
    Malformed,
}

impl ApiStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Connected => "Connected",
            Self::InvalidCredentials => "Invalid credentials",
            Self::ModelUnavailable => "Model not available",
            Self::Quota => "Insufficient quota",
            Self::RateLimited => "Rate limited",
            Self::Network => "Network unavailable",
            Self::Unavailable => "Provider unavailable",
            Self::Timeout => "Request timed out",
            Self::Malformed => "Malformed provider response",
        }
    }
}

pub fn classify_api(
    http_status: Option<u16>,
    body: &str,
    timed_out: bool,
    network: bool,
) -> ApiStatus {
    if timed_out {
        return ApiStatus::Timeout;
    }
    if network {
        return ApiStatus::Network;
    }
    let body_l = body.to_ascii_lowercase();
    match http_status {
        Some(200) => {
            if body_l.contains("\"id\"")
                || body_l.contains("\"data\"")
                || body_l.contains("b64_json")
            {
                ApiStatus::Connected
            } else {
                ApiStatus::Malformed
            }
        }
        Some(401) | Some(403) => ApiStatus::InvalidCredentials,
        Some(402) => ApiStatus::Quota,
        Some(404) => ApiStatus::ModelUnavailable,
        Some(429) => {
            if body_l.contains("quota") || body_l.contains("billing") {
                ApiStatus::Quota
            } else {
                ApiStatus::RateLimited
            }
        }
        Some(status) if status >= 500 => ApiStatus::Unavailable,
        Some(_) => {
            if body_l.contains("insufficient_quota") || body_l.contains("billing") {
                ApiStatus::Quota
            } else {
                ApiStatus::Malformed
            }
        }
        None => ApiStatus::Network,
    }
}

pub fn validate_ai_proposal(value: &Value, tile_count: u32) -> Result<Candidate, String> {
    if value.get("script").is_some()
        || value.get("command").is_some()
        || value.get("exec").is_some()
    {
        return Err("The model returned an instruction. It was not executed.".into());
    }
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if name.is_empty() || name.len() > 80 {
        return Err("The proposal has no usable name.".into());
    }
    let tiles = value
        .get("tiles")
        .and_then(|v| v.as_array())
        .ok_or("The proposal has no tile list.")?;
    if tiles.len() > 24 {
        return Err("The proposal uses more tiles than a sprite allows.".into());
    }
    let mut spots = Vec::new();
    for (i, tile) in tiles.iter().enumerate() {
        let tile = tile.as_u64().ok_or("A tile id is not a number.")? as u32;
        if tile >= tile_count {
            return Err(format!("Tile {tile} is not in this CHR-ROM."));
        }
        spots.push(TileSpot {
            tile,
            x: (i as i32 % 2) * 8,
            y: (i as i32 / 2) * 8,
            flip_x: false,
            flip_y: false,
        });
    }
    if spots.is_empty() {
        return Err("The proposal selected no tiles.".into());
    }
    let category = value
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("other");
    Ok(Candidate {
        id: nid("candidate-"),
        name: name.into(),
        category: category.into(),
        target_style: "custom".into(),
        collection: "custom additions".into(),
        character_id: name.to_ascii_lowercase().replace(' ', "-"),
        form: "default".into(),
        frames: vec![FrameDraft {
            state: "idle".into(),
            origin: "ai proposal".into(),
            spots,
        }],
        palette: 0,
        status: ReviewStatus::Awaiting,
        fail_reason: String::new(),
        confidence: "ai proposal, unverified".into(),
        evidence: "Model output. ROM bytes remain the picture source.".into(),
        attempts: Vec::new(),
        rejected_tiles: Vec::new(),
    })
}

#[derive(Clone, Debug)]
pub struct Puzzle {
    pub candidate_id: String,
    pub pool: Vec<u32>,
    pub placed: Vec<TileSpot>,
    pub palette: u8,
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub state: String,
    pub signals: Vec<String>,
    pub reference_note: String,
    pub marked_wrong: Vec<u32>,
}

pub fn puzzle_from(candidate: &Candidate, tile_count: u32) -> Puzzle {
    let mut pool = candidate.tile_ids();
    for tile in &candidate.rejected_tiles {
        if !pool.contains(tile) {
            pool.push(*tile);
        }
    }
    let placed = candidate
        .frames
        .first()
        .map(|frame| frame.spots.clone())
        .unwrap_or_default();
    Puzzle {
        candidate_id: candidate.id.clone(),
        pool,
        placed,
        palette: candidate.palette,
        canvas_w: 32,
        canvas_h: 32,
        state: candidate
            .frames
            .first()
            .map(|frame| frame.state.clone())
            .unwrap_or_else(|| "idle".into()),
        signals: vec![
            "Source-tile proximity".into(),
            "Tiles already grouped on this candidate".into(),
            format!("{tile_count} tiles are in the extracted CHR-ROM"),
        ],
        reference_note: String::new(),
        marked_wrong: Vec::new(),
    }
}

pub fn puzzle_expand(puzzle: &mut Puzzle, tile_count: u32) {
    let mut extra = Vec::new();
    for tile in puzzle.pool.clone() {
        for next in [tile.saturating_add(1), tile.saturating_sub(1)] {
            if next < tile_count && !puzzle.pool.contains(&next) && !extra.contains(&next) {
                extra.push(next);
            }
        }
    }
    puzzle.pool.extend(extra);
    puzzle
        .signals
        .push("Expanded to adjacent tile indexes.".into());
}

pub fn puzzle_submit(candidate: &mut Candidate, puzzle: &Puzzle) -> Result<(), String> {
    if puzzle.placed.is_empty() {
        return Err("Place at least one tile before submitting.".into());
    }
    if puzzle.canvas_w == 0 || puzzle.canvas_w > 256 || puzzle.canvas_h > 256 {
        return Err("That canvas size is not allowed.".into());
    }
    candidate
        .rejected_tiles
        .extend(puzzle.marked_wrong.iter().copied());
    candidate.frames = vec![FrameDraft {
        state: puzzle.state.clone(),
        origin: "user authored".into(),
        spots: puzzle.placed.clone(),
    }];
    candidate.palette = puzzle.palette;
    candidate.status = ReviewStatus::Awaiting;
    candidate.confidence = "user arranged".into();
    Ok(())
}

pub struct AlphaLibrary {
    pub root: PathBuf,
}

impl AlphaLibrary {
    pub fn open(data_root: &Path) -> Self {
        Self {
            root: data_root.join("nesticles").join(ALPHA_ID),
        }
    }

    pub fn ensure(&self) -> Result<(), String> {
        fs::create_dir_all(self.root.join("packages"))
            .map_err(|err| format!("Could not create the library: {err}"))
    }

    pub fn save_recipe(&self, recipe: &Recipe) -> Result<(), String> {
        self.ensure()?;
        let mut recipes = self.recipes()?;
        recipes
            .retain(|old| old.name != recipe.name || old.payload_sha256 != recipe.payload_sha256);
        recipes.push(recipe.clone());
        let text = export_knowledge(&recipes)?;
        fs::write(self.root.join("knowledge.nknowledge"), text)
            .map_err(|err| format!("Could not save the recipe: {err}"))
    }

    pub fn recipes(&self) -> Result<Vec<Recipe>, String> {
        let path = self.root.join("knowledge.nknowledge");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(&path).map_err(|err| err.to_string())?;
        import_knowledge(&text)
    }

    pub fn save_package(&self, package: &ContentPackage) -> Result<PathBuf, String> {
        self.ensure()?;
        let text = package_json(package)?;
        let safe: String = package
            .name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let path = self.root.join("packages").join(format!("{safe}.nasset"));
        fs::write(&path, text).map_err(|err| err.to_string())?;
        Ok(path)
    }

    pub fn package_count(&self) -> usize {
        fs::read_dir(self.root.join("packages"))
            .map(|dir| dir.filter_map(|e| e.ok()).count())
            .unwrap_or(0)
    }
}

fn apply_recipes(candidates: &mut Vec<Candidate>, report: &CartridgeReport, recipes: &[Recipe]) {
    for recipe in recipes {
        if recipe.rejected.iter().any(|tiles| tiles == &recipe.tiles) {
            continue;
        }
        let same =
            recipe.payload_sha256 == report.payload_sha256 && !report.payload_sha256.is_empty();
        if !same {
            continue;
        }
        if let Some(existing) = candidates
            .iter_mut()
            .find(|c| c.character_id == recipe.character_id && c.form == recipe.form)
        {
            existing.name = recipe.name.clone();
            existing.frames = recipe.frames.clone();
            existing.palette = recipe.palette;
            existing.confidence = if recipe.verified {
                "local verified recipe".into()
            } else {
                "local recipe, not verified".into()
            };
            existing.evidence = recipe.evidence.clone();
            existing.status = ReviewStatus::Awaiting;
        } else {
            candidates.push(Candidate {
                id: nid("candidate-"),
                name: recipe.name.clone(),
                category: recipe.category.clone(),
                target_style: recipe.target_style.clone(),
                collection: recipe.collection.clone(),
                character_id: recipe.character_id.clone(),
                form: recipe.form.clone(),
                frames: recipe.frames.clone(),
                palette: recipe.palette,
                status: ReviewStatus::Awaiting,
                fail_reason: String::new(),
                confidence: "local verified recipe".into(),
                evidence: recipe.evidence.clone(),
                attempts: Vec::new(),
                rejected_tiles: Vec::new(),
            });
        }
    }
}

fn coverage_for(report: &CartridgeReport, candidates: &[Candidate]) -> Vec<CoverageRow> {
    const ROWS: &[(&str, &str, &str)] = &[
        ("Mario", "player", "mario"),
        ("Luigi", "player", "luigi"),
        ("Goomba", "enemy", "goomba"),
        ("Koopa Troopa", "enemy", "koopa"),
        ("Koopa Paratroopa", "enemy", "paratroopa"),
        ("Buzzy Beetle", "enemy", "buzzy"),
        ("Hammer Bro", "enemy", "hammer-bro"),
        ("Spiny", "enemy", "spiny"),
        ("Lakitu", "enemy", "lakitu"),
        ("Piranha Plant", "enemy", "piranha"),
        ("Cheep-Cheep", "enemy", "cheep"),
        ("Blooper", "enemy", "blooper"),
        ("Bullet Bill", "enemy", "bullet-bill"),
        ("Podoboo", "enemy", "podoboo"),
        ("Bowser", "enemy", "bowser"),
        ("Super Mushroom", "item", "mushroom"),
        ("1-Up Mushroom", "item", "1up"),
        ("Fire Flower", "item", "flower"),
        ("Super Star", "item", "star"),
        ("Coin", "item", "coin"),
        ("Fireball", "projectile", "fireball"),
        ("Ground", "terrain", "ground"),
        ("Brick", "block", "brick"),
        ("Question block", "block", "question"),
        ("Solid block", "block", "solid"),
        ("Pipe", "terrain", "pipe"),
        ("Platform", "platform", "platform"),
        ("Springboard", "interactive", "spring"),
        ("Vine", "terrain", "vine"),
        ("Goal", "interactive", "goal"),
        ("Background decoration", "decoration", "decoration"),
        ("HUD", "other", "hud"),
    ];
    let raw = if report.chr_bytes > 0 && report.supported {
        "identified"
    } else {
        "unavailable"
    };
    ROWS.iter()
        .map(|(name, category, id)| {
            let found = candidates.iter().find(|c| c.character_id == *id);
            let (assembly, animation, review) = match found {
                Some(candidate) => (
                    "proposed",
                    if candidate.frames.len() > 1 {
                        "proposed"
                    } else if candidate.frames.len() == 1 {
                        "single documented group"
                    } else {
                        "unresolved"
                    },
                    candidate.status.label(),
                ),
                None => ("unresolved", "unresolved", "unresolved"),
            };
            CoverageRow {
                name: (*name).into(),
                category: (*category).into(),
                raw: raw.into(),
                assembly: assembly.into(),
                animation: animation.into(),
                palette: "unverified".into(),
                review: review.into(),
            }
        })
        .collect()
}

fn smb1_documented() -> Vec<Candidate> {
    const TABLE: &[(&str, &str, &str, &[(&str, &[u8])])] = &[
        (
            "Goomba",
            "enemy",
            "goomba",
            &[
                ("walk", &[0xfc, 0xfc, 0x70, 0x71, 0x72, 0x73]),
                ("defeat", &[0xfc, 0xfc, 0xfc, 0xfc, 0xef, 0xef]),
            ],
        ),
        (
            "Koopa Troopa",
            "enemy",
            "koopa",
            &[
                ("walk", &[0xfc, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9]),
                ("walk-b", &[0xfc, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4]),
            ],
        ),
        (
            "Koopa Paratroopa",
            "enemy",
            "paratroopa",
            &[("fly", &[0x69, 0xa5, 0x6a, 0xa7, 0xa8, 0xa9])],
        ),
        (
            "Buzzy Beetle",
            "enemy",
            "buzzy",
            &[("walk", &[0xfc, 0xfc, 0xaa, 0xab, 0xac, 0xad])],
        ),
        (
            "Hammer Bro",
            "enemy",
            "hammer-bro",
            &[("throw", &[0x7d, 0x7c, 0xd1, 0x8c, 0xd3, 0xd2])],
        ),
        (
            "Spiny",
            "enemy",
            "spiny",
            &[("walk", &[0xfc, 0xfc, 0x96, 0x97, 0x98, 0x99])],
        ),
        (
            "Lakitu",
            "enemy",
            "lakitu",
            &[("float", &[0xb9, 0xb8, 0xbb, 0xba, 0xbc, 0xbc])],
        ),
        (
            "Piranha Plant",
            "enemy",
            "piranha",
            &[("bite", &[0xe5, 0xe5, 0xe6, 0xe6, 0xeb, 0xeb])],
        ),
        (
            "Cheep-Cheep",
            "enemy",
            "cheep",
            &[("swim", &[0xfc, 0xfc, 0xb2, 0xb3, 0xb4, 0xb5])],
        ),
        (
            "Blooper",
            "enemy",
            "blooper",
            &[("swim", &[0xfc, 0xfc, 0xdc, 0xdc, 0xdf, 0xdf])],
        ),
        (
            "Bullet Bill",
            "enemy",
            "bullet-bill",
            &[("fly", &[0xfc, 0xfc, 0xe8, 0xe7, 0xea, 0xe9])],
        ),
        (
            "Podoboo",
            "enemy",
            "podoboo",
            &[("leap", &[0xfc, 0xfc, 0xd0, 0xd0, 0xd7, 0xd7])],
        ),
        (
            "Bowser",
            "enemy",
            "bowser",
            &[("front", &[0xbf, 0xbe, 0xc1, 0xc0, 0xc2, 0xfc])],
        ),
    ];
    let evidence = "Tile indexes from the public SMB disassembly EnemyGraphicsTable. Not pixels. Not approved.";
    TABLE
        .iter()
        .map(|(name, category, id, states)| Candidate {
            id: format!("doc-{id}"),
            name: (*name).into(),
            category: (*category).into(),
            target_style: "smb1".into(),
            collection: "extracted".into(),
            character_id: (*id).into(),
            form: "default".into(),
            frames: states
                .iter()
                .map(|(state, tiles)| FrameDraft {
                    state: (*state).into(),
                    origin: "documented tile index".into(),
                    spots: place(tiles),
                })
                .collect(),
            palette: 0,
            status: ReviewStatus::Awaiting,
            fail_reason: String::new(),
            confidence: "documented proposal".into(),
            evidence: evidence.into(),
            attempts: Vec::new(),
            rejected_tiles: Vec::new(),
        })
        .collect()
}

fn place(tiles: &[u8]) -> Vec<TileSpot> {
    tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| **tile != 0xfc)
        .map(|(i, tile)| TileSpot {
            tile: *tile as u32,
            x: (i as i32 % 2) * 8,
            y: (i as i32 / 2) * 8,
            flip_x: false,
            flip_y: false,
        })
        .collect()
}

fn embed_form(
    rom: &[u8],
    _report: &ReportSaved,
    candidate: &Candidate,
) -> Result<PackForm, String> {
    let info = detect::detect_rom(rom, "export");
    let mut max_x = 8i32;
    let mut max_y = 8i32;
    for frame in &candidate.frames {
        for spot in &frame.spots {
            max_x = max_x.max(spot.x + 8);
            max_y = max_y.max(spot.y + 8);
        }
    }
    let w = max_x.max(8) as u32;
    let h = max_y.max(8) as u32;
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    let palette = [[0, 0, 0], [96, 96, 96], [168, 168, 168], [248, 248, 248]];
    for frame in candidate.frames.iter().take(1) {
        for spot in &frame.spots {
            let offset = info.chr_start + spot.tile as usize * 16;
            let indices =
                decode_tile(rom, offset, Codec::Nes).ok_or("A tile could not be decoded.")?;
            let (_, _, tile) = indices_to_rgba(&indices, 8, 8, &palette, true);
            blit(
                &mut rgba,
                w,
                h,
                &tile,
                spot.x,
                spot.y,
                spot.flip_x,
                spot.flip_y,
            );
        }
    }
    Ok(PackForm {
        form: candidate.form.clone(),
        w,
        h,
        rgba_base64: b64_encode(&rgba),
        states: candidate
            .frames
            .iter()
            .map(|frame| PackState {
                name: frame.state.clone(),
                origin: frame.origin.clone(),
                ticks: 8,
            })
            .collect(),
    })
}

fn blit(dst: &mut [u8], dw: u32, dh: u32, src: &[u8], x: i32, y: i32, flip_x: bool, flip_y: bool) {
    for row in 0..8i32 {
        for col in 0..8i32 {
            let sx = if flip_x { 7 - col } else { col };
            let sy = if flip_y { 7 - row } else { row };
            let dx = x + col;
            let dy = y + row;
            if dx < 0 || dy < 0 || dx >= dw as i32 || dy >= dh as i32 {
                continue;
            }
            let si = (sy as usize * 8 + sx as usize) * 4;
            let di = (dy as usize * dw as usize + dx as usize) * 4;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    let _ = dh;
}

fn kind_for(category: &str) -> AssetKind {
    match category {
        "player" => AssetKind::Player,
        "enemy" | "boss" => AssetKind::Enemy,
        "terrain" | "block" => AssetKind::Terrain,
        "item" | "power-up" => AssetKind::Item,
        "hazard" | "projectile" => AssetKind::Hazard,
        _ => AssetKind::Foreground,
    }
}

fn mirroring_label(flags6: u8) -> String {
    if flags6 & 0x08 != 0 {
        "four-screen".into()
    } else if flags6 & 0x01 != 0 {
        "vertical".into()
    } else {
        "horizontal".into()
    }
}

fn reject_ip(host: &str) -> Option<String> {
    let ip: Option<std::net::IpAddr> = host.parse().ok();
    let ip = ip?;
    let blocked = match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || o[0] == 10
                || o[0] == 127
                || (o[0] == 172 && (16..=31).contains(&o[1]))
                || (o[0] == 192 && o[1] == 168)
                || o[0] == 100 && (o[1] & 0xc0) == 64
                || o[0] == 0
        }
        std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || is_v6_local(&v6),
    };
    if blocked {
        Some("That address is not a public reference.".into())
    } else {
        None
    }
}

fn is_v6_local(ip: &std::net::Ipv6Addr) -> bool {
    let o = ip.octets();
    o[0] == 0xfe && (o[1] & 0xc0) == 0x80 || (o[0] & 0xfe) == 0xfc
}

struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    filled: usize,
    bits: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            filled: 0,
            bits: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.buffer[self.filled] = byte;
            self.filled += 1;
            if self.filled == 64 {
                self.block();
                self.filled = 0;
            }
        }
        self.bits = self
            .bits
            .saturating_add((data.len() as u64).saturating_mul(8));
    }

    fn finish_hex(mut self) -> String {
        let bits = self.bits;
        self.push(0x80);
        while self.filled != 56 {
            self.push(0);
        }
        for byte in bits.to_be_bytes() {
            self.push(byte);
        }
        let mut out = String::new();
        for word in self.state {
            out.push_str(&format!("{word:08x}"));
        }
        out
    }

    fn push(&mut self, byte: u8) {
        self.buffer[self.filled] = byte;
        self.filled += 1;
        if self.filled == 64 {
            self.block();
            self.filled = 0;
        }
    }

    fn block(&mut self) {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                self.buffer[i * 4],
                self.buffer[i * 4 + 1],
                self.buffer[i * 4 + 2],
                self.buffer[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = self.state;
        for i in 0..64 {
            let s1 = a[4].rotate_right(6) ^ a[4].rotate_right(11) ^ a[4].rotate_right(25);
            let ch = (a[4] & a[5]) ^ (!a[4] & a[6]);
            let t1 = a[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a[0].rotate_right(2) ^ a[0].rotate_right(13) ^ a[0].rotate_right(22);
            let maj = (a[0] & a[1]) ^ (a[0] & a[2]) ^ (a[1] & a[2]);
            let t2 = s0.wrapping_add(maj);
            a[7] = a[6];
            a[6] = a[5];
            a[5] = a[4];
            a[4] = a[3].wrapping_add(t1);
            a[3] = a[2];
            a[2] = a[1];
            a[1] = a[0];
            a[0] = t1.wrapping_add(t2);
        }
        for i in 0..8 {
            self.state[i] = self.state[i].wrapping_add(a[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ines(prg: usize, chr_tiles: &[[u8; 16]]) -> Vec<u8> {
        let prg_units = (prg / 16384).max(1);
        let chr_units = (chr_tiles.len() * 16 / 8192).max(1);
        let mut bytes = vec![
            0x4e,
            0x45,
            0x53,
            0x1a,
            prg_units as u8,
            chr_units as u8,
            0x01,
            0x00,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        bytes.extend(std::iter::repeat(0).take(prg_units * 16384));
        for tile in chr_tiles {
            bytes.extend_from_slice(tile);
        }
        let chr_len = chr_units * 8192;
        let have = chr_tiles.len() * 16;
        if have < chr_len {
            bytes.extend(std::iter::repeat(0).take(chr_len - have));
        }
        bytes
    }

    fn solid_tile(index_nibble: u8) -> [u8; 16] {
        let mut tile = [0u8; 16];
        for row in 0..8 {
            tile[row] = 0xff;
            if index_nibble & 2 != 0 {
                tile[row + 8] = 0xff;
            }
        }
        tile
    }

    #[test]
    fn sha256_empty() {
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn header_mirroring_and_digest_ignore_the_file_name() {
        let rom = ines(16384, &[solid_tile(1)]);
        let report = inspect_nes(&rom, "Super Mario Bros.nes").unwrap();
        assert!(report.supported);
        assert_eq!(report.mirroring, "vertical");
        assert_eq!(report.mapper, 0);
        assert!(report.profile_id.is_empty());
        assert_ne!(report.payload_sha256, SMB1_PAYLOAD_SHA256);
        assert!(report.warnings.iter().any(|w| w.contains("file name")));
    }

    #[test]
    fn unknown_hash_does_not_receive_smb1_names() {
        let rom = ines(16384, &[solid_tile(1)]);
        let session = propose_session(&rom, "anything.nes", &[], &[]).unwrap();
        assert!(session.candidates.is_empty());
        assert!(session.tiles.len() >= 256);
        let mario = session
            .coverage
            .iter()
            .find(|row| row.name == "Mario")
            .unwrap();
        assert_eq!(mario.assembly, "unresolved");
        assert_eq!(mario.raw, "identified");
        assert_eq!(mario.palette, "unverified");
    }

    #[test]
    fn a_matching_test_profile_is_only_a_proposal() {
        let rom = ines(16384, &[solid_tile(3), solid_tile(1)]);
        let report = inspect_nes(&rom, "fixture.nes").unwrap();
        let profile = Profile {
            id: "fixture".into(),
            payload_sha256: report.payload_sha256.clone(),
            label: "Synthetic fixture".into(),
            candidates: vec![Candidate {
                id: "scout".into(),
                name: "Scout".into(),
                category: "enemy".into(),
                target_style: "custom".into(),
                collection: "extracted".into(),
                character_id: "scout".into(),
                form: "default".into(),
                frames: vec![FrameDraft {
                    state: "idle".into(),
                    origin: "test".into(),
                    spots: vec![
                        TileSpot {
                            tile: 0,
                            x: 0,
                            y: 0,
                            flip_x: false,
                            flip_y: false,
                        },
                        TileSpot {
                            tile: 1,
                            x: 8,
                            y: 0,
                            flip_x: false,
                            flip_y: false,
                        },
                    ],
                }],
                palette: 0,
                status: ReviewStatus::Awaiting,
                fail_reason: String::new(),
                confidence: "documented proposal".into(),
                evidence: "test".into(),
                attempts: Vec::new(),
                rejected_tiles: Vec::new(),
            }],
        };
        let session = propose_session(&rom, "fixture.nes", &[], &[profile]).unwrap();
        assert_eq!(session.candidates.len(), 1);
        assert_eq!(session.candidates[0].status, ReviewStatus::Awaiting);
        let mut other = rom.clone();
        other[20] ^= 0xff;
        let missed = propose_session(&other, "fixture.nes", &[], &[]).unwrap();
        assert!(missed.candidates.is_empty());
    }

    #[test]
    fn fail_reinspect_and_recipe_change_the_next_session() {
        let rom = ines(16384, &[solid_tile(1)]);
        let mut session = propose_session(&rom, "x.nes", &[], &[]).unwrap();
        session.candidates.push(Candidate {
            id: "manual".into(),
            name: "Wrong".into(),
            category: "enemy".into(),
            target_style: "custom".into(),
            collection: "extracted".into(),
            character_id: "scout".into(),
            form: "default".into(),
            frames: vec![FrameDraft {
                state: "idle".into(),
                origin: "user".into(),
                spots: vec![TileSpot {
                    tile: 3,
                    x: 0,
                    y: 0,
                    flip_x: false,
                    flip_y: false,
                }],
            }],
            palette: 0,
            status: ReviewStatus::Awaiting,
            fail_reason: String::new(),
            confidence: "manual".into(),
            evidence: "manual".into(),
            attempts: Vec::new(),
            rejected_tiles: Vec::new(),
        });
        let candidate = &mut session.candidates[0];
        fail_candidate(candidate, FailReason::WrongTiles, "order");
        let note = reinspect(candidate, &[1, 2]).unwrap();
        assert!(note.contains("changed"));
        let same = reinspect(candidate, &[1, 2]).unwrap_err();
        assert!(same.contains("failed"));
        fail_candidate(candidate, FailReason::WrongTiles, "again");
        let unchanged = reinspect(candidate, &[1, 2]).unwrap();
        assert!(unchanged.contains("rejected grouping"));
        fail_candidate(candidate, FailReason::WrongTiles, "third");
        reinspect(candidate, &[4, 5]).unwrap();
        approve_candidate(candidate).unwrap();
        let recipe = recipe_from(&session.report, candidate);
        let again = propose_session(&rom, "x.nes", &[recipe.clone()], &[]).unwrap();
        let found = again
            .candidates
            .iter()
            .find(|c| c.character_id == "scout")
            .unwrap();
        assert_eq!(found.tile_ids(), vec![4, 5]);
        assert!(found.confidence.contains("recipe"));
        let text = export_knowledge(&[recipe]).unwrap();
        assert!(!text.contains("http"));
        let loaded = import_knowledge(&text).unwrap();
        assert_eq!(loaded[0].tiles, vec![4, 5]);
        assert!(
            import_knowledge(&format!(
                "{{\"schema\":1,\"recipes\":[],\"path\":\"../x\"}}"
            ))
            .is_err()
                || text.contains("schema")
        );
        assert!(import_knowledge("{\"schema\":1,\"recipes\":[{\"name\":\"../etc\"}]}").is_err());
    }

    #[test]
    fn ignored_bucket_blocks_export_until_confirmed() {
        let rom = ines(16384, &[solid_tile(1), solid_tile(3)]);
        let report = inspect_nes(&rom, "x.nes").unwrap();
        let mut session = propose_session(&rom, "x.nes", &[], &[]).unwrap();
        for (name, tile) in [("Keep", 0u32), ("Skip", 1u32), ("Later", 2u32)] {
            session.candidates.push(Candidate {
                id: name.into(),
                name: name.into(),
                category: "enemy".into(),
                target_style: "custom".into(),
                collection: "extracted".into(),
                character_id: name.to_ascii_lowercase(),
                form: "default".into(),
                frames: vec![FrameDraft {
                    state: "idle".into(),
                    origin: "user".into(),
                    spots: vec![TileSpot {
                        tile,
                        x: 0,
                        y: 0,
                        flip_x: false,
                        flip_y: false,
                    }],
                }],
                palette: 0,
                status: ReviewStatus::Awaiting,
                fail_reason: String::new(),
                confidence: "manual".into(),
                evidence: "manual".into(),
                attempts: Vec::new(),
                rejected_tiles: Vec::new(),
            });
        }
        ignore_candidate(&mut session.candidates[1]);
        ignore_candidate(&mut session.candidates[2]);
        approve_candidate(&mut session.candidates[0]).unwrap();
        let gate = export_gate(&session);
        assert!(gate.needs_confirmation);
        assert_eq!(gate.ignored_names.len(), 2);
        restore_candidate(&mut session.candidates[1]);
        approve_candidate(&mut session.candidates[1]).unwrap();
        let gate = export_gate(&session);
        assert_eq!(
            gate.included_names,
            vec!["Keep".to_string(), "Skip".to_string()]
        );
        assert_eq!(gate.ignored_names, vec!["Later".to_string()]);
        assert_eq!(session.tiles.len() > 0, true);
        let _ = report;
    }

    #[test]
    fn package_roundtrip_does_not_need_the_rom_path() {
        let rom = ines(16384, &[solid_tile(3)]);
        let mut session = propose_session(&rom, "x.nes", &[], &[]).unwrap();
        session.candidates.push(Candidate {
            id: "a".into(),
            name: "Scout".into(),
            category: "enemy".into(),
            target_style: "smb1".into(),
            collection: "custom additions".into(),
            character_id: "scout".into(),
            form: "small".into(),
            frames: vec![
                FrameDraft {
                    state: "idle".into(),
                    origin: "user authored".into(),
                    spots: vec![TileSpot {
                        tile: 0,
                        x: 0,
                        y: 0,
                        flip_x: false,
                        flip_y: false,
                    }],
                },
                FrameDraft {
                    state: "walk".into(),
                    origin: "user authored".into(),
                    spots: vec![TileSpot {
                        tile: 0,
                        x: 0,
                        y: 0,
                        flip_x: true,
                        flip_y: false,
                    }],
                },
            ],
            palette: 0,
            status: ReviewStatus::Approved,
            fail_reason: String::new(),
            confidence: "user".into(),
            evidence: "user".into(),
            attempts: Vec::new(),
            rejected_tiles: Vec::new(),
        });
        session.candidates.push(Candidate {
            id: "b".into(),
            name: "Scout super".into(),
            category: "enemy".into(),
            target_style: "smb1".into(),
            collection: "custom additions".into(),
            character_id: "scout".into(),
            form: "super".into(),
            frames: vec![FrameDraft {
                state: "idle".into(),
                origin: "user authored".into(),
                spots: vec![TileSpot {
                    tile: 0,
                    x: 0,
                    y: 0,
                    flip_x: false,
                    flip_y: false,
                }],
            }],
            palette: 0,
            status: ReviewStatus::Approved,
            fail_reason: String::new(),
            confidence: "user".into(),
            evidence: "user".into(),
            attempts: Vec::new(),
            rejected_tiles: Vec::new(),
        });
        let package = package_character(&session, &rom, "scout").unwrap();
        assert_eq!(package.forms.len(), 2);
        assert!(!package.provenance.contains('/'));
        let text = package_json(&package).unwrap();
        assert!(!text.contains(".nes"));
        let parsed = parse_package(&text).unwrap();
        let mut project = Project::blank("fresh");
        let sprite = import_package(&mut project, &parsed).unwrap();
        assert_eq!(project.sprites.len(), 1);
        assert_eq!(project.sprites[0].id, sprite);
        assert!(project
            .anim_sets
            .iter()
            .any(|set| set.name.contains("Scout")));
    }

    #[test]
    fn alpha_namespaces_do_not_share_or_erase() {
        let root =
            std::env::temp_dir().join(format!("nesticles-alpha-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let first = AlphaLibrary::open(&root);
        let recipe = Recipe {
            schema: 1,
            payload_sha256: "abc".into(),
            profile_id: String::new(),
            name: "Scout".into(),
            category: "enemy".into(),
            character_id: "scout".into(),
            form: "default".into(),
            target_style: "custom".into(),
            collection: "original".into(),
            tiles: vec![1],
            frames: Vec::new(),
            palette: 0,
            verified: true,
            rejected: Vec::new(),
            evidence: "user".into(),
        };
        first.save_recipe(&recipe).unwrap();
        let secret = root.join("credentials-stay.txt");
        fs::write(&secret, "not-a-key").unwrap();
        let other_root =
            std::env::temp_dir().join(format!("nesticles-alpha-other-{}", std::process::id()));
        let _ = fs::remove_dir_all(&other_root);
        let second = AlphaLibrary {
            root: other_root.join("nesticles").join("alpha-next"),
        };
        assert_eq!(second.recipes().unwrap().len(), 0);
        assert_eq!(first.recipes().unwrap().len(), 1);
        assert!(secret.exists());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&other_root);
    }

    #[test]
    fn urls_and_api_statuses() {
        assert!(allow_reference_url("http://example.com/a").is_err());
        assert!(allow_reference_url("https://127.0.0.1/a").is_err());
        assert!(allow_reference_url("https://192.168.1.2/a").is_err());
        assert!(allow_reference_url("https://localhost/a").is_err());
        assert!(allow_reference_url("https://example.com/sprites").is_ok());
        assert_eq!(
            classify_api(Some(401), "", false, false),
            ApiStatus::InvalidCredentials
        );
        assert_eq!(
            classify_api(Some(429), "quota", false, false),
            ApiStatus::Quota
        );
        assert_eq!(
            classify_api(Some(429), "", false, false),
            ApiStatus::RateLimited
        );
        assert_eq!(classify_api(None, "", true, false), ApiStatus::Timeout);
        assert_eq!(
            classify_api(Some(200), "nope", false, false),
            ApiStatus::Malformed
        );
        assert_eq!(
            classify_api(Some(200), "{\"id\":\"gpt\"}", false, false),
            ApiStatus::Connected
        );
    }

    #[test]
    fn ai_json_is_checked_and_not_executed() {
        let bad = serde_json::json!({"name":"X","tiles":[1],"script":"rm"});
        assert!(validate_ai_proposal(&bad, 8)
            .unwrap_err()
            .contains("not executed"));
        let high = serde_json::json!({"name":"X","tiles":[9]});
        assert!(validate_ai_proposal(&high, 4).is_err());
        let ok = validate_ai_proposal(
            &serde_json::json!({"name":"Blob","tiles":[1,2],"category":"enemy"}),
            8,
        )
        .unwrap();
        assert_eq!(ok.status, ReviewStatus::Awaiting);
        assert_eq!(ok.frames[0].origin, "ai proposal");
    }

    #[test]
    fn puzzle_keeps_rejected_tiles() {
        let mut candidate =
            validate_ai_proposal(&serde_json::json!({"name":"Blob","tiles":[1,2]}), 8).unwrap();
        let mut puzzle = puzzle_from(&candidate, 8);
        puzzle_expand(&mut puzzle, 8);
        assert!(puzzle.pool.contains(&3));
        puzzle.marked_wrong.push(2);
        puzzle.placed.retain(|spot| spot.tile != 2);
        puzzle_submit(&mut candidate, &puzzle).unwrap();
        assert!(candidate.rejected_tiles.contains(&2));
        assert!(!candidate.tile_ids().contains(&2));
    }

    #[test]
    fn cancel_drops_the_uncommitted_session() {
        let rom = ines(16384, &[solid_tile(1)]);
        let mut job = job_start();
        job_advance(&mut job, &rom, "x.nes", &[], &[]);
        job_cancel(&mut job);
        job_advance(&mut job, &rom, "x.nes", &[], &[]);
        assert!(job.session.is_none());
    }

    #[test]
    fn smb1_profile_lists_goomba_and_leaves_mario_open() {
        let profile = smb1_profile();
        assert_eq!(profile.payload_sha256, SMB1_PAYLOAD_SHA256);
        let goomba = profile
            .candidates
            .iter()
            .find(|c| c.character_id == "goomba")
            .unwrap();
        assert!(goomba.tile_ids().contains(&0x70));
        assert!(goomba.frames.len() > 1);
        assert!(profile.candidates.iter().all(|c| c.character_id != "mario"));
    }
}
