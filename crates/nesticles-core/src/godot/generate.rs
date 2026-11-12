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

use crate::blueprint::{ensure_legacy_blueprints, LegacyBlock, LegacyRole};
use crate::physics::CharacterPhysics;
use crate::pngutil::b64_decode;
use crate::project::{Asset, AssetKind, Layer, Level, Project};

/// Godot pixels along one course cell.
///
/// Saved Nesticles coordinates stay in cells. This scale is applied only while
/// writing the Godot scene. It is not the 1080p fixed-point tile, and the
/// world is not defined as a 1920 by 1080 screen.
pub const CELL_PX: i32 = 16;

const PLAYER_SOURCE: &str =
    "Course spawn and character preset. Temporary adapter, not a Player Blueprint.";

const PROJECT_GODOT: &str = r#"; Nesticles Godot proof. This file is fixed and contains no project text.
config_version=5

[application]

config/name="Nesticles Godot Proof"
run/main_scene="res://scenes/proof.tscn"
config/features=PackedStringArray("4.7", "GL Compatibility")

[display]

window/size/viewport_width=640
window/size/viewport_height=360
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
    move_speed: f64,
    jump_velocity: f64,
    gravity: f64,
    player: BodyFile,
    support_top: Option<f64>,
    drop_px: f64,
    solids: Vec<BodyFile>,
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
    scale: f64,
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
    scale: f64,
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
    let player_size = fit(player_px_w, player_px_h);
    let mut stems = stem_map(&image_ids(&view, level, &player.id));
    let player_stem = stems
        .get(&player.id)
        .cloned()
        .ok_or_else(|| "The player picture has no safe file name.".to_string())?;
    let mut solids = Vec::new();
    let mut ordered: Vec<&crate::project::Cell> = level.cells.iter().collect();
    ordered.sort_by(|a, b| (a.y, a.x, layer_rank(a.layer)).cmp(&(b.y, b.x, layer_rank(b.layer))));
    for cell in ordered {
        if !is_solid(&view, cell)? {
            continue;
        }
        let asset = view
            .assets
            .iter()
            .find(|asset| asset.id == cell.asset_id)
            .ok_or_else(|| {
                format!(
                    "Solid fixture at {},{} uses missing asset {}. The Godot scene was not created.",
                    cell.x, cell.y, cell.asset_id
                )
            })?;
        let (px_w, px_h, _) = decode_image(asset).map_err(|err| {
            format!(
                "Solid fixture at {},{} has no usable picture ({err}). The Godot scene was not created.",
                cell.x, cell.y
            )
        })?;
        let size = fit(px_w, px_h);
        let stem = stems
            .get(&asset.id)
            .cloned()
            .unwrap_or_else(|| sanitize(&asset.id));
        solids.push(BodyFile {
            asset_id: asset.id.clone(),
            label: asset.name.clone(),
            cell_x: f64::from(cell.x),
            cell_y: f64::from(cell.y),
            x: world(f64::from(cell.x)),
            y: world(f64::from(cell.y)),
            w: size.w,
            h: size.h,
            scale: size.scale,
            texture: format!("res://assets/{stem}.png"),
        });
    }
    if solids.is_empty() {
        return Err(
            "This course has no placed solid fixture. Place a solid blueprint before the Godot proof. No scene was created."
                .into(),
        );
    }
    let player_body = BodyFile {
        asset_id: player.id.clone(),
        label: player.name.clone(),
        cell_x: f64::from(level.player.x),
        cell_y: f64::from(level.player.y),
        x: world(f64::from(level.player.x)),
        y: world(f64::from(level.player.y)),
        w: player_size.w,
        h: player_size.h,
        scale: player_size.scale,
        texture: format!("res://assets/{player_stem}.png"),
    };
    let support = support_top(&player_body, &solids);
    let drop_px = support
        .map(|top| (top - (player_body.y + player_body.h)).max(0.0))
        .unwrap_or(0.0);
    let motion = motion_for(view.character_physics);
    let mut images = BTreeMap::new();
    images.insert(
        format!("assets/{player_stem}.png"),
        encode_png(&player_rgba, player_px_w, player_px_h)?,
    );
    let mut seen = BTreeSet::new();
    seen.insert(player.id.clone());
    for solid in &solids {
        if !seen.insert(solid.asset_id.clone()) {
            continue;
        }
        let asset = view
            .assets
            .iter()
            .find(|asset| asset.id == solid.asset_id)
            .expect("solid asset was validated");
        let (w, h, rgba) = decode_image(asset)?;
        let stem = stems
            .remove(&asset.id)
            .unwrap_or_else(|| sanitize(&asset.id));
        images.insert(format!("assets/{stem}.png"), encode_png(&rgba, w, h)?);
    }
    let level_file = LevelFile {
        format: 1,
        cell_px: CELL_PX,
        player_source: PLAYER_SOURCE,
        preset: view.character_physics.label().to_string(),
        move_speed: motion.move_speed,
        jump_velocity: motion.jump_velocity,
        gravity: motion.gravity,
        player: player_body,
        support_top: support,
        drop_px,
        solids,
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
    write_inside(dest, "scenes/proof.tscn", plan.scene.as_bytes())?;
    let json = serde_json::to_vec_pretty(&plan.level_file)
        .map_err(|err| format!("Could not encode the Godot level data: {err}"))?;
    write_inside(dest, "data/level.json", &json)?;
    for (rel, bytes) in &plan.images {
        write_inside(dest, rel, bytes)?;
    }
    write_inside(dest, ".nesticles-godot-out", b"phase6\n")?;
    Ok(())
}

fn prepare_output(dest: &Path) -> Result<(), String> {
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

fn write_inside(dest: &Path, rel: &str, bytes: &[u8]) -> Result<(), String> {
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

fn is_solid(project: &Project, cell: &crate::project::Cell) -> Result<bool, String> {
    let Some(id) = cell.blueprint_id.as_deref() else {
        return Ok(false);
    };
    let Some(blueprint) = project
        .blueprints
        .iter()
        .find(|blueprint| blueprint.id == id)
    else {
        return Err(format!(
            "Cell {},{} references missing blueprint {id}. No scene was created.",
            cell.x, cell.y
        ));
    };
    Ok(matches!(
        blueprint.legacy_role,
        LegacyRole::Fixture {
            block: Some(LegacyBlock::Solid),
            ..
        }
    ))
}

fn image_ids(project: &Project, level: &Level, player_id: &str) -> Vec<String> {
    let mut ids = vec![player_id.to_string()];
    for cell in &level.cells {
        if matches!(is_solid(project, cell), Ok(true)) {
            ids.push(cell.asset_id.clone());
        }
    }
    ids
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

fn fit(width: u32, height: u32) -> SpriteSize {
    let longest = width.max(height).max(1);
    let scale = f64::from(CELL_PX) / f64::from(longest);
    SpriteSize {
        w: f64::from(width) * scale,
        h: f64::from(height) * scale,
        scale,
    }
}

fn world(cell: f64) -> f64 {
    cell * f64::from(CELL_PX)
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

struct Motion {
    move_speed: f64,
    jump_velocity: f64,
    gravity: f64,
}

/// Pixel rates for the Godot controller.
///
/// These are a small subset so the proof can move and jump. They are not the
/// Rust SMB1 fixed-point speeds, and they are not a claim of SMB1 parity.
fn motion_for(physics: CharacterPhysics) -> Motion {
    match physics {
        CharacterPhysics::Smb1Mario | CharacterPhysics::Smb1Toad => Motion {
            move_speed: 96.0,
            jump_velocity: -280.0,
            gravity: 900.0,
        },
        CharacterPhysics::Smb1Luigi => Motion {
            move_speed: 96.0,
            jump_velocity: -340.0,
            gravity: 820.0,
        },
        CharacterPhysics::Smb1Peach => Motion {
            move_speed: 80.0,
            jump_velocity: -260.0,
            gravity: 900.0,
        },
    }
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

fn layer_rank(layer: Layer) -> u8 {
    match layer {
        Layer::Back => 0,
        Layer::Main => 1,
        Layer::Front => 2,
    }
}

fn scene_text(level: &LevelFile) -> String {
    let mut textures = BTreeMap::new();
    textures.insert(
        level.player.texture.clone(),
        texture_id(&level.player.texture),
    );
    for solid in &level.solids {
        textures
            .entry(solid.texture.clone())
            .or_insert_with(|| texture_id(&solid.texture));
    }
    let mut shapes = BTreeMap::new();
    shapes.insert(
        shape_id(level.player.w, level.player.h),
        (level.player.w, level.player.h),
    );
    for solid in &level.solids {
        shapes.insert(shape_id(solid.w, solid.h), (solid.w, solid.h));
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
        "[node name=\"Player\" type=\"CharacterBody2D\" parent=\".\"]\nposition = Vector2({}, {})\nscript = ExtResource(\"2_player\")\n\n",
        num(level.player.x),
        num(level.player.y)
    ));
    scene.push_str(&format!(
        "[node name=\"Sprite\" type=\"Sprite2D\" parent=\"Player\"]\ncentered = false\ntexture_filter = 1\ntexture = ExtResource(\"{}\")\n",
        textures[&level.player.texture]
    ));
    push_scale(&mut scene, level.player.scale);
    scene.push('\n');
    scene.push_str(&format!(
        "[node name=\"Collision\" type=\"CollisionShape2D\" parent=\"Player\"]\nposition = Vector2({}, {})\nshape = SubResource(\"{}\")\n\n",
        num(level.player.w / 2.0),
        num(level.player.h / 2.0),
        shape_id(level.player.w, level.player.h)
    ));
    scene.push_str(&format!(
        "[node name=\"Camera\" type=\"Camera2D\" parent=\"Player\"]\nposition = Vector2({}, {})\nenabled = true\nzoom = Vector2(2, 2)\n\n",
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
        push_scale(&mut scene, solid.scale);
        scene.push('\n');
        scene.push_str(&format!(
            "[node name=\"Collision\" type=\"CollisionShape2D\" parent=\"Solid_{index}\"]\nposition = Vector2({}, {})\nshape = SubResource(\"{}\")\n\n",
            num(solid.w / 2.0),
            num(solid.h / 2.0),
            shape_id(solid.w, solid.h)
        ));
    }
    scene
}

fn push_scale(scene: &mut String, scale: f64) {
    if (scale - 1.0).abs() > 0.001 {
        scene.push_str(&format!(
            "scale = Vector2({}, {})\n",
            num(scale),
            num(scale)
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
    use crate::blueprint::{assign_cell, compose_fixture};
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
        assert_eq!(level["cell_px"], 16);
        assert_eq!(level["player"]["x"], 32.0);
        assert_eq!(level["player"]["y"], 96.0);
        assert_eq!(level["support_top"], 192.0);
        assert_eq!(level["drop_px"], 80.0);
        assert_eq!(level["solids"][0]["x"], 0.0);
        assert_eq!(level["solids"][0]["y"], 192.0);
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
        assert!(scene.contains("position = Vector2(32, 176)"));
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
        assert_eq!(level["jump_velocity"], -340.0);
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
}
