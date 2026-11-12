//! Turn one Nesticles course into a Godot project.
//!
//! Pictures are decoded from the asset's stored pixels and written as PNG.
//! The asset record itself is not modified, and no ROM is opened.
//! Scripts copied into the project are the fixed templates. Names and other
//! project text are written only as JSON data.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::blueprint::ensure_legacy_blueprints;
use crate::engine::res_1080p::{World, FP};
use crate::physics::map_character;
use crate::pngutil::b64_decode;
use crate::project::{Asset, AssetKind, Level, Project};

use super::parity::{sim_file_from_world, skipped_enemy_kinds, SimFile};

/// Screen pixels along one stored course cell.
///
/// This is the 1080p play tile. Saved Nesticles coordinates stay in cells.
/// The Godot step uses 4096 fixed-point units per one of these pixels.
pub const CELL_PX: i32 = 72;

const PLAYER_SOURCE: &str =
    "Course spawn and character preset, stepped with the 1080p SMB1-style rules. Temporary adapter, not a Player Blueprint.";

const NOT_SIMULATED: &[&str] = &[
    "pipes",
    "vines",
    "springs",
    "hazards",
    "items",
    "power_ups",
    "question_blocks",
    "non_goomba_enemies",
];

const PROJECT_GODOT: &str = r#"; Nesticles Godot proof. This file is fixed and contains no project text.
config_version=5

[application]

config/name="Nesticles Godot Proof"
run/main_scene="res://scenes/proof.tscn"
config/features=PackedStringArray("4.7", "GL Compatibility")

[display]

window/size/viewport_width=1280
window/size/viewport_height=720
window/stretch/mode="canvas_items"

[physics]

common/physics_ticks_per_second=60

[rendering]

textures/canvas_textures/default_texture_filter=0
renderer/rendering_method="gl_compatibility"
environment/defaults/default_clear_color=Color(0.16, 0.22, 0.26, 1)
"#;

const PLAYER_SCRIPT: &str = include_str!("player.gd");
const PROOF_SCRIPT: &str = include_str!("proof_driver.gd");
const SIM_SCRIPT: &str = include_str!("nesticles_sim.gd");
const EXPORT_PRESETS: &str = include_str!("export_presets.cfg");

#[derive(Clone, Debug, PartialEq)]
pub struct ProofInfo {
    pub directory: PathBuf,
    pub level_id: String,
    pub player_asset_id: String,
    pub solid_count: usize,
    pub files: Vec<String>,
}

#[derive(Serialize)]
struct LevelFile {
    format: u32,
    cell_px: i32,
    player_source: &'static str,
    preset: String,
    player: BodyFile,
    support_top: Option<f64>,
    drop_px: f64,
    solids: Vec<BodyFile>,
    enemies: Vec<BodyFile>,
    goals: Vec<BodyFile>,
    not_simulated: &'static [&'static str],
    skipped_enemies: Vec<String>,
    sim: SimFile,
    /// Non-solid terrain pictures. They have no collision body.
    #[serde(default)]
    scenery: Vec<BodyFile>,
    /// Resolved terrain cells. Empty on courses that have no terrain brushes.
    #[serde(default)]
    terrain_expect: Vec<TerrainExpect>,
}

#[derive(Serialize)]
struct TerrainExpect {
    cell_x: i32,
    cell_y: i32,
    x: f64,
    y: f64,
    texture: String,
    solid: bool,
    asset_id: String,
}

#[derive(Serialize)]
struct BodyFile {
    asset_id: String,
    label: String,
    cell_x: f64,
    cell_y: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    scale_x: f64,
    scale_y: f64,
    texture: String,
}

struct Plan {
    level_id: String,
    level_file: LevelFile,
    images: BTreeMap<String, Vec<u8>>,
    scene: String,
}

struct SpriteSize {
    w: f64,
    h: f64,
    scale_x: f64,
    scale_y: f64,
}

/// Directory the editor uses for the experimental preview.
pub fn preview_dir() -> PathBuf {
    std::env::temp_dir().join("nesticles-godot-preview")
}

/// Write a Godot project for `level_id`. The Nesticles project is not changed.
pub fn generate_proof(project: &Project, level_id: &str, dest: &Path) -> Result<ProofInfo, String> {
    let plan = plan_proof(project, level_id)?;
    write_plan(dest, &plan)?;
    let mut files = listed_files(dest)?;
    files.sort();
    Ok(ProofInfo {
        directory: dest.to_path_buf(),
        level_id: plan.level_id,
        player_asset_id: plan.level_file.player.asset_id,
        solid_count: plan.level_file.solids.len(),
        files,
    })
}

fn plan_proof(project: &Project, level_id: &str) -> Result<Plan, String> {
    let view = gameplay_view(project);
    let level = view
        .levels
        .iter()
        .find(|level| level.id == level_id)
        .ok_or_else(|| format!("No course with id {level_id}. The Godot scene was not created."))?;
    validate_references(&view, level)?;
    let player = player_asset(&view)?;
    let (player_px_w, player_px_h, player_rgba) = decode_image(player).map_err(|err| {
        format!("The player picture cannot be decoded ({err}). The Godot scene was not created.")
    })?;
    let world = World::from_level(&view, level, map_character(view.character_physics));
    let sim = sim_file_from_world(&world);
    if sim.solids.is_empty() {
        return Err(
            "This course has no placed solid fixture. Place a solid blueprint before the Godot proof. No scene was created."
                .into(),
        );
    }
    let mut picture_ids = vec![player.id.clone()];
    for solid in &sim.solids {
        picture_ids.push(
            picture_for_cell(&view, level, solid.cell_x, solid.cell_y)?
                .id
                .clone(),
        );
    }
    for enemy in &world.enemies {
        if enemy.kind != crate::engine::res_1080p::rules::smb1::enemies::Kind::Goomba {
            continue;
        }
        picture_ids.push(enemy.asset_id.clone());
    }
    for goal in &sim.goals {
        let cell_x = goal.x / crate::engine::res_1080p::TILE_FP;
        let cell_y = goal.y / crate::engine::res_1080p::TILE_FP;
        if let Some(asset) = picture_for_cell(&view, level, cell_x, cell_y).ok() {
            picture_ids.push(asset.id.clone());
        }
    }
    for cell in &level.cells {
        if cell.terrain.is_some() {
            picture_ids.push(cell.asset_id.clone());
        }
    }
    let mut stems = stem_map(&picture_ids);
    let player_stem = stems
        .get(&player.id)
        .cloned()
        .ok_or_else(|| "The player picture has no safe file name.".to_string())?;
    let player_size = fit_box(
        player_px_w,
        player_px_h,
        screen_px(sim.player.w),
        screen_px(sim.player.h),
    );
    let player_body = BodyFile {
        asset_id: player.id.clone(),
        label: player.name.clone(),
        cell_x: f64::from(level.player.x),
        cell_y: f64::from(level.player.y),
        x: screen_px(sim.player.x),
        y: screen_px(sim.player.y),
        w: player_size.w,
        h: player_size.h,
        scale_x: player_size.scale_x,
        scale_y: player_size.scale_y,
        texture: format!("res://assets/{player_stem}.png"),
    };
    let mut solids = Vec::new();
    let mut solid_cells = BTreeSet::new();
    for solid in &sim.solids {
        if !solid_cells.insert((solid.cell_x, solid.cell_y)) {
            continue;
        }
        let asset = picture_for_cell(&view, level, solid.cell_x, solid.cell_y)?;
        let (px_w, px_h, _) = decode_image(asset).map_err(|err| {
            format!(
                "Solid fixture at {},{} has no usable picture ({err}). The Godot scene was not created.",
                solid.cell_x, solid.cell_y
            )
        })?;
        let size = fit_box(px_w, px_h, screen_px(solid.w), screen_px(solid.h));
        let stem = stems
            .get(&asset.id)
            .cloned()
            .unwrap_or_else(|| sanitize(&asset.id));
        solids.push(BodyFile {
            asset_id: asset.id.clone(),
            label: asset.name.clone(),
            cell_x: f64::from(solid.cell_x),
            cell_y: f64::from(solid.cell_y),
            x: screen_px(solid.x),
            y: screen_px(solid.y),
            w: size.w,
            h: size.h,
            scale_x: size.scale_x,
            scale_y: size.scale_y,
            texture: format!("res://assets/{stem}.png"),
        });
    }
    let mut enemies = Vec::new();
    for enemy in world
        .enemies
        .iter()
        .filter(|enemy| enemy.kind == crate::engine::res_1080p::rules::smb1::enemies::Kind::Goomba)
    {
        let asset = view
            .assets
            .iter()
            .find(|asset| asset.id == enemy.asset_id)
            .ok_or_else(|| {
                format!(
                    "Goomba at cell {},{} uses missing asset {}. The Godot scene was not created.",
                    enemy.cell_x, enemy.cell_y, enemy.asset_id
                )
            })?;
        let (px_w, px_h, _) = decode_image(asset).map_err(|err| {
            format!(
                "Goomba at {},{} has no usable picture ({err}). The Godot scene was not created.",
                enemy.cell_x, enemy.cell_y
            )
        })?;
        let size = fit_box(px_w, px_h, screen_px(enemy.w), screen_px(enemy.h));
        let stem = stems
            .get(&asset.id)
            .cloned()
            .unwrap_or_else(|| sanitize(&asset.id));
        enemies.push(BodyFile {
            asset_id: asset.id.clone(),
            label: asset.name.clone(),
            cell_x: f64::from(enemy.cell_x),
            cell_y: f64::from(enemy.cell_y),
            x: screen_px(enemy.x),
            y: screen_px(enemy.y),
            w: size.w,
            h: size.h,
            scale_x: size.scale_x,
            scale_y: size.scale_y,
            texture: format!("res://assets/{stem}.png"),
        });
    }
    let mut goals = Vec::new();
    for goal in &sim.goals {
        let cell_x = goal.x / crate::engine::res_1080p::TILE_FP;
        let cell_y = goal.y / crate::engine::res_1080p::TILE_FP;
        let Ok(asset) = picture_for_cell(&view, level, cell_x, cell_y) else {
            continue;
        };
        let Ok((px_w, px_h, _)) = decode_image(asset) else {
            continue;
        };
        let size = fit_box(px_w, px_h, screen_px(goal.w), screen_px(goal.h));
        let stem = stems
            .get(&asset.id)
            .cloned()
            .unwrap_or_else(|| sanitize(&asset.id));
        goals.push(BodyFile {
            asset_id: asset.id.clone(),
            label: asset.name.clone(),
            cell_x: f64::from(cell_x),
            cell_y: f64::from(cell_y),
            x: screen_px(goal.x),
            y: screen_px(goal.y),
            w: size.w,
            h: size.h,
            scale_x: size.scale_x,
            scale_y: size.scale_y,
            texture: format!("res://assets/{stem}.png"),
        });
    }
    let mut terrain_cells: Vec<&crate::project::Cell> = level
        .cells
        .iter()
        .filter(|cell| cell.terrain.is_some())
        .collect();
    terrain_cells.sort_by_key(|cell| (cell.y, cell.x, cell.asset_id.as_str()));
    let mut scenery = Vec::new();
    let mut terrain_expect = Vec::new();
    for cell in terrain_cells {
        let asset = view
            .assets
            .iter()
            .find(|asset| asset.id == cell.asset_id)
            .ok_or_else(|| {
                format!(
                    "Terrain at {},{} uses missing asset {}. The Godot scene was not created.",
                    cell.x, cell.y, cell.asset_id
                )
            })?;
        let (px_w, px_h, _) = decode_image(asset).map_err(|err| {
            format!(
                "Terrain at {},{} has no usable picture ({err}). The Godot scene was not created.",
                cell.x, cell.y
            )
        })?;
        let size = fit_box(px_w, px_h, f64::from(CELL_PX), f64::from(CELL_PX));
        let stem = stems
            .get(&asset.id)
            .cloned()
            .unwrap_or_else(|| sanitize(&asset.id));
        let texture = format!("res://assets/{stem}.png");
        let x = f64::from(cell.x.saturating_mul(CELL_PX));
        let y = f64::from(cell.y.saturating_mul(CELL_PX));
        let solid = solid_cells.contains(&(cell.x, cell.y));
        if !solid {
            scenery.push(BodyFile {
                asset_id: asset.id.clone(),
                label: asset.name.clone(),
                cell_x: f64::from(cell.x),
                cell_y: f64::from(cell.y),
                x,
                y,
                w: size.w,
                h: size.h,
                scale_x: size.scale_x,
                scale_y: size.scale_y,
                texture: texture.clone(),
            });
        }
        terrain_expect.push(TerrainExpect {
            cell_x: cell.x,
            cell_y: cell.y,
            x,
            y,
            texture,
            solid,
            asset_id: asset.id.clone(),
        });
    }
    let support = support_top(&player_body, &solids);
    let drop_px = support
        .map(|top| (top - (player_body.y + player_body.h)).max(0.0))
        .unwrap_or(0.0);
    let mut images = BTreeMap::new();
    images.insert(
        format!("assets/{player_stem}.png"),
        encode_png(&player_rgba, player_px_w, player_px_h)?,
    );
    let mut seen = BTreeSet::new();
    seen.insert(player.id.clone());
    for body in solids
        .iter()
        .chain(enemies.iter())
        .chain(goals.iter())
        .chain(scenery.iter())
    {
        if !seen.insert(body.asset_id.clone()) {
            continue;
        }
        let asset = view
            .assets
            .iter()
            .find(|asset| asset.id == body.asset_id)
            .expect("actor asset was validated");
        let (w, h, rgba) = decode_image(asset)?;
        let stem = stems
            .remove(&asset.id)
            .unwrap_or_else(|| sanitize(&asset.id));
        images.insert(format!("assets/{stem}.png"), encode_png(&rgba, w, h)?);
    }
    let level_file = LevelFile {
        format: 2,
        cell_px: CELL_PX,
        player_source: PLAYER_SOURCE,
        preset: view.character_physics.label().to_string(),
        player: player_body,
        support_top: support,
        drop_px,
        solids,
        enemies,
        goals,
        not_simulated: NOT_SIMULATED,
        skipped_enemies: skipped_enemy_kinds(&world),
        sim,
        scenery,
        terrain_expect,
    };
    let scene = scene_text(&level_file);
    Ok(Plan {
        level_id: level.id.clone(),
        level_file,
        images,
        scene,
    })
}

fn write_plan(dest: &Path, plan: &Plan) -> Result<(), String> {
    prepare_output(dest)?;
    write_inside(dest, "project.godot", PROJECT_GODOT.as_bytes())?;
    write_inside(dest, "export_presets.cfg", EXPORT_PRESETS.as_bytes())?;
    write_inside(dest, "scripts/player.gd", PLAYER_SCRIPT.as_bytes())?;
    write_inside(dest, "scripts/proof_driver.gd", PROOF_SCRIPT.as_bytes())?;
    write_inside(dest, "scripts/nesticles_sim.gd", SIM_SCRIPT.as_bytes())?;
    write_inside(dest, "scenes/proof.tscn", plan.scene.as_bytes())?;
    let json = serde_json::to_vec_pretty(&plan.level_file)
        .map_err(|err| format!("Could not encode the Godot level data: {err}"))?;
    write_inside(dest, "data/level.json", &json)?;
    for (rel, bytes) in &plan.images {
        write_inside(dest, rel, bytes)?;
    }
    write_inside(dest, ".nesticles-godot-out", b"phase7\n")?;
    Ok(())
}

pub(crate) fn prepare_output(dest: &Path) -> Result<(), String> {
    if !owned_output(dest) {
        if dest.exists() && !dest.is_dir() {
            return Err(format!("{} is not a directory.", dest.display()));
        }
        if dest.exists() && !output_is_empty_or_marked(dest)? {
            return Err(format!(
                "Refusing to write a Godot project into {} because that folder is not a Nesticles Godot output.",
                dest.display()
            ));
        }
    }
    if dest.exists() {
        for entry in
            fs::read_dir(dest).map_err(|err| format!("Could not read {}: {err}", dest.display()))?
        {
            let entry = entry.map_err(|err| err.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)
                    .map_err(|err| format!("Could not clear {}: {err}", path.display()))?;
            } else {
                fs::remove_file(&path)
                    .map_err(|err| format!("Could not clear {}: {err}", path.display()))?;
            }
        }
    } else {
        fs::create_dir_all(dest)
            .map_err(|err| format!("Could not create {}: {err}", dest.display()))?;
    }
    Ok(())
}

fn owned_output(dest: &Path) -> bool {
    dest.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == "nesticles-godot-preview" || name.starts_with("nesticles-godot-")
        })
}

fn output_is_empty_or_marked(dest: &Path) -> Result<bool, String> {
    if dest.join(".nesticles-godot-out").is_file() {
        return Ok(true);
    }
    let mut entries = fs::read_dir(dest).map_err(|err| err.to_string())?;
    Ok(entries.next().is_none())
}

pub(crate) fn write_inside(dest: &Path, rel: &str, bytes: &[u8]) -> Result<(), String> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err("Blocked a Godot output path that leaves the destination.".into());
    }
    let path = dest.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create {}: {err}", parent.display()))?;
    }
    fs::write(&path, bytes).map_err(|err| format!("Could not write {}: {err}", path.display()))?;
    let root = dest
        .canonicalize()
        .map_err(|err| format!("Could not resolve {}: {err}", dest.display()))?;
    let written = path
        .canonicalize()
        .map_err(|err| format!("Could not resolve {}: {err}", path.display()))?;
    if !written.starts_with(&root) {
        let _ = fs::remove_file(&path);
        return Err("Blocked a file that escaped the Godot output directory.".into());
    }
    Ok(())
}

fn gameplay_view(project: &Project) -> Project {
    let mut copy = project.clone();
    if !copy.blueprint_cutover {
        ensure_legacy_blueprints(&mut copy);
    }
    copy
}

fn validate_references(project: &Project, level: &Level) -> Result<(), String> {
    for cell in &level.cells {
        if !project.assets.iter().any(|asset| asset.id == cell.asset_id) {
            return Err(format!(
                "Cell {},{} references missing asset {}. No scene was created.",
                cell.x, cell.y, cell.asset_id
            ));
        }
        if let Some(id) = &cell.blueprint_id {
            if !project
                .blueprints
                .iter()
                .any(|blueprint| &blueprint.id == id)
            {
                return Err(format!(
                    "Cell {},{} references missing blueprint {id}. No scene was created.",
                    cell.x, cell.y
                ));
            }
        }
    }
    Ok(())
}

fn player_asset(project: &Project) -> Result<&Asset, String> {
    project
        .assets
        .iter()
        .find(|asset| asset.kind == AssetKind::Player && has_image(asset))
        .ok_or_else(|| {
            "This project has no player picture. Phase 6 uses the existing player artwork as a temporary stand-in and does not create a Player Blueprint. No scene was created.".into()
        })
}

fn picture_for_cell<'a>(
    project: &'a Project,
    level: &Level,
    cell_x: i32,
    cell_y: i32,
) -> Result<&'a Asset, String> {
    let mut terrain_hit = None;
    let mut solid_hit = None;
    let mut fallback = None;
    for cell in &level.cells {
        if cell.x != cell_x || cell.y != cell_y {
            continue;
        }
        let Some(asset) = project
            .assets
            .iter()
            .find(|asset| asset.id == cell.asset_id)
        else {
            continue;
        };
        if !has_image(asset) {
            continue;
        }
        if cell.terrain.is_some() && terrain_hit.is_none() {
            terrain_hit = Some(asset);
            continue;
        }
        if (asset.kind == AssetKind::Terrain || asset.solid) && solid_hit.is_none() {
            solid_hit = Some(asset);
            continue;
        }
        if fallback.is_none() {
            fallback = Some(asset);
        }
    }
    terrain_hit.or(solid_hit).or(fallback).ok_or_else(|| {
        format!(
            "Solid fixture at {cell_x},{cell_y} has no usable picture. The Godot scene was not created."
        )
    })
}

fn has_image(asset: &Asset) -> bool {
    !asset.rgba_base64.trim().is_empty() || !asset.png_base64.trim().is_empty()
}

fn decode_image(asset: &Asset) -> Result<(u32, u32, Vec<u8>), String> {
    if !asset.rgba_base64.trim().is_empty() {
        let bytes = b64_decode(&asset.rgba_base64)?;
        let need = (asset.w as usize)
            .saturating_mul(asset.h as usize)
            .saturating_mul(4);
        if asset.w == 0 || asset.h == 0 || bytes.len() < need {
            return Err(format!(
                "asset {} RGBA does not match {} by {}",
                asset.id, asset.w, asset.h
            ));
        }
        return Ok((asset.w, asset.h, bytes[..need].to_vec()));
    }
    if !asset.png_base64.trim().is_empty() {
        let bytes = b64_decode(&asset.png_base64)?;
        return decode_png(&bytes);
    }
    Err(format!("asset {} has no image data", asset.id))
}

#[cfg(feature = "png")]
fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    crate::pngutil::png_to_rgba(bytes)
}

#[cfg(not(feature = "png"))]
fn decode_png(_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    Err("PNG decoding is not enabled in this build.".into())
}

#[cfg(feature = "png")]
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    crate::pngutil::rgba_to_png(rgba, width, height)
}

#[cfg(not(feature = "png"))]
fn encode_png(_rgba: &[u8], _width: u32, _height: u32) -> Result<Vec<u8>, String> {
    Err(
        "Godot generation needs the png feature so pictures can be written. No scene was created."
            .into(),
    )
}

fn fit_box(width: u32, height: u32, box_w: f64, box_h: f64) -> SpriteSize {
    SpriteSize {
        w: box_w,
        h: box_h,
        scale_x: box_w / f64::from(width.max(1)),
        scale_y: box_h / f64::from(height.max(1)),
    }
}

fn screen_px(units: i32) -> f64 {
    f64::from(units) / f64::from(FP)
}

fn support_top(player: &BodyFile, solids: &[BodyFile]) -> Option<f64> {
    let left = player.x;
    let right = player.x + player.w;
    let feet = player.y + player.h;
    let mut best: Option<f64> = None;
    for solid in solids {
        let overlaps = right > solid.x + 0.01 && left < solid.x + solid.w - 0.01;
        if !overlaps || solid.y + 0.01 < feet {
            continue;
        }
        best = Some(match best {
            Some(current) if current <= solid.y => current,
            _ => solid.y,
        });
    }
    best
}

fn stem_map(ids: &[String]) -> BTreeMap<String, String> {
    let mut unique = ids.to_vec();
    unique.sort();
    unique.dedup();
    let mut used = BTreeSet::new();
    let mut map = BTreeMap::new();
    for id in unique {
        let mut stem = sanitize(&id);
        if !used.insert(stem.clone()) {
            let mut suffix = 2u32;
            loop {
                let next = format!("{stem}_{suffix}");
                if used.insert(next.clone()) {
                    stem = next;
                    break;
                }
                suffix += 1;
            }
        }
        map.insert(id, stem);
    }
    map
}

fn sanitize(id: &str) -> String {
    let mut out: String = id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect();
    if out.is_empty() {
        out = "asset".into();
    }
    if out.len() > 48 {
        out.truncate(48);
    }
    out
}

fn scene_text(level: &LevelFile) -> String {
    let mut textures = BTreeMap::new();
    textures.insert(
        level.player.texture.clone(),
        texture_id(&level.player.texture),
    );
    for actor in level
        .solids
        .iter()
        .chain(level.enemies.iter())
        .chain(level.goals.iter())
        .chain(level.scenery.iter())
    {
        textures
            .entry(actor.texture.clone())
            .or_insert_with(|| texture_id(&actor.texture));
    }
    let mut shapes = BTreeMap::new();
    shapes.insert(
        shape_id(level.player.w, level.player.h),
        (level.player.w, level.player.h),
    );
    for actor in level
        .solids
        .iter()
        .chain(level.enemies.iter())
        .chain(level.goals.iter())
    {
        shapes.insert(shape_id(actor.w, actor.h), (actor.w, actor.h));
    }
    let load_steps = 2 + textures.len() + shapes.len() + 1;
    let mut scene = String::new();
    scene.push_str(&format!("[gd_scene load_steps={load_steps} format=3]\n\n"));
    scene.push_str(
        "[ext_resource type=\"Script\" path=\"res://scripts/proof_driver.gd\" id=\"1_driver\"]\n",
    );
    scene.push_str(
        "[ext_resource type=\"Script\" path=\"res://scripts/player.gd\" id=\"2_player\"]\n",
    );
    for (path, id) in &textures {
        scene.push_str(&format!(
            "[ext_resource type=\"Texture2D\" path=\"{path}\" id=\"{id}\"]\n"
        ));
    }
    scene.push('\n');
    for (id, (w, h)) in &shapes {
        scene.push_str(&format!(
            "[sub_resource type=\"RectangleShape2D\" id=\"{id}\"]\nsize = Vector2({}, {})\n\n",
            num(*w),
            num(*h)
        ));
    }
    scene.push_str("[node name=\"Proof\" type=\"Node2D\"]\nscript = ExtResource(\"1_driver\")\n\n");
    scene.push_str(&format!(
        "[node name=\"Player\" type=\"Node2D\" parent=\".\"]\nz_index = 2\nposition = Vector2({}, {})\nscript = ExtResource(\"2_player\")\n\n",
        num(level.player.x),
        num(level.player.y)
    ));
    scene.push_str(&format!(
        "[node name=\"Sprite\" type=\"Sprite2D\" parent=\"Player\"]\ncentered = false\ntexture_filter = 1\ntexture = ExtResource(\"{}\")\n",
        textures[&level.player.texture]
    ));
    push_scale(&mut scene, level.player.scale_x, level.player.scale_y);
    scene.push('\n');
    scene.push_str(&format!(
        "[node name=\"Collision\" type=\"CollisionShape2D\" parent=\"Player\"]\nposition = Vector2({}, {})\nshape = SubResource(\"{}\")\n\n",
        num(level.player.w / 2.0),
        num(level.player.h / 2.0),
        shape_id(level.player.w, level.player.h)
    ));
    scene.push_str(&format!(
        "[node name=\"Camera\" type=\"Camera2D\" parent=\"Player\"]\nposition = Vector2({}, {})\nenabled = true\nzoom = Vector2(1, 1)\n\n",
        num(level.player.w / 2.0),
        num(level.player.h / 2.0)
    ));
    for (index, solid) in level.solids.iter().enumerate() {
        scene.push_str(&format!(
            "[node name=\"Solid_{index}\" type=\"StaticBody2D\" parent=\".\"]\nposition = Vector2({}, {})\n\n",
            num(solid.x),
            num(solid.y)
        ));
        scene.push_str(&format!(
            "[node name=\"Sprite\" type=\"Sprite2D\" parent=\"Solid_{index}\"]\ncentered = false\ntexture_filter = 1\ntexture = ExtResource(\"{}\")\n",
            textures[&solid.texture]
        ));
        push_scale(&mut scene, solid.scale_x, solid.scale_y);
        scene.push('\n');
        scene.push_str(&format!(
            "[node name=\"Collision\" type=\"CollisionShape2D\" parent=\"Solid_{index}\"]\nposition = Vector2({}, {})\nshape = SubResource(\"{}\")\n\n",
            num(solid.w / 2.0),
            num(solid.h / 2.0),
            shape_id(solid.w, solid.h)
        ));
    }
    for (index, enemy) in level.enemies.iter().enumerate() {
        push_actor(&mut scene, &textures, "Enemy", index, enemy, 1);
    }
    for (index, goal) in level.goals.iter().enumerate() {
        push_actor(&mut scene, &textures, "Goal", index, goal, 1);
    }
    for (index, piece) in level.scenery.iter().enumerate() {
        push_scenery(&mut scene, &textures, index, piece);
    }
    scene
}

fn push_scenery(
    scene: &mut String,
    textures: &BTreeMap<String, String>,
    index: usize,
    actor: &BodyFile,
) {
    scene.push_str(&format!(
        "[node name=\"Scenery_{index}\" type=\"Node2D\" parent=\".\"]\nz_index = 0\nposition = Vector2({}, {})\n\n",
        num(actor.x),
        num(actor.y)
    ));
    scene.push_str(&format!(
        "[node name=\"Sprite\" type=\"Sprite2D\" parent=\"Scenery_{index}\"]\ncentered = false\ntexture_filter = 1\ntexture = ExtResource(\"{}\")\n",
        textures[&actor.texture]
    ));
    push_scale(scene, actor.scale_x, actor.scale_y);
    scene.push('\n');
}

fn push_actor(
    scene: &mut String,
    textures: &BTreeMap<String, String>,
    kind: &str,
    index: usize,
    actor: &BodyFile,
    z_index: i32,
) {
    scene.push_str(&format!(
        "[node name=\"{kind}_{index}\" type=\"Node2D\" parent=\".\"]\nz_index = {z_index}\nposition = Vector2({}, {})\n\n",
        num(actor.x),
        num(actor.y)
    ));
    scene.push_str(&format!(
        "[node name=\"Sprite\" type=\"Sprite2D\" parent=\"{kind}_{index}\"]\ncentered = false\ntexture_filter = 1\ntexture = ExtResource(\"{}\")\n",
        textures[&actor.texture]
    ));
    push_scale(scene, actor.scale_x, actor.scale_y);
    scene.push('\n');
    scene.push_str(&format!(
        "[node name=\"Collision\" type=\"CollisionShape2D\" parent=\"{kind}_{index}\"]\nposition = Vector2({}, {})\nshape = SubResource(\"{}\")\n\n",
        num(actor.w / 2.0),
        num(actor.h / 2.0),
        shape_id(actor.w, actor.h)
    ));
}

fn push_scale(scene: &mut String, scale_x: f64, scale_y: f64) {
    if (scale_x - 1.0).abs() > 0.001 || (scale_y - 1.0).abs() > 0.001 {
        scene.push_str(&format!(
            "scale = Vector2({}, {})\n",
            num(scale_x),
            num(scale_y)
        ));
    }
}

fn texture_id(path: &str) -> String {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("asset");
    format!("tex_{stem}")
}

fn shape_id(w: f64, h: f64) -> String {
    format!(
        "shape_{}_{}",
        num(w).replace('.', "p"),
        num(h).replace('.', "p")
    )
}

fn num(value: f64) -> String {
    if (value - value.round()).abs() < 1e-6 {
        format!("{}", value.round() as i64)
    } else {
        let text = format!("{value:.4}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn listed_files(dest: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    visit_files(dest, dest, &mut files)?;
    Ok(files)
}

fn visit_files(root: &Path, dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            visit_files(root, &path, files)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|err| err.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            files.push(rel);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint::{assign_cell, compose_fixture, LegacyBlock};
    use crate::demo::demo_project;
    use crate::physics::CharacterPhysics;
    use crate::pngutil::png_to_rgba;
    use crate::project::{Cell, Level, Project, Spawn};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn proof_fixture() -> Project {
        let demo = demo_project();
        let pip = demo
            .assets
            .iter()
            .find(|asset| asset.id == "pip")
            .unwrap()
            .clone();
        let sod = demo
            .assets
            .iter()
            .find(|asset| asset.id == "sod")
            .unwrap()
            .clone();
        let mut project = Project::blank("Proof");
        project.blueprint_cutover = true;
        project.assets.push(pip);
        project.assets.push(sod);
        project.levels.push(Level {
            id: "proof".into(),
            name: "Proof course".into(),
            style_id: String::new(),
            w: 32,
            h: 15,
            sky: "#24343c".into(),
            music_id: None,
            player: Spawn { x: 2.0, y: 6.0 },
            cells: (0..16)
                .map(|x| Cell {
                    x,
                    y: 12,
                    asset_id: "sod".into(),
                    ..Cell::default()
                })
                .collect(),
            enemy_ai: crate::enemy_ai::EnemyAiStyle::Smb1Nes,
            entrances: Vec::new(),
        });
        let role = compose_fixture(Some(LegacyBlock::Solid), None, None).unwrap();
        for x in 0..16 {
            assign_cell(&mut project, "proof", x, 12, role.clone()).unwrap();
        }
        project
    }

    fn fresh_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn fixture_uses_real_pixels_and_cell_scale() {
        let project = proof_fixture();
        let before = serde_json::to_string(&project).unwrap();
        let pip_before = project.assets[0].rgba_base64.clone();
        let dest = fresh_dir("nesticles-godot-fixture");
        let info = generate_proof(&project, "proof", &dest).unwrap();
        assert_eq!(serde_json::to_string(&project).unwrap(), before);
        assert_eq!(project.assets[0].rgba_base64, pip_before);
        assert_eq!(info.player_asset_id, "pip");
        assert_eq!(info.solid_count, 16);
        let level: serde_json::Value =
            serde_json::from_slice(&fs::read(dest.join("data/level.json")).unwrap()).unwrap();
        assert_eq!(level["cell_px"], 72);
        assert_eq!(level["player"]["x"], 153.0);
        assert_eq!(level["player"]["y"], 432.0);
        assert_eq!(level["player"]["w"], 54.0);
        assert_eq!(level["player"]["h"], 72.0);
        assert_eq!(level["support_top"], 864.0);
        assert_eq!(level["drop_px"], 360.0);
        assert_eq!(level["solids"][0]["x"], 0.0);
        assert_eq!(level["solids"][0]["y"], 864.0);
        assert_eq!(level["sim"]["motion"]["jump_slow"], -73728);
        assert_eq!(level["sim"]["player"]["x"], 2 * 72 * 4096 + 9 * 4096);
        assert!(level.get("move_speed").is_none());
        assert!(level["player_source"]
            .as_str()
            .unwrap()
            .contains("not a Player Blueprint"));
        let png = fs::read(dest.join("assets/pip.png")).unwrap();
        let (w, h, rgba) = png_to_rgba(&png).unwrap();
        let original =
            decode_image(project.assets.iter().find(|a| a.id == "pip").unwrap()).unwrap();
        assert_eq!((w, h, rgba.clone()), original);
        assert!(rgba.chunks(4).any(|px| px[3] == 0));
        assert_eq!(
            fs::read_to_string(dest.join("scripts/player.gd")).unwrap(),
            PLAYER_SCRIPT
        );
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn seed_spirit_floor_comes_from_project_data() {
        let project = demo_project();
        assert!(!project.blueprint_cutover);
        let saved = serde_json::to_string(&project).unwrap();
        let dest = fresh_dir("nesticles-godot-seed");
        let info = generate_proof(&project, "moss", &dest).unwrap();
        assert_eq!(serde_json::to_string(&project).unwrap(), saved);
        assert!(!project.blueprint_cutover);
        assert!(info.solid_count > 0);
        assert_eq!(info.player_asset_id, "pip");
        let scene = fs::read_to_string(dest.join("scenes/proof.tscn")).unwrap();
        assert!(scene.contains("res://assets/sod.png"));
        assert!(scene.contains("position = Vector2(153, 792)"));
        assert!(fs::read(dest.join("assets/sod.png"))
            .unwrap()
            .starts_with(b"\x89PNG"));
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn regeneration_is_byte_stable() {
        let project = proof_fixture();
        let first = fresh_dir("nesticles-godot-stable-a");
        let second = fresh_dir("nesticles-godot-stable-b");
        generate_proof(&project, "proof", &first).unwrap();
        generate_proof(&project, "proof", &second).unwrap();
        let mut left = listed_files(&first).unwrap();
        let mut right = listed_files(&second).unwrap();
        left.sort();
        right.sort();
        assert_eq!(left, right);
        for rel in left {
            assert_eq!(
                fs::read(first.join(&rel)).unwrap(),
                fs::read(second.join(&rel)).unwrap(),
                "{rel}"
            );
        }
        let _ = fs::remove_dir_all(&first);
        let _ = fs::remove_dir_all(&second);
    }

    #[test]
    fn malicious_name_cannot_change_scripts_or_escape() {
        let mut project = proof_fixture();
        let payload = "\"; OS.shell_open(\"/tmp/pwned\")\n# ../../etc/passwd";
        project.name = payload.into();
        project.assets[0].name = payload.into();
        project.assets[0].id = "../../tmp/pwned".into();
        project.levels[0].player = Spawn { x: 2.0, y: 6.0 };
        for cell in &mut project.levels[0].cells {
            if cell.asset_id == "pip" {
                cell.asset_id = "../../tmp/pwned".into();
            }
        }
        for blueprint in &mut project.blueprints {
            if blueprint.visual_id == "pip" {
                blueprint.visual_id = "../../tmp/pwned".into();
            }
        }
        let dest = fresh_dir("nesticles-godot-malicious");
        let info = generate_proof(&project, "proof", &dest).unwrap();
        assert_eq!(info.player_asset_id, "../../tmp/pwned");
        assert!(!Path::new("/tmp/pwned").exists());
        assert!(!Path::new("/tmp/pwned.png").exists());
        let root = dest.canonicalize().unwrap();
        for rel in listed_files(&dest).unwrap() {
            assert!(!rel.contains(".."), "{rel}");
            let path = dest.join(&rel).canonicalize().unwrap();
            assert!(path.starts_with(&root), "{rel}");
        }
        assert_eq!(
            fs::read_to_string(dest.join("scripts/player.gd")).unwrap(),
            PLAYER_SCRIPT
        );
        assert_eq!(
            fs::read_to_string(dest.join("scripts/proof_driver.gd")).unwrap(),
            PROOF_SCRIPT
        );
        assert_eq!(
            fs::read_to_string(dest.join("scripts/nesticles_sim.gd")).unwrap(),
            SIM_SCRIPT
        );
        let scene = fs::read_to_string(dest.join("scenes/proof.tscn")).unwrap();
        let project_file = fs::read_to_string(dest.join("project.godot")).unwrap();
        let presets = fs::read_to_string(dest.join("export_presets.cfg")).unwrap();
        for text in [&scene, &project_file, &presets] {
            assert!(!text.contains("OS.shell_open"));
            assert!(!text.contains(payload));
            assert!(!text.contains("../"));
        }
        assert!(scene.contains("res://assets/tmppwned.png"));
        assert!(dest.join("assets/tmppwned.png").is_file());
        let json = fs::read_to_string(dest.join("data/level.json")).unwrap();
        assert!(json.contains("OS.shell_open"));
        assert!(json.contains("\\n"));
        assert!(!json.contains(payload));
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn output_omits_keys_rom_and_editor_files() {
        let mut project = proof_fixture();
        project.name = "sk-LIVEKEY-should-not-leak".into();
        let rom = std::env::temp_dir().join("nesticles-not-a-rom.nes");
        fs::write(&rom, b"NESTICLES_ROM_BYTES").unwrap();
        project.styles.push(crate::project::Style {
            id: "style-secret".into(),
            name: "keys.json".into(),
            source: rom.display().to_string(),
        });
        project.assets[1].wav_base64 = "NESTICLES_ROM_BYTES".into();
        let sibling = std::env::temp_dir().join("nesticles-secret-keys.json");
        fs::write(&sibling, b"sk-FROM-KEYS-FILE").unwrap();
        let dest = fresh_dir("nesticles-godot-secrets");
        generate_proof(&project, "proof", &dest).unwrap();
        let mut blob = String::new();
        for rel in listed_files(&dest).unwrap() {
            assert_ne!(
                Path::new(&rel).file_name().and_then(|n| n.to_str()),
                Some("keys.json")
            );
            assert!(!rel.ends_with(".nes"));
            blob.push_str(&String::from_utf8_lossy(
                &fs::read(dest.join(&rel)).unwrap(),
            ));
        }
        assert!(!blob.contains("sk-LIVEKEY-should-not-leak"));
        assert!(!blob.contains("sk-FROM-KEYS-FILE"));
        assert!(!blob.contains("NESTICLES_ROM_BYTES"));
        assert!(!blob.contains("keys.json"));
        let _ = fs::remove_dir_all(&dest);
        let _ = fs::remove_file(&rom);
        let _ = fs::remove_file(&sibling);
    }

    #[test]
    fn missing_picture_or_blueprint_does_not_write_a_scene() {
        let mut project = proof_fixture();
        project.assets.retain(|asset| asset.id != "sod");
        let dest = fresh_dir("nesticles-godot-missing");
        let err = generate_proof(&project, "proof", &dest).unwrap_err();
        assert!(err.contains("missing asset"));
        assert!(!dest.join("scenes/proof.tscn").exists());

        let mut project = proof_fixture();
        project
            .assets
            .iter_mut()
            .find(|asset| asset.id == "sod")
            .unwrap()
            .rgba_base64
            .clear();
        project
            .assets
            .iter_mut()
            .find(|asset| asset.id == "sod")
            .unwrap()
            .png_base64
            .clear();
        let err = generate_proof(&project, "proof", &dest).unwrap_err();
        assert!(err.contains("no usable picture"));
        assert!(!dest.exists() || !dest.join("scenes/proof.tscn").exists());

        let mut project = proof_fixture();
        project.levels[0].cells[0].blueprint_id = Some("missing-blueprint".into());
        let err = generate_proof(&project, "proof", &dest).unwrap_err();
        assert!(err.contains("missing blueprint"));
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn preset_changes_motion_data_only() {
        let mut project = proof_fixture();
        project.character_physics = CharacterPhysics::Smb1Luigi;
        let dest = fresh_dir("nesticles-godot-luigi");
        generate_proof(&project, "proof", &dest).unwrap();
        let level: serde_json::Value =
            serde_json::from_slice(&fs::read(dest.join("data/level.json")).unwrap()).unwrap();
        assert_eq!(level["sim"]["motion"]["jump_slow"], -81920);
        assert_eq!(level["sim"]["motion"]["max_walk"], 27648);
        assert!(level.get("jump_velocity").is_none());
        assert_eq!(
            fs::read_to_string(dest.join("scripts/player.gd")).unwrap(),
            PLAYER_SCRIPT
        );
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn foreign_directory_is_refused() {
        let project = proof_fixture();
        let dest = std::env::temp_dir().join(format!("nesticles-foreign-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dest);
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("keep.txt"), b"keep").unwrap();
        let err = generate_proof(&project, "proof", &dest).unwrap_err();
        assert!(err.contains("Refusing"));
        assert_eq!(fs::read(dest.join("keep.txt")).unwrap(), b"keep");
        assert!(!dest.join("scenes/proof.tscn").exists());
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn headless_gameplay_and_optional_export() {
        let tool = match crate::godot::discover(None) {
            Ok(tool) => tool,
            Err(err) => panic!(
                "Godot {} is required for the headless proof: {err}",
                crate::godot::PINNED_VERSION
            ),
        };
        let project = proof_fixture();
        let dest = fresh_dir(&format!("nesticles-godot-run-{}", std::process::id()));
        generate_proof(&project, "proof", &dest).unwrap();
        let output = crate::godot::run_proof(&tool.executable, &dest)
            .unwrap_or_else(|err| panic!("headless proof failed to start: {err}"));
        assert!(
            output.stdout.contains("NESTICLES_GODOT_PROOF_OK"),
            "stdout:\n{}\nstderr:\n{}",
            output.stdout,
            output.stderr
        );
        assert!(
            !output.stdout.contains("NESTICLES_GODOT_PROOF_FAIL"),
            "{}\n{}",
            output.stdout,
            output.stderr
        );
        assert_eq!(
            output.status,
            Some(0),
            "{}\n{}",
            output.stdout,
            output.stderr
        );
        let combined = format!("{}\n{}", output.stdout, output.stderr);
        assert!(!combined.contains("SCRIPT ERROR"), "{combined}");

        let exported = dest.join("build").join("nesticles-proof");
        match crate::godot::export_native(&tool.executable, &dest, &exported) {
            Err(err) if err.contains("export templates were not found") => {
                eprintln!("native export blocked: {err}");
            }
            Err(err) => panic!("{err}"),
            Ok(_) => {
                let standalone = crate::godot::run_exported(&exported)
                    .unwrap_or_else(|err| panic!("standalone launch failed: {err}"));
                assert!(
                    standalone.stdout.contains("NESTICLES_GODOT_PROOF_OK"),
                    "stdout:\n{}\nstderr:\n{}",
                    standalone.stdout,
                    standalone.stderr
                );
                assert_eq!(
                    standalone.status,
                    Some(0),
                    "{}\n{}",
                    standalone.stdout,
                    standalone.stderr
                );
            }
        }
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn headless_terrain_uses_connected_tiles() {
        let project = terrain_course();
        let first = fresh_dir(&format!("nesticles-godot-terrain-{}", std::process::id()));
        let second = fresh_dir(&format!(
            "nesticles-godot-terrain-again-{}",
            std::process::id()
        ));
        generate_proof(&project, "course", &first).unwrap();
        generate_proof(&project, "course", &second).unwrap();
        let files = listed_files(&first).unwrap();
        assert_eq!(files, listed_files(&second).unwrap());
        for rel in &files {
            assert_eq!(
                fs::read(first.join(rel)).unwrap(),
                fs::read(second.join(rel)).unwrap(),
                "{rel}"
            );
        }
        let level: serde_json::Value =
            serde_json::from_slice(&fs::read(first.join("data/level.json")).unwrap()).unwrap();
        assert_eq!(level["cell_px"], 72);
        assert_eq!(level["player"]["x"], 153.0);
        assert_eq!(level["player"]["y"], 432.0);
        assert_eq!(level["support_top"], 864.0);
        assert_eq!(level["drop_px"], 360.0);
        assert_eq!(level["solids"].as_array().unwrap().len(), 13);
        let expect = level["terrain_expect"].as_array().unwrap();
        assert_eq!(expect.len(), 22);
        let at = |x: i64, y: i64| {
            expect
                .iter()
                .find(|row| row["cell_x"].as_i64() == Some(x) && row["cell_y"].as_i64() == Some(y))
                .unwrap_or_else(|| panic!("missing terrain {x},{y}"))
        };
        assert!(at(0, 12)["texture"]
            .as_str()
            .unwrap()
            .ends_with("grass_left.png"));
        assert_eq!(at(0, 12)["solid"], true);
        assert_eq!(at(0, 12)["x"], 0.0);
        assert_eq!(at(0, 12)["y"], 864.0);
        assert!(at(1, 12)["texture"]
            .as_str()
            .unwrap()
            .ends_with("grass_mid.png"));
        assert!(at(3, 12)["texture"]
            .as_str()
            .unwrap()
            .ends_with("cracked.png"));
        assert_eq!(at(3, 12)["solid"], true);
        assert_eq!(at(3, 12)["x"], 216.0);
        assert!(at(7, 12)["texture"]
            .as_str()
            .unwrap()
            .ends_with("grass_right.png"));
        assert!(at(8, 12)["texture"]
            .as_str()
            .unwrap()
            .ends_with("stone_iso.png"));
        assert_eq!(
            at(7, 12)["x"].as_f64().unwrap() + 72.0,
            at(8, 12)["x"].as_f64().unwrap()
        );
        assert!(at(22, 9)["texture"]
            .as_str()
            .unwrap()
            .ends_with("stone_top.png"));
        assert!(at(22, 10)["texture"]
            .as_str()
            .unwrap()
            .ends_with("stone_mid.png"));
        assert!(at(22, 12)["texture"]
            .as_str()
            .unwrap()
            .ends_with("stone_bot.png"));
        assert!(at(10, 2)["texture"]
            .as_str()
            .unwrap()
            .ends_with("decor_tl.png"));
        assert!(at(11, 3)["texture"]
            .as_str()
            .unwrap()
            .ends_with("decor_fill.png"));
        assert_eq!(at(11, 3)["solid"], false);
        assert_eq!(at(11, 3)["x"], 792.0);
        assert_eq!(at(11, 3)["y"], 216.0);
        let scene = fs::read_to_string(first.join("scenes/proof.tscn")).unwrap();
        assert!(scene.contains("res://assets/grass_left.png"));
        assert!(scene.contains("res://assets/grass_mid.png"));
        assert!(scene.contains("res://assets/grass_right.png"));
        assert!(scene.contains("res://assets/cracked.png"));
        assert!(scene.contains("res://assets/decor_fill.png"));
        assert!(scene.contains("Scenery_"));
        assert!(scene.contains("texture_filter = 1"));
        assert!(!scene.contains("OS.shell_open"));
        assert_eq!(
            fs::read_to_string(first.join("scripts/player.gd")).unwrap(),
            PLAYER_SCRIPT
        );
        assert_eq!(
            fs::read_to_string(first.join("scripts/proof_driver.gd")).unwrap(),
            PROOF_SCRIPT
        );
        assert_eq!(
            fs::read_to_string(first.join("scripts/nesticles_sim.gd")).unwrap(),
            SIM_SCRIPT
        );
        let tool = match crate::godot::discover(None) {
            Ok(tool) => tool,
            Err(err) => panic!(
                "Godot {} is required for the terrain proof: {err}",
                crate::godot::PINNED_VERSION
            ),
        };
        let output = crate::godot::run_proof(&tool.executable, &first)
            .unwrap_or_else(|err| panic!("headless terrain proof failed to start: {err}"));
        let combined = format!("{}\n{}", output.stdout, output.stderr);
        assert!(
            output.stdout.contains("NESTICLES_TERRAIN_OK cells=22"),
            "{combined}"
        );
        assert!(
            output.stdout.contains("NESTICLES_GODOT_PROOF_OK"),
            "{combined}"
        );
        assert!(!combined.contains("SCRIPT ERROR"), "{combined}");
        assert!(
            !combined.contains("NESTICLES_GODOT_PROOF_FAIL"),
            "{combined}"
        );
        assert_eq!(output.status, Some(0), "{combined}");
        let exported = first.join("build").join("nesticles-proof");
        match crate::godot::export_native(&tool.executable, &first, &exported) {
            Err(err) if err.contains("export templates were not found") => {
                eprintln!("native terrain export blocked: {err}");
            }
            Err(err) => panic!("{err}"),
            Ok(_) => {
                let standalone = crate::godot::run_exported(&exported)
                    .unwrap_or_else(|err| panic!("standalone terrain launch failed: {err}"));
                let text = format!("{}\n{}", standalone.stdout, standalone.stderr);
                assert!(text.contains("NESTICLES_TERRAIN_OK cells=22"), "{text}");
                assert!(text.contains("NESTICLES_GODOT_PROOF_OK"), "{text}");
                assert_eq!(standalone.status, Some(0), "{text}");
            }
        }
        let _ = fs::remove_dir_all(&first);
        let _ = fs::remove_dir_all(&second);
    }

    fn terrain_course() -> Project {
        use crate::enemy_ai::EnemyAiStyle;
        use crate::project::{Asset, AssetKind, BlockTag, HazardTag, ItemTag};
        use crate::terrain::{
            apply_stroke, set_manual_override, NeighborMode, TerrainBrush, TerrainSlot,
        };
        use std::collections::BTreeMap;

        fn swatch(id: &str, rgb: [u8; 3], kind: AssetKind) -> Asset {
            let mut bytes = Vec::with_capacity(16 * 16 * 4);
            for _ in 0..16 * 16 {
                bytes.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
            Asset {
                id: id.into(),
                name: id.into(),
                kind,
                style_id: String::new(),
                w: 16,
                h: 16,
                rgba_base64: crate::pngutil::b64_encode(&bytes),
                png_base64: String::new(),
                wav_base64: String::new(),
                solid: false,
                cue: String::new(),
                item_tag: ItemTag::Auto,
                block_tag: BlockTag::Auto,
                hazard_tag: HazardTag::Auto,
                warp: None,
            }
        }
        fn material(id: &str, group: &str, pairs: &[(TerrainSlot, &str)]) -> TerrainBrush {
            let mut variants = BTreeMap::new();
            for (slot, asset) in pairs {
                variants.insert(*slot, (*asset).to_string());
            }
            TerrainBrush {
                id: id.into(),
                name: "\"; OS.shell_open(\"/tmp/pwned\")".into(),
                group: group.into(),
                mode: NeighborMode::Four,
                variants,
                fallback_asset: pairs[0].1.to_string(),
                default_role: crate::blueprint::LegacyRole::Inert,
                notes: String::new(),
            }
        }
        let grass = material(
            "grass",
            "meadow",
            &[
                (TerrainSlot::HorizontalLeft, "grass_left"),
                (TerrainSlot::HorizontalMiddle, "grass_mid"),
                (TerrainSlot::HorizontalRight, "grass_right"),
                (TerrainSlot::Isolated, "grass_iso"),
            ],
        );
        let stone = material(
            "stone",
            "wall",
            &[
                (TerrainSlot::Isolated, "stone_iso"),
                (TerrainSlot::VerticalTop, "stone_top"),
                (TerrainSlot::VerticalMiddle, "stone_mid"),
                (TerrainSlot::VerticalBottom, "stone_bot"),
            ],
        );
        let decor = material(
            "decor",
            "ornament",
            &[
                (TerrainSlot::TopLeftOuter, "decor_tl"),
                (TerrainSlot::TopEdge, "decor_top"),
                (TerrainSlot::TopRightOuter, "decor_tr"),
                (TerrainSlot::LeftEdge, "decor_left"),
                (TerrainSlot::Interior, "decor_fill"),
                (TerrainSlot::RightEdge, "decor_right"),
                (TerrainSlot::BottomLeftOuter, "decor_bl"),
                (TerrainSlot::BottomEdge, "decor_bot"),
                (TerrainSlot::BottomRightOuter, "decor_br"),
            ],
        );
        let mut project = Project::blank("Terrain proof");
        project.blueprint_cutover = true;
        project
            .assets
            .push(swatch("pip", [80, 160, 255], AssetKind::Player));
        for (id, rgb) in [
            ("grass_left", [40, 140, 50]),
            ("grass_mid", [50, 160, 60]),
            ("grass_right", [30, 120, 40]),
            ("grass_iso", [70, 180, 80]),
            ("cracked", [90, 70, 40]),
            ("stone_iso", [140, 140, 150]),
            ("stone_top", [160, 160, 170]),
            ("stone_mid", [120, 120, 130]),
            ("stone_bot", [100, 100, 110]),
            ("decor_tl", [180, 80, 80]),
            ("decor_top", [190, 90, 90]),
            ("decor_tr", [170, 70, 70]),
            ("decor_left", [200, 100, 100]),
            ("decor_fill", [210, 110, 110]),
            ("decor_right", [160, 60, 60]),
            ("decor_bl", [150, 50, 50]),
            ("decor_bot", [140, 40, 40]),
            ("decor_br", [130, 30, 30]),
        ] {
            project.assets.push(swatch(id, rgb, AssetKind::Terrain));
        }
        project.terrain_brushes = vec![grass, stone, decor];
        project.levels.push(Level {
            id: "course".into(),
            name: "Connected ground".into(),
            style_id: String::new(),
            w: 32,
            h: 15,
            sky: "#24343c".into(),
            music_id: None,
            player: Spawn { x: 2.0, y: 6.0 },
            cells: Vec::new(),
            enemy_ai: EnemyAiStyle::Smb1Nes,
            entrances: Vec::new(),
        });
        let brushes = project.terrain_brushes.clone();
        let solid = compose_fixture(Some(LegacyBlock::Solid), None, None).unwrap();
        let inert = crate::blueprint::LegacyRole::Inert;
        let lay = |project: &mut Project,
                   brush_id: &str,
                   coords: &[(i32, i32)],
                   role: crate::blueprint::LegacyRole| {
            let report = apply_stroke(
                &mut project.levels[0],
                &brushes,
                brush_id,
                coords,
                false,
                None,
            )
            .unwrap();
            for (x, y) in report.created {
                assign_cell(project, "course", x, y, role.clone()).unwrap();
            }
        };
        lay(
            &mut project,
            "grass",
            &(0..=7).map(|x| (x, 12)).collect::<Vec<_>>(),
            solid.clone(),
        );
        set_manual_override(&mut project.levels[0], 3, 12, "cracked").unwrap();
        lay(&mut project, "stone", &[(8, 12)], solid.clone());
        lay(
            &mut project,
            "stone",
            &(9..=12).map(|y| (22, y)).collect::<Vec<_>>(),
            solid,
        );
        let decor_cells: Vec<_> = (2..=4)
            .flat_map(|y| (10..=12).map(move |x| (x, y)))
            .collect();
        lay(&mut project, "decor", &decor_cells, inert);
        project
    }
}
