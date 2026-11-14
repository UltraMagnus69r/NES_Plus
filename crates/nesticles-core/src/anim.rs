//! Style-independent sprite assembly and animation.
//!
//! Pictures stay on [`crate::project::Asset`]. A sprite frame only stores
//! placement. An animation clip only stores frame ids, tick durations, and a
//! playback mode. Nothing here is an egui widget, a GPU texture, or a physics
//! body. Godot can read the same JSON later. Playback in Godot is Phase 19.
//!
//! Durations are 60 Hz simulation ticks. A preview may spend wall-clock time
//! to decide how many ticks to advance. It does not store a monitor refresh rate.

use crate::pngutil::{b64_decode, b64_encode};
use crate::project::{nid, Asset, AssetKind, Project};
use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;
pub const MAX_EDGE: u32 = 256;
pub const MAX_FRAMES: usize = 48;
pub const MAX_PARTS: usize = 24;
pub const MAX_CLIPS: usize = 24;
pub const MAX_SPRITES: usize = 64;
pub const MAX_SETS: usize = 64;
pub const HISTORY_LIMIT: usize = 32;

/// How a clip continues after its frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Playback {
    #[default]
    Loop,
    Once,
    Hold,
    PingPong,
}

impl Playback {
    pub fn label(self) -> &'static str {
        match self {
            Self::Loop => "Loop",
            Self::Once => "Play once",
            Self::Hold => "Hold last frame",
            Self::PingPong => "Ping-pong",
        }
    }

    pub const ALL: [Playback; 4] = [Self::Loop, Self::Once, Self::Hold, Self::PingPong];
}

/// One placed picture on a frame. The pixels stay on the asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    pub id: String,
    pub asset_id: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    /// Higher draws later.
    #[serde(default)]
    pub z: i32,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    /// Source rectangle inside the asset. All zeros means the whole picture.
    #[serde(default)]
    pub src_x: u32,
    #[serde(default)]
    pub src_y: u32,
    #[serde(default)]
    pub src_w: u32,
    #[serde(default)]
    pub src_h: u32,
}

fn default_true() -> bool {
    true
}

/// One arrangement of parts. `ticks` is the duration used when a clip does not
/// override it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteFrame {
    pub id: String,
    #[serde(default)]
    pub parts: Vec<Part>,
    #[serde(default = "default_ticks")]
    pub ticks: u32,
}

fn default_ticks() -> u32 {
    6
}

/// A named point on the sprite canvas. Examples: feet, grip, mount, rider,
/// projectile, attack. These are data. Most of them have no gameplay yet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    pub name: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

/// A reusable picture assembly. It is not a Blueprint and it does not move.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub w: u32,
    pub h: u32,
    #[serde(default)]
    pub origin_x: i32,
    #[serde(default)]
    pub origin_y: i32,
    #[serde(default)]
    pub anchors: Vec<Anchor>,
    #[serde(default)]
    pub frames: Vec<SpriteFrame>,
    /// Author palette. Empty means the stored colors are used as they are.
    /// Baked RGBA is not turned back into cartridge palette indices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette: Vec<[u8; 3]>,
    #[serde(default = "default_schema")]
    pub schema: u32,
}

fn default_schema() -> u32 {
    SCHEMA
}

/// Ordered frames and how they play. The name is the state name.
/// `facing` is 0 for either direction, -1 for a left-drawn clip, 1 for a right-drawn clip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub frame_ids: Vec<String>,
    /// Per-frame durations in 60 Hz ticks. A missing entry uses the frame's own ticks.
    #[serde(default)]
    pub ticks: Vec<u32>,
    #[serde(default)]
    pub playback: Playback,
    #[serde(default)]
    pub facing: i32,
}

/// Clips that belong to one sprite. Missing states use `fallbacks` only.
/// A missing state never borrows an unrelated clip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimSet {
    pub id: String,
    pub name: String,
    pub sprite_id: String,
    #[serde(default)]
    pub clips: Vec<Clip>,
    /// `state` plays `clip` when that state has no clip of its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<(String, String)>,
}

/// What the renderer should draw for one actor on one tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Present {
    pub pose: String,
    pub clip_name: String,
    pub frame_index: usize,
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
    /// Mirror the composited picture. A clip already drawn facing left is not mirrored again.
    pub mirror: bool,
    /// No clip resolved. The caller should keep the Blueprint's still picture.
    pub missing: bool,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoseClock {
    pub pose: String,
    pub elapsed: u32,
}

impl Default for PoseClock {
    fn default() -> Self {
        Self {
            pose: String::new(),
            elapsed: 0,
        }
    }
}

impl PoseClock {
    /// Advance one simulation tick. A new pose starts at frame time 0.
    pub fn step(&mut self, pose: &str) {
        if self.pose != pose {
            self.pose = pose.to_string();
            self.elapsed = 0;
        } else {
            self.elapsed = self.elapsed.saturating_add(1);
        }
    }
}

/// Editor undo for sprite and binding edits. The history is not saved.
#[derive(Clone, Debug, Default)]
pub struct AnimHistory {
    undo: Vec<AnimSnap>,
    redo: Vec<AnimSnap>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AnimSnap {
    sprites: Vec<SpriteDef>,
    sets: Vec<AnimSet>,
    player_sets: Vec<(String, String)>,
    blueprint_sets: Vec<(String, String)>,
}

impl AnimHistory {
    pub fn capture(&mut self, project: &Project) {
        self.undo.push(snap(project));
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self, project: &mut Project) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(snap(project));
        restore(project, previous);
        true
    }

    pub fn redo(&mut self, project: &mut Project) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(snap(project));
        restore(project, next);
        true
    }
}

fn snap(project: &Project) -> AnimSnap {
    AnimSnap {
        sprites: project.sprites.clone(),
        sets: project.anim_sets.clone(),
        player_sets: project
            .player_blueprints
            .iter()
            .map(|blueprint| (blueprint.id.clone(), blueprint.anim_set_id.clone()))
            .collect(),
        blueprint_sets: project
            .blueprints
            .iter()
            .map(|blueprint| (blueprint.id.clone(), blueprint.anim_set_id.clone()))
            .collect(),
    }
}

fn restore(project: &mut Project, snap: AnimSnap) {
    project.sprites = snap.sprites;
    project.anim_sets = snap.sets;
    for (id, set_id) in snap.player_sets {
        if let Some(blueprint) = project
            .player_blueprints
            .iter_mut()
            .find(|blueprint| blueprint.id == id)
        {
            blueprint.anim_set_id = set_id;
        }
    }
    for (id, set_id) in snap.blueprint_sets {
        if let Some(blueprint) = project
            .blueprints
            .iter_mut()
            .find(|blueprint| blueprint.id == id)
        {
            blueprint.anim_set_id = set_id;
        }
    }
}

pub fn check_canvas(w: u32, h: u32) -> Result<(), String> {
    if w == 0 || h == 0 || w > MAX_EDGE || h > MAX_EDGE {
        return Err(format!(
            "A sprite must be between 1 and {MAX_EDGE} pixels on each side. {w}×{h} was refused."
        ));
    }
    let bytes = (w as usize)
        .checked_mul(h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "That sprite size overflows.".to_string())?;
    if bytes > (MAX_EDGE as usize) * (MAX_EDGE as usize) * 4 {
        return Err("That sprite is larger than the editor allows.".into());
    }
    Ok(())
}

pub fn create_sprite(project: &mut Project, name: &str, w: u32, h: u32) -> Result<String, String> {
    check_canvas(w, h)?;
    if project.sprites.len() >= MAX_SPRITES {
        return Err(format!("This project already has {MAX_SPRITES} sprites."));
    }
    let id = nid("sprite-def-");
    let frame_id = nid("frame-");
    let name = {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            "Sprite".to_string()
        } else {
            trimmed.to_string()
        }
    };
    project.sprites.push(SpriteDef {
        id: id.clone(),
        name,
        tags: Vec::new(),
        w,
        h,
        origin_x: w as i32 / 2,
        origin_y: h as i32,
        anchors: vec![Anchor {
            name: "feet".into(),
            x: w as i32 / 2,
            y: h as i32,
        }],
        frames: vec![SpriteFrame {
            id: frame_id,
            parts: Vec::new(),
            ticks: default_ticks(),
        }],
        palette: Vec::new(),
        schema: SCHEMA,
    });
    Ok(id)
}

pub fn add_part(
    project: &mut Project,
    sprite_id: &str,
    frame_id: &str,
    asset_id: &str,
    x: i32,
    y: i32,
) -> Result<String, String> {
    if project.asset(asset_id).is_none() {
        return Err(format!(
            "Picture {asset_id} is not in the project. The frame was not changed."
        ));
    }
    let sprite = sprite_mut(project, sprite_id)?;
    let frame = frame_mut(sprite, frame_id)?;
    if frame.parts.len() >= MAX_PARTS {
        return Err(format!("A frame can hold {MAX_PARTS} pieces."));
    }
    let id = nid("part-");
    let z = frame.parts.len() as i32;
    frame.parts.push(Part {
        id: id.clone(),
        asset_id: asset_id.into(),
        x,
        y,
        z,
        visible: true,
        flip_x: false,
        flip_y: false,
        src_x: 0,
        src_y: 0,
        src_w: 0,
        src_h: 0,
    });
    Ok(id)
}

pub fn move_part(
    project: &mut Project,
    sprite_id: &str,
    frame_id: &str,
    part_id: &str,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let sprite = sprite_mut(project, sprite_id)?;
    let frame = frame_mut(sprite, frame_id)?;
    let part = part_mut(frame, part_id)?;
    part.x = x;
    part.y = y;
    Ok(())
}

pub fn set_part_layer(
    project: &mut Project,
    sprite_id: &str,
    frame_id: &str,
    part_id: &str,
    z: i32,
) -> Result<(), String> {
    let sprite = sprite_mut(project, sprite_id)?;
    let frame = frame_mut(sprite, frame_id)?;
    let part = part_mut(frame, part_id)?;
    part.z = z;
    Ok(())
}

pub fn duplicate_part(
    project: &mut Project,
    sprite_id: &str,
    frame_id: &str,
    part_id: &str,
) -> Result<String, String> {
    let sprite = sprite_mut(project, sprite_id)?;
    let frame = frame_mut(sprite, frame_id)?;
    if frame.parts.len() >= MAX_PARTS {
        return Err(format!("A frame can hold {MAX_PARTS} pieces."));
    }
    let Some(source) = frame.parts.iter().find(|part| part.id == part_id).cloned() else {
        return Err("That piece is not on this frame.".into());
    };
    let id = nid("part-");
    frame.parts.push(Part {
        id: id.clone(),
        x: source.x + 1,
        y: source.y,
        z: source.z + 1,
        ..source
    });
    Ok(id)
}

/// A new frame copies placement only. Asset pixels are not copied.
pub fn duplicate_frame(
    project: &mut Project,
    sprite_id: &str,
    frame_id: &str,
) -> Result<String, String> {
    let sprite = sprite_mut(project, sprite_id)?;
    if sprite.frames.len() >= MAX_FRAMES {
        return Err(format!("A sprite can hold {MAX_FRAMES} frames."));
    }
    let Some(source) = sprite
        .frames
        .iter()
        .find(|frame| frame.id == frame_id)
        .cloned()
    else {
        return Err("That frame is not on this sprite.".into());
    };
    let id = nid("frame-");
    let parts = source
        .parts
        .into_iter()
        .map(|part| Part {
            id: nid("part-"),
            ..part
        })
        .collect();
    sprite.frames.push(SpriteFrame {
        id: id.clone(),
        parts,
        ticks: source.ticks,
    });
    Ok(id)
}

pub fn delete_frame(project: &mut Project, sprite_id: &str, frame_id: &str) -> Result<(), String> {
    let sprite = sprite_mut(project, sprite_id)?;
    let before = sprite.frames.len();
    sprite.frames.retain(|frame| frame.id != frame_id);
    if sprite.frames.len() == before {
        return Err("That frame is not on this sprite.".into());
    }
    let gone = frame_id.to_string();
    for set in &mut project.anim_sets {
        if set.sprite_id != sprite_id {
            continue;
        }
        for clip in &mut set.clips {
            let mut kept_ticks = Vec::new();
            let mut kept_ids = Vec::new();
            for (index, id) in clip.frame_ids.iter().enumerate() {
                if id == &gone {
                    continue;
                }
                kept_ids.push(id.clone());
                kept_ticks.push(clip.ticks.get(index).copied().unwrap_or(default_ticks()));
            }
            clip.frame_ids = kept_ids;
            clip.ticks = kept_ticks;
        }
    }
    Ok(())
}

pub fn create_set(project: &mut Project, name: &str, sprite_id: &str) -> Result<String, String> {
    if project.sprites.iter().all(|sprite| sprite.id != sprite_id) {
        return Err("Choose a sprite before making an animation.".into());
    }
    if project.anim_sets.len() >= MAX_SETS {
        return Err(format!(
            "This project already has {MAX_SETS} animation sets."
        ));
    }
    let id = nid("anim-");
    let name = {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            "Animation".to_string()
        } else {
            trimmed.to_string()
        }
    };
    project.anim_sets.push(AnimSet {
        id: id.clone(),
        name,
        sprite_id: sprite_id.into(),
        clips: Vec::new(),
        fallbacks: Vec::new(),
    });
    Ok(id)
}

pub fn add_clip(project: &mut Project, set_id: &str, name: &str) -> Result<String, String> {
    let set = set_mut(project, set_id)?;
    if set.clips.len() >= MAX_CLIPS {
        return Err(format!("An animation can hold {MAX_CLIPS} states."));
    }
    let id = nid("clip-");
    let name = {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            "idle".to_string()
        } else {
            trimmed.to_string()
        }
    };
    set.clips.push(Clip {
        id: id.clone(),
        name,
        frame_ids: Vec::new(),
        ticks: Vec::new(),
        playback: Playback::Loop,
        facing: 0,
    });
    Ok(id)
}

pub fn append_frame_to_clip(
    project: &mut Project,
    set_id: &str,
    clip_id: &str,
    frame_id: &str,
    ticks: u32,
) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    let sprite_id = set.sprite_id.clone();
    if project
        .sprites
        .iter()
        .find(|sprite| sprite.id == sprite_id)
        .and_then(|sprite| sprite.frames.iter().find(|frame| frame.id == frame_id))
        .is_none()
    {
        return Err("That frame is not on this sprite. The clip was not changed.".into());
    }
    let set = set_mut(project, set_id)?;
    let clip = clip_mut(set, clip_id)?;
    if clip.frame_ids.len() >= MAX_FRAMES {
        return Err(format!("A clip can hold {MAX_FRAMES} frames."));
    }
    clip.frame_ids.push(frame_id.into());
    clip.ticks.push(ticks.max(1));
    Ok(())
}

pub fn set_clip_ticks(
    project: &mut Project,
    set_id: &str,
    clip_id: &str,
    index: usize,
    ticks: u32,
) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    let clip = clip_mut(set, clip_id)?;
    let Some(slot) = clip.ticks.get_mut(index) else {
        return Err("That frame is not on this clip.".into());
    };
    *slot = ticks.max(1);
    Ok(())
}

pub fn reorder_clip(
    project: &mut Project,
    set_id: &str,
    clip_id: &str,
    from: usize,
    to: usize,
) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    let clip = clip_mut(set, clip_id)?;
    if from >= clip.frame_ids.len() || to >= clip.frame_ids.len() {
        return Err("That frame is not on this clip.".into());
    }
    let id = clip.frame_ids.remove(from);
    clip.frame_ids.insert(to, id);
    if clip.ticks.len() == clip.frame_ids.len() {
        let ticks = clip.ticks.remove(from);
        clip.ticks.insert(to, ticks);
    }
    Ok(())
}

pub fn delete_clip_frame(
    project: &mut Project,
    set_id: &str,
    clip_id: &str,
    index: usize,
) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    let clip = clip_mut(set, clip_id)?;
    if index >= clip.frame_ids.len() {
        return Err("That frame is not on this clip.".into());
    }
    clip.frame_ids.remove(index);
    if index < clip.ticks.len() {
        clip.ticks.remove(index);
    }
    Ok(())
}

pub fn set_playback(
    project: &mut Project,
    set_id: &str,
    clip_id: &str,
    playback: Playback,
) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    let clip = clip_mut(set, clip_id)?;
    clip.playback = playback;
    Ok(())
}

pub fn set_clip_facing(
    project: &mut Project,
    set_id: &str,
    clip_id: &str,
    facing: i32,
) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    let clip = clip_mut(set, clip_id)?;
    clip.facing = match facing {
        -1 => -1,
        1 => 1,
        _ => 0,
    };
    Ok(())
}

pub fn rename_clip(
    project: &mut Project,
    set_id: &str,
    clip_id: &str,
    name: &str,
) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    let clip = clip_mut(set, clip_id)?;
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("An animation needs a name.".into());
    }
    clip.name = trimmed.to_string();
    Ok(())
}

/// Explicit chain. Run uses Walk only when the author adds that fallback.
pub fn standard_fallbacks() -> Vec<(String, String)> {
    vec![
        ("run".into(), "walk".into()),
        ("walk".into(), "idle".into()),
        ("jump".into(), "idle".into()),
        ("fall".into(), "jump".into()),
        ("skid".into(), "walk".into()),
        ("hurt".into(), "idle".into()),
        ("defeat".into(), "walk".into()),
    ]
}

pub fn apply_standard_fallbacks(project: &mut Project, set_id: &str) -> Result<(), String> {
    let set = set_mut(project, set_id)?;
    set.fallbacks = standard_fallbacks();
    Ok(())
}

pub fn bind_player(project: &mut Project, player_id: &str, set_id: &str) -> Result<(), String> {
    if !set_id.is_empty() && project.anim_sets.iter().all(|set| set.id != set_id) {
        return Err("That animation is not in the project. The player was not changed.".into());
    }
    let Some(blueprint) = project
        .player_blueprints
        .iter_mut()
        .find(|blueprint| blueprint.id == player_id)
    else {
        return Err("That Player Blueprint is not in the project.".into());
    };
    blueprint.anim_set_id = set_id.to_string();
    Ok(())
}

pub fn bind_blueprint(
    project: &mut Project,
    blueprint_id: &str,
    set_id: &str,
) -> Result<(), String> {
    if !set_id.is_empty() && project.anim_sets.iter().all(|set| set.id != set_id) {
        return Err("That animation is not in the project. The blueprint was not changed.".into());
    }
    let Some(blueprint) = project
        .blueprints
        .iter_mut()
        .find(|blueprint| blueprint.id == blueprint_id)
    else {
        return Err("That Blueprint is not in the project.".into());
    };
    blueprint.anim_set_id = set_id.to_string();
    Ok(())
}

pub fn grid_parts(asset_ids: &[String], columns: u32, tile: u32) -> Vec<(String, i32, i32)> {
    let columns = columns.max(1);
    asset_ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let col = index as u32 % columns;
            let row = index as u32 / columns;
            (id.clone(), (col * tile) as i32, (row * tile) as i32)
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slice {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Even grid. The caller can edit the rectangles afterward.
pub fn slice_grid(
    image_w: u32,
    image_h: u32,
    frame_w: u32,
    frame_h: u32,
    margin: u32,
    spacing: u32,
) -> Result<Vec<Slice>, String> {
    if frame_w == 0 || frame_h == 0 || frame_w > MAX_EDGE || frame_h > MAX_EDGE {
        return Err("Frame size must be between 1 and 256.".into());
    }
    if image_w == 0 || image_h == 0 {
        return Err("That picture has no size.".into());
    }
    let mut slices = Vec::new();
    let mut y = margin;
    while y + frame_h <= image_h {
        let mut x = margin;
        while x + frame_w <= image_w {
            slices.push(Slice {
                x,
                y,
                w: frame_w,
                h: frame_h,
            });
            if slices.len() > MAX_FRAMES {
                return Err(format!("That sheet has more than {MAX_FRAMES} frames."));
            }
            x = x.saturating_add(frame_w).saturating_add(spacing);
        }
        y = y.saturating_add(frame_h).saturating_add(spacing);
    }
    if slices.is_empty() {
        return Err("Those frame settings do not fit on the picture.".into());
    }
    Ok(slices)
}

pub fn player_pose(on_ground: bool, vx: f32, vy: f32, hurt: bool) -> &'static str {
    if hurt {
        return "hurt";
    }
    if !on_ground {
        if vy < 0.0 {
            "jump"
        } else {
            "fall"
        }
    } else if vx.abs() > 8.0 {
        "walk"
    } else {
        "idle"
    }
}

pub fn enemy_pose(alive: bool) -> &'static str {
    if alive {
        "walk"
    } else {
        "defeat"
    }
}

pub fn anchor_of(sprite: &SpriteDef, name: &str) -> (i32, i32) {
    if let Some(anchor) = sprite
        .anchors
        .iter()
        .find(|anchor| anchor.name.eq_ignore_ascii_case(name))
    {
        return (anchor.x, anchor.y);
    }
    match name {
        "origin" => (sprite.origin_x, sprite.origin_y),
        "feet" => (sprite.w as i32 / 2, sprite.h as i32),
        _ => (sprite.origin_x, sprite.origin_y),
    }
}

pub fn missing_states(set: &AnimSet, wanted: &[&str]) -> Vec<String> {
    wanted
        .iter()
        .filter(|state| resolve_clip(set, state, 1).is_none())
        .map(|state| (*state).to_string())
        .collect()
}

pub fn resolve_clip<'a>(set: &'a AnimSet, pose: &str, facing: i32) -> Option<&'a Clip> {
    let mut seen = Vec::new();
    let mut name = pose.to_string();
    for _ in 0..8 {
        if seen.iter().any(|previous: &String| previous == &name) {
            return None;
        }
        if let Some(clip) = clip_for_name(set, &name, facing) {
            return Some(clip);
        }
        seen.push(name.clone());
        let Some((_, next)) = set
            .fallbacks
            .iter()
            .find(|(state, _)| state.eq_ignore_ascii_case(&name))
        else {
            return None;
        };
        name = next.clone();
    }
    None
}

fn clip_for_name<'a>(set: &'a AnimSet, name: &str, facing: i32) -> Option<&'a Clip> {
    let facing = if facing < 0 { -1 } else { 1 };
    set.clips
        .iter()
        .find(|clip| clip.name.eq_ignore_ascii_case(name) && clip.facing == facing)
        .or_else(|| {
            set.clips
                .iter()
                .find(|clip| clip.name.eq_ignore_ascii_case(name) && clip.facing == 0)
        })
}

pub fn sample(
    _set: &AnimSet,
    sprite: &SpriteDef,
    clip: &Clip,
    elapsed: u32,
) -> Option<(usize, String)> {
    if clip.frame_ids.is_empty() {
        return None;
    }
    let durations: Vec<u32> = clip
        .frame_ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            clip.ticks
                .get(index)
                .copied()
                .filter(|ticks| *ticks > 0)
                .or_else(|| {
                    sprite
                        .frames
                        .iter()
                        .find(|frame| &frame.id == id)
                        .map(|frame| frame.ticks.max(1))
                })
                .unwrap_or(default_ticks())
                .max(1)
        })
        .collect();
    let index = match clip.playback {
        Playback::Loop => walk_loop(&durations, elapsed),
        Playback::Once | Playback::Hold => walk_hold(&durations, elapsed),
        Playback::PingPong => walk_ping(&durations, elapsed),
    };
    let id = clip.frame_ids.get(index)?.clone();
    Some((index, id))
}

pub fn clip_finished(clip: &Clip, sprite: &SpriteDef, elapsed: u32) -> bool {
    if clip.playback != Playback::Once {
        return false;
    }
    let total: u32 = clip
        .frame_ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            clip.ticks
                .get(index)
                .copied()
                .filter(|ticks| *ticks > 0)
                .or_else(|| {
                    sprite
                        .frames
                        .iter()
                        .find(|frame| &frame.id == id)
                        .map(|frame| frame.ticks.max(1))
                })
                .unwrap_or(1)
        })
        .sum();
    elapsed >= total
}

fn walk_loop(durations: &[u32], elapsed: u32) -> usize {
    let total: u32 = durations.iter().sum();
    if total == 0 {
        return 0;
    }
    let mut left = elapsed % total;
    for (index, duration) in durations.iter().enumerate() {
        if left < *duration {
            return index;
        }
        left -= *duration;
    }
    0
}

fn walk_hold(durations: &[u32], elapsed: u32) -> usize {
    let mut left = elapsed;
    for (index, duration) in durations.iter().enumerate() {
        if index + 1 == durations.len() || left < *duration {
            return index;
        }
        left -= *duration;
    }
    0
}

fn walk_ping(durations: &[u32], elapsed: u32) -> usize {
    let n = durations.len();
    if n <= 1 {
        return 0;
    }
    let mut order = Vec::with_capacity(n * 2);
    for index in 0..n {
        order.push(index);
    }
    for index in (1..n - 1).rev() {
        order.push(index);
    }
    let cycle: Vec<u32> = order.iter().map(|index| durations[*index]).collect();
    let at = walk_loop(&cycle, elapsed);
    order[at]
}

#[derive(Clone, Debug)]
pub struct ImageBuf {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

pub fn asset_image(asset: &Asset) -> Result<ImageBuf, String> {
    if asset.w == 0 || asset.h == 0 {
        return Err(format!("{} has no size.", asset.name));
    }
    if !asset.rgba_base64.trim().is_empty() {
        let rgba = b64_decode(&asset.rgba_base64)?;
        let need = asset.w as usize * asset.h as usize * 4;
        if rgba.len() < need {
            return Err(format!("{} is shorter than its size.", asset.name));
        }
        return Ok(ImageBuf {
            w: asset.w,
            h: asset.h,
            rgba,
        });
    }
    if !asset.png_base64.trim().is_empty() {
        let bytes = b64_decode(&asset.png_base64)?;
        let (w, h, rgba) = crate::pngutil::png_to_rgba(&bytes)?;
        return Ok(ImageBuf { w, h, rgba });
    }
    Err(format!("{} has no picture.", asset.name))
}

pub fn composite(
    sprite: &SpriteDef,
    frame: &SpriteFrame,
    project: &Project,
) -> Result<ImageBuf, String> {
    check_canvas(sprite.w, sprite.h)?;
    let mut rgba = vec![0u8; sprite.w as usize * sprite.h as usize * 4];
    let mut missing = Vec::new();
    let mut parts = frame.parts.clone();
    parts.sort_by(|a, b| a.z.cmp(&b.z).then(a.id.cmp(&b.id)));
    for part in parts {
        if !part.visible {
            continue;
        }
        let Some(asset) = project.asset(&part.asset_id) else {
            missing.push(part.asset_id);
            continue;
        };
        let image = match asset_image(asset) {
            Ok(image) => image,
            Err(_) => {
                missing.push(part.asset_id);
                continue;
            }
        };
        blit_part(&mut rgba, sprite.w, sprite.h, &part, &image);
    }
    if !missing.is_empty() {
        return Err(format!(
            "Missing picture {}. The other pieces were kept.",
            missing.join(", ")
        ));
    }
    Ok(ImageBuf {
        w: sprite.w,
        h: sprite.h,
        rgba,
    })
}

fn blit_part(dest: &mut [u8], dw: u32, dh: u32, part: &Part, image: &ImageBuf) {
    let (sx, sy, sw, sh) = if part.src_w == 0 || part.src_h == 0 {
        (0, 0, image.w, image.h)
    } else {
        (part.src_x, part.src_y, part.src_w, part.src_h)
    };
    for row in 0..sh {
        for col in 0..sw {
            let src_x = if part.flip_x { sw - 1 - col } else { col };
            let src_y = if part.flip_y { sh - 1 - row } else { row };
            let ix = sx + src_x;
            let iy = sy + src_y;
            if ix >= image.w || iy >= image.h {
                continue;
            }
            let dx = part.x + col as i32;
            let dy = part.y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dw || dy as u32 >= dh {
                continue;
            }
            let so = ((iy * image.w + ix) * 4) as usize;
            let d_o = ((dy as u32 * dw + dx as u32) * 4) as usize;
            if so + 4 > image.rgba.len() || d_o + 4 > dest.len() {
                continue;
            }
            over(
                &mut dest[d_o..d_o + 4],
                [
                    image.rgba[so],
                    image.rgba[so + 1],
                    image.rgba[so + 2],
                    image.rgba[so + 3],
                ],
            );
        }
    }
}

fn over(dst: &mut [u8], src: [u8; 4]) {
    if src[3] == 0 {
        return;
    }
    if src[3] == 255 || dst[3] == 0 {
        dst.copy_from_slice(&src);
        return;
    }
    let sa = src[3] as u32;
    let da = dst[3] as u32;
    let out_a = sa + da * (255 - sa) / 255;
    if out_a == 0 {
        return;
    }
    for channel in 0..3 {
        let mixed = src[channel] as u32 * sa + dst[channel] as u32 * da * (255 - sa) / 255;
        dst[channel] = (mixed / out_a) as u8;
    }
    dst[3] = out_a as u8;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FramePick {
    pub frame_id: String,
    pub mirror: bool,
    pub note: String,
}

/// Which saved frame to draw. This does not allocate a picture.
pub fn select_frame(
    project: &Project,
    set_id: &str,
    pose: &str,
    facing: i32,
    elapsed: u32,
) -> Result<FramePick, String> {
    let facing = if facing < 0 { -1 } else { 1 };
    let set = project
        .anim_sets
        .iter()
        .find(|set| set.id == set_id)
        .ok_or_else(|| "No animation is assigned. The still picture is used.".to_string())?;
    let sprite = project
        .sprites
        .iter()
        .find(|sprite| sprite.id == set.sprite_id)
        .ok_or_else(|| format!("Animation {} points at a missing sprite.", set.name))?;
    let clip = resolve_clip(set, pose, facing)
        .ok_or_else(|| format!("No {pose} animation. The still picture is used."))?;
    let (_, frame_id) = sample(set, sprite, clip, elapsed)
        .ok_or_else(|| format!("{} has no frames.", clip.name))?;
    if sprite.frames.iter().all(|frame| frame.id != frame_id) {
        return Err(format!("Frame {frame_id} is missing. It was not replaced."));
    }
    Ok(FramePick {
        frame_id,
        mirror: facing < 0 && clip.facing == 0,
        note: String::new(),
    })
}

pub fn present(project: &Project, set_id: &str, pose: &str, facing: i32, elapsed: u32) -> Present {
    let facing = if facing < 0 { -1 } else { 1 };
    let Some(set) = project.anim_sets.iter().find(|set| set.id == set_id) else {
        return Present {
            pose: pose.into(),
            clip_name: String::new(),
            frame_index: 0,
            rgba: Vec::new(),
            w: 0,
            h: 0,
            mirror: false,
            missing: true,
            note: "No animation is assigned. The still picture is used.".into(),
        };
    };
    let Some(sprite) = project
        .sprites
        .iter()
        .find(|sprite| sprite.id == set.sprite_id)
    else {
        return Present {
            pose: pose.into(),
            clip_name: String::new(),
            frame_index: 0,
            rgba: Vec::new(),
            w: 0,
            h: 0,
            mirror: false,
            missing: true,
            note: format!(
                "Animation {} points at a sprite that is not in the project.",
                set.name
            ),
        };
    };
    let Some(clip) = resolve_clip(set, pose, facing) else {
        return Present {
            pose: pose.into(),
            clip_name: String::new(),
            frame_index: 0,
            rgba: Vec::new(),
            w: sprite.w,
            h: sprite.h,
            mirror: false,
            missing: true,
            note: format!("No {pose} animation. The still picture is used."),
        };
    };
    let Some((index, frame_id)) = sample(set, sprite, clip, elapsed) else {
        return Present {
            pose: pose.into(),
            clip_name: clip.name.clone(),
            frame_index: 0,
            rgba: Vec::new(),
            w: sprite.w,
            h: sprite.h,
            mirror: false,
            missing: true,
            note: format!("{} has no frames.", clip.name),
        };
    };
    let Some(frame) = sprite.frames.iter().find(|frame| frame.id == frame_id) else {
        return Present {
            pose: pose.into(),
            clip_name: clip.name.clone(),
            frame_index: index,
            rgba: Vec::new(),
            w: sprite.w,
            h: sprite.h,
            mirror: false,
            missing: true,
            note: format!("Frame {frame_id} is missing. It was not replaced."),
        };
    };
    match composite(sprite, frame, project) {
        Ok(image) => Present {
            pose: pose.into(),
            clip_name: clip.name.clone(),
            frame_index: index,
            rgba: image.rgba,
            w: image.w,
            h: image.h,
            mirror: facing < 0 && clip.facing == 0,
            missing: false,
            note: String::new(),
        },
        Err(note) => Present {
            pose: pose.into(),
            clip_name: clip.name.clone(),
            frame_index: index,
            rgba: Vec::new(),
            w: sprite.w,
            h: sprite.h,
            mirror: false,
            missing: true,
            note,
        },
    }
}

pub fn set_for_player<'a>(project: &'a Project, level_id: &str) -> Option<&'a AnimSet> {
    let level = project.levels.iter().find(|level| level.id == level_id)?;
    let player_id = level.player_blueprint_id.as_deref()?;
    let set_id = project
        .player_blueprints
        .iter()
        .find(|blueprint| blueprint.id == player_id)?
        .anim_set_id
        .as_str();
    if set_id.is_empty() {
        return None;
    }
    project.anim_sets.iter().find(|set| set.id == set_id)
}

pub fn set_for_blueprint<'a>(project: &'a Project, blueprint_id: &str) -> Option<&'a AnimSet> {
    let set_id = project
        .blueprints
        .iter()
        .find(|blueprint| blueprint.id == blueprint_id)?
        .anim_set_id
        .as_str();
    if set_id.is_empty() {
        return None;
    }
    project.anim_sets.iter().find(|set| set.id == set_id)
}

fn sprite_mut<'a>(project: &'a mut Project, id: &str) -> Result<&'a mut SpriteDef, String> {
    project
        .sprites
        .iter_mut()
        .find(|sprite| sprite.id == id)
        .ok_or_else(|| "That sprite is not in the project.".into())
}

fn frame_mut<'a>(sprite: &'a mut SpriteDef, id: &str) -> Result<&'a mut SpriteFrame, String> {
    sprite
        .frames
        .iter_mut()
        .find(|frame| frame.id == id)
        .ok_or_else(|| "That frame is not on this sprite.".into())
}

fn part_mut<'a>(frame: &'a mut SpriteFrame, id: &str) -> Result<&'a mut Part, String> {
    frame
        .parts
        .iter_mut()
        .find(|part| part.id == id)
        .ok_or_else(|| "That piece is not on this frame.".into())
}

fn set_mut<'a>(project: &'a mut Project, id: &str) -> Result<&'a mut AnimSet, String> {
    project
        .anim_sets
        .iter_mut()
        .find(|set| set.id == id)
        .ok_or_else(|| "That animation is not in the project.".into())
}

fn clip_mut<'a>(set: &'a mut AnimSet, id: &str) -> Result<&'a mut Clip, String> {
    set.clips
        .iter_mut()
        .find(|clip| clip.id == id)
        .ok_or_else(|| "That clip is not on this animation.".into())
}

pub fn write_project(path: &std::path::Path, project: &Project) -> Result<(), String> {
    let text = serde_json::to_string_pretty(project).map_err(|err| err.to_string())?;
    std::fs::write(path, text).map_err(|err| err.to_string())
}

pub fn solid_asset(id: &str, name: &str, kind: AssetKind, rgb: [u8; 3], w: u32, h: u32) -> Asset {
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    for pixel in rgba.chunks_mut(4) {
        pixel[0] = rgb[0];
        pixel[1] = rgb[1];
        pixel[2] = rgb[2];
        pixel[3] = 255;
    }
    Asset {
        id: id.into(),
        name: name.into(),
        kind,
        style_id: String::new(),
        w,
        h,
        rgba_base64: b64_encode(&rgba),
        png_base64: String::new(),
        wav_base64: String::new(),
        solid: kind == AssetKind::Terrain,
        cue: String::new(),
        item_tag: crate::project::ItemTag::Auto,
        block_tag: crate::project::BlockTag::Auto,
        hazard_tag: crate::project::HazardTag::Auto,
        warp: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint::ensure_legacy_blueprints;
    use crate::enemy_ai::EnemyAiStyle;
    use crate::physics::{CharacterPhysics, Input, Sim, TILE};
    use crate::player::new_player_blueprint;
    use crate::project::{Cell, Layer, Level, PowerUpStyle, Spawn};

    fn paint(
        project: &mut Project,
        sprite: &str,
        frame: &str,
        asset: &str,
        x: i32,
        y: i32,
    ) -> String {
        add_part(project, sprite, frame, asset, x, y).unwrap()
    }

    fn one_sprite() -> (Project, String, String) {
        let mut project = Project::blank("anim");
        project.assets.push(solid_asset(
            "red",
            "Red",
            AssetKind::Player,
            [200, 40, 40],
            8,
            8,
        ));
        project.assets.push(solid_asset(
            "blue",
            "Blue",
            AssetKind::Player,
            [40, 80, 200],
            8,
            8,
        ));
        let sprite = create_sprite(&mut project, "Explorer", 16, 16).unwrap();
        let frame = project.sprites[0].frames[0].id.clone();
        (project, sprite, frame)
    }

    #[test]
    fn four_tiles_assemble_into_a_sixteen_sprite() {
        let (mut project, sprite, frame) = one_sprite();
        let placed = grid_parts(
            &["red".into(), "blue".into(), "blue".into(), "red".into()],
            2,
            8,
        );
        assert_eq!(placed[1].1, 8);
        assert_eq!(placed[2].2, 8);
        for (id, x, y) in &placed {
            paint(&mut project, &sprite, &frame, id, *x, *y);
        }
        let image =
            composite(&project.sprites[0], &project.sprites[0].frames[0], &project).unwrap();
        assert_eq!((image.w, image.h), (16, 16));
        assert_eq!(&image.rgba[0..3], &[200, 40, 40]);
        let top_right = (8 * 4) as usize;
        assert_eq!(&image.rgba[top_right..top_right + 3], &[40, 80, 200]);
    }

    #[test]
    fn moving_one_frame_does_not_move_the_other() {
        let (mut project, sprite, frame) = one_sprite();
        let part = paint(&mut project, &sprite, &frame, "red", 0, 0);
        let copy = duplicate_frame(&mut project, &sprite, &frame).unwrap();
        let moved = project.sprites[0].frames[1].parts[0].id.clone();
        move_part(&mut project, &sprite, &copy, &moved, 4, 2).unwrap();
        let original = project.sprites[0].frames[0]
            .parts
            .iter()
            .find(|item| item.id == part)
            .unwrap();
        assert_eq!((original.x, original.y), (0, 0));
        assert_eq!(
            (
                project.sprites[0].frames[1].parts[0].x,
                project.sprites[0].frames[1].parts[0].y
            ),
            (4, 2)
        );
        assert_eq!(
            project.sprites[0].frames[0].parts[0].asset_id,
            project.sprites[0].frames[1].parts[0].asset_id
        );
    }

    #[test]
    fn clip_order_duration_and_playback_modes() {
        let (mut project, sprite, frame_a) = one_sprite();
        let frame_b = duplicate_frame(&mut project, &sprite, &frame_a).unwrap();
        let set = create_set(&mut project, "Explorer", &sprite).unwrap();
        let clip = add_clip(&mut project, &set, "walk").unwrap();
        append_frame_to_clip(&mut project, &set, &clip, &frame_a, 4).unwrap();
        append_frame_to_clip(&mut project, &set, &clip, &frame_b, 4).unwrap();
        {
            let set_ref = project
                .anim_sets
                .iter()
                .find(|item| item.id == set)
                .unwrap();
            let clip_ref = &set_ref.clips[0];
            let sprite_ref = &project.sprites[0];
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 0).unwrap().0, 0);
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 3).unwrap().0, 0);
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 4).unwrap().0, 1);
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 8).unwrap().0, 0);
        }
        set_playback(&mut project, &set, &clip, Playback::Once).unwrap();
        {
            let set_ref = project
                .anim_sets
                .iter()
                .find(|item| item.id == set)
                .unwrap();
            let clip_ref = &set_ref.clips[0];
            let sprite_ref = &project.sprites[0];
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 20).unwrap().0, 1);
            assert!(clip_finished(clip_ref, sprite_ref, 8));
        }
        set_playback(&mut project, &set, &clip, Playback::Hold).unwrap();
        {
            let set_ref = project
                .anim_sets
                .iter()
                .find(|item| item.id == set)
                .unwrap();
            let clip_ref = &set_ref.clips[0];
            let sprite_ref = &project.sprites[0];
            assert!(!clip_finished(clip_ref, sprite_ref, 30));
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 30).unwrap().0, 1);
        }
        set_playback(&mut project, &set, &clip, Playback::PingPong).unwrap();
        {
            let set_ref = project
                .anim_sets
                .iter()
                .find(|item| item.id == set)
                .unwrap();
            let clip_ref = &set_ref.clips[0];
            let sprite_ref = &project.sprites[0];
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 0).unwrap().0, 0);
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 4).unwrap().0, 1);
            assert_eq!(sample(set_ref, sprite_ref, clip_ref, 8).unwrap().0, 0);
        }
    }

    #[test]
    fn fallback_is_explicit_and_unrelated_art_is_not_used() {
        let (mut project, sprite, frame) = one_sprite();
        let set = create_set(&mut project, "Explorer", &sprite).unwrap();
        let idle = add_clip(&mut project, &set, "idle").unwrap();
        append_frame_to_clip(&mut project, &set, &idle, &frame, 6).unwrap();
        let set_ref = project
            .anim_sets
            .iter()
            .find(|item| item.id == set)
            .unwrap();
        assert!(resolve_clip(set_ref, "walk", 1).is_none());
        assert_eq!(
            missing_states(set_ref, &["idle", "walk"]),
            vec!["walk".to_string()]
        );
        apply_standard_fallbacks(&mut project, &set).unwrap();
        let set_ref = project
            .anim_sets
            .iter()
            .find(|item| item.id == set)
            .unwrap();
        assert_eq!(resolve_clip(set_ref, "walk", 1).unwrap().name, "idle");
        assert!(resolve_clip(set_ref, "attack", 1).is_none());
    }

    #[test]
    fn direction_prefers_a_matching_clip() {
        let (mut project, sprite, frame) = one_sprite();
        let set = create_set(&mut project, "Explorer", &sprite).unwrap();
        let either = add_clip(&mut project, &set, "walk").unwrap();
        append_frame_to_clip(&mut project, &set, &either, &frame, 6).unwrap();
        let left = add_clip(&mut project, &set, "walk").unwrap();
        append_frame_to_clip(&mut project, &set, &left, &frame, 6).unwrap();
        set_clip_facing(&mut project, &set, &left, -1).unwrap();
        let set_ref = project
            .anim_sets
            .iter()
            .find(|item| item.id == set)
            .unwrap();
        assert_eq!(resolve_clip(set_ref, "walk", -1).unwrap().id, left);
        assert_eq!(resolve_clip(set_ref, "walk", 1).unwrap().id, either);
        let shown = present(&project, &set, "walk", -1, 0);
        assert!(!shown.mirror);
        let shown = present(&project, &set, "walk", 1, 0);
        assert!(!shown.mirror);
    }

    #[test]
    fn transparency_and_origin_survive_compositing() {
        let mut project = Project::blank("clear");
        let mut rgba = vec![0u8; 8 * 8 * 4];
        rgba[0] = 10;
        rgba[1] = 20;
        rgba[2] = 30;
        rgba[3] = 255;
        project.assets.push(Asset {
            id: "speck".into(),
            name: "Speck".into(),
            kind: AssetKind::Player,
            style_id: String::new(),
            w: 8,
            h: 8,
            rgba_base64: b64_encode(&rgba),
            png_base64: String::new(),
            wav_base64: String::new(),
            solid: false,
            cue: String::new(),
            item_tag: crate::project::ItemTag::Auto,
            block_tag: crate::project::BlockTag::Auto,
            hazard_tag: crate::project::HazardTag::Auto,
            warp: None,
        });
        let sprite = create_sprite(&mut project, "Speck", 8, 8).unwrap();
        let frame = project.sprites[0].frames[0].id.clone();
        paint(&mut project, &sprite, &frame, "speck", 0, 0);
        let image =
            composite(&project.sprites[0], &project.sprites[0].frames[0], &project).unwrap();
        assert_eq!(image.rgba[3], 255);
        assert_eq!(image.rgba[7], 0);
        assert_eq!(anchor_of(&project.sprites[0], "feet"), (4, 8));
        assert_eq!(anchor_of(&project.sprites[0], "origin"), (4, 8));
    }

    #[test]
    fn invalid_sizes_and_missing_pictures_do_not_drop_the_reference() {
        let mut project = Project::blank("limits");
        assert!(create_sprite(&mut project, "No", 0, 8).is_err());
        assert!(create_sprite(&mut project, "Huge", 400, 8).is_err());
        assert!(project.sprites.is_empty());
        let sprite = create_sprite(&mut project, "Ok", 8, 8).unwrap();
        let frame = project.sprites[0].frames[0].id.clone();
        assert!(add_part(&mut project, &sprite, &frame, "gone", 0, 0).is_err());
        project.sprites[0].frames[0].parts.push(Part {
            id: "kept".into(),
            asset_id: "gone".into(),
            x: 1,
            y: 2,
            z: 0,
            visible: true,
            flip_x: false,
            flip_y: false,
            src_x: 0,
            src_y: 0,
            src_w: 0,
            src_h: 0,
        });
        let err =
            composite(&project.sprites[0], &project.sprites[0].frames[0], &project).unwrap_err();
        assert!(err.contains("gone"));
        let text = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.sprites[0].frames[0].parts[0].asset_id, "gone");
    }

    #[test]
    fn sheet_slice_and_round_trip_keep_order_timing_and_origin() {
        let slices = slice_grid(34, 18, 8, 8, 1, 2).unwrap();
        assert_eq!(slices.len(), 3);
        assert_eq!(slices[1].x, 11);
        assert!(slice_grid(8, 8, 16, 8, 0, 0).is_err());
        let (mut project, sprite, frame) = one_sprite();
        paint(&mut project, &sprite, &frame, "red", 1, 2);
        let set = create_set(&mut project, "Explorer", &sprite).unwrap();
        let clip = add_clip(&mut project, &set, "idle").unwrap();
        append_frame_to_clip(&mut project, &set, &clip, &frame, 9).unwrap();
        let text = serde_json::to_string(&project).unwrap();
        assert!(!text.contains("egui"));
        assert!(!text.contains("TextureHandle"));
        assert!(!text.contains("NES\u{1a}"));
        let loaded: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.sprites[0].frames[0].parts[0].x, 1);
        assert_eq!(loaded.anim_sets[0].clips[0].ticks, vec![9]);
        assert_eq!(loaded.anim_sets[0].clips[0].frame_ids.len(), 1);
        assert_eq!(loaded.sprites[0].origin_x, 8);
    }

    #[test]
    fn an_old_project_opens_and_a_still_picture_stays_a_still_picture() {
        let old = r#"{"name":"Old","assets":[{"id":"pip","name":"Pip","kind":"player","w":8,"h":8,"rgba_base64":"AQIDBA=="}],"levels":[]}"#;
        let project: Project = serde_json::from_str(old).unwrap();
        assert!(project.sprites.is_empty());
        assert!(project.anim_sets.is_empty());
        assert_eq!(project.assets[0].id, "pip");
        let shown = present(&project, "", "idle", 1, 0);
        assert!(shown.missing);
    }

    #[test]
    fn undo_restores_a_move_and_a_binding() {
        let (mut project, sprite, frame) = one_sprite();
        let part = paint(&mut project, &sprite, &frame, "red", 0, 0);
        let set = create_set(&mut project, "Explorer", &sprite).unwrap();
        let mut history = AnimHistory::default();
        history.capture(&project);
        move_part(&mut project, &sprite, &frame, &part, 5, 6).unwrap();
        let player = new_player_blueprint("Explorer", "red", CharacterPhysics::Smb1Mario);
        let player_id = player.id.clone();
        project.player_blueprints.push(player);
        history.capture(&project);
        bind_player(&mut project, &player_id, &set).unwrap();
        assert!(history.undo(&mut project));
        assert!(project.player_blueprints[0].anim_set_id.is_empty());
        assert_eq!(project.sprites[0].frames[0].parts[0].x, 5);
        assert!(history.undo(&mut project));
        assert_eq!(project.sprites[0].frames[0].parts[0].x, 0);
        assert!(history.redo(&mut project));
        assert_eq!(project.sprites[0].frames[0].parts[0].x, 5);
    }

    #[test]
    fn save_failure_leaves_the_project_intact() {
        let (project, _, _) = one_sprite();
        let err = write_project(std::path::Path::new("/tmp"), &project).unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(project.sprites.len(), 1);
    }

    #[test]
    fn play_switches_player_and_enemy_pictures_without_moving_physics() {
        let bare = course_project(false);
        let dressed = course_project(true);
        let preset = CharacterPhysics::Smb1Mario;
        let step = Input {
            right: true,
            ..Input::default()
        };
        let mut bare_sim = Sim::from_level(&bare, &bare.levels[0]);
        let mut dressed_sim = Sim::from_level(&dressed, &dressed.levels[0]);
        let mut clock = PoseClock::default();
        let mut saw_idle = false;
        let mut saw_walk = false;
        for _ in 0..20 {
            bare_sim.tick_play(preset, PowerUpStyle::Smb1, &step);
            dressed_sim.tick_play(preset, PowerUpStyle::Smb1, &step);
            assert!((bare_sim.player.x - dressed_sim.player.x).abs() < 0.001);
            assert_eq!(bare_sim.player.vx, dressed_sim.player.vx);
            let pose = player_pose(
                dressed_sim.player.on_ground,
                dressed_sim.player.vx,
                dressed_sim.player.vy,
                dressed_sim.events.hurt,
            );
            clock.step(pose);
            let set = dressed.anim_sets[0].id.clone();
            let shown = present(
                &dressed,
                &set,
                pose,
                dressed_sim.player.facing,
                clock.elapsed,
            );
            assert!(!shown.missing);
            if pose == "idle" {
                saw_idle = true;
                assert_eq!(&shown.rgba[0..3], &[200, 40, 40]);
            }
            if pose == "walk" {
                saw_walk = true;
                assert_eq!(shown.clip_name, "walk");
                assert_eq!(&shown.rgba[0..3], &[40, 80, 200]);
            }
        }
        assert!(saw_idle);
        assert!(saw_walk);
        let _ = bare;
    }

    #[test]
    fn a_stomp_plays_the_defeat_frame_and_keeps_the_stomp() {
        let mut project = course_project(true);
        project.levels[0].player.x = 5.0;
        project.levels[0].player.y = 10.0;
        let sprite_id = project.sprites[0].id.clone();
        let set = create_set(&mut project, "Pebble", &sprite_id).unwrap();
        let walk = add_clip(&mut project, &set, "walk").unwrap();
        let defeat = add_clip(&mut project, &set, "defeat").unwrap();
        let frames = project.sprites[0].frames.clone();
        append_frame_to_clip(&mut project, &set, &walk, &frames[0].id, 8).unwrap();
        let defeat_frame = duplicate_frame(&mut project, &sprite_id, &frames[0].id).unwrap();
        project.sprites[0]
            .frames
            .iter_mut()
            .find(|frame| frame.id == defeat_frame)
            .unwrap()
            .parts[0]
            .asset_id = "gold".into();
        append_frame_to_clip(&mut project, &set, &defeat, &defeat_frame, 8).unwrap();
        ensure_legacy_blueprints(&mut project);
        let enemy_blueprint = project
            .blueprints
            .iter_mut()
            .find(|blueprint| blueprint.visual_id == "goomba")
            .unwrap();
        enemy_blueprint.anim_set_id = set.clone();
        let mut sim = Sim::from_level(&project, &project.levels[0]);
        let mut stomped = false;
        for _ in 0..40 {
            sim.tick_play(
                CharacterPhysics::Smb1Mario,
                PowerUpStyle::Smb1,
                &Input::default(),
            );
            if sim.events.stomp {
                stomped = true;
                break;
            }
        }
        assert!(stomped);
        assert!(!sim.enemies[0].alive);
        assert!(sim.enemies[0].defeat_ticks > 0);
        assert!((sim.player.vy - -240.0).abs() < 0.001);
        let shown = present(&project, &set, enemy_pose(false), 1, 0);
        assert_eq!(shown.clip_name, "defeat");
        assert_eq!(&shown.rgba[0..3], &[220, 180, 40]);
        let walking = present(&project, &set, enemy_pose(true), sim.enemies[0].dir, 0);
        assert_eq!(walking.clip_name, "walk");
    }

    #[test]
    fn showcase_course_keeps_player_and_enemy_bindings() {
        let project = showcase_project();
        let text = serde_json::to_string_pretty(&project).unwrap();
        assert!(!text.contains("egui"));
        assert!(!text.contains("TextureHandle"));
        assert!(!text.contains("NES\u{1a}"));
        let loaded: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.anim_sets.len(), 2);
        assert!(!loaded.player_blueprints[0].anim_set_id.is_empty());
        assert!(loaded.blueprints.iter().any(|blueprint| {
            blueprint.visual_id == "goomba" && !blueprint.anim_set_id.is_empty()
        }));
        let mut sim = Sim::from_level(&loaded, &loaded.levels[0]);
        let preset = CharacterPhysics::Smb1Mario;
        let mut clock = PoseClock::default();
        let mut enemy_clock = PoseClock::default();
        let player_set = loaded.player_blueprints[0].anim_set_id.clone();
        let enemy_set = loaded
            .blueprints
            .iter()
            .find(|blueprint| blueprint.visual_id == "goomba")
            .unwrap()
            .anim_set_id
            .clone();
        let mut saw_walk = false;
        let mut enemy_colors = Vec::new();
        for _ in 0..30 {
            sim.tick_play(
                preset,
                PowerUpStyle::Smb1,
                &Input {
                    right: true,
                    ..Input::default()
                },
            );
            let pose = player_pose(
                sim.player.on_ground,
                sim.player.vx,
                sim.player.vy,
                sim.events.hurt,
            );
            clock.step(pose);
            if pose == "walk" {
                saw_walk = true;
                let shown = present(&loaded, &player_set, pose, sim.player.facing, clock.elapsed);
                assert_eq!(shown.clip_name, "walk");
            }
            enemy_clock.step(enemy_pose(sim.enemies[0].alive));
            let enemy = present(
                &loaded,
                &enemy_set,
                enemy_pose(sim.enemies[0].alive),
                sim.enemies[0].dir,
                enemy_clock.elapsed,
            );
            enemy_colors.push(enemy.rgba[0]);
        }
        assert!(saw_walk);
        assert!(enemy_colors.contains(&120));
        assert!(enemy_colors.contains(&230));
        sim.player.vy = -40.0;
        sim.player.on_ground = false;
        let jump = present(
            &loaded,
            &player_set,
            player_pose(false, sim.player.vx, sim.player.vy, false),
            1,
            0,
        );
        assert_eq!(jump.clip_name, "jump");
        assert_eq!(&jump.rgba[0..3], &[40, 160, 70]);
        if let Ok(path) = std::env::var("NESTICLES_WRITE_FIXTURE") {
            std::fs::write(path, text).unwrap();
        }
    }

    fn showcase_project() -> Project {
        let mut project = course_project(true);
        if let Some(cell) = project.levels[0]
            .cells
            .iter_mut()
            .find(|cell| cell.asset_id == "goomba")
        {
            cell.x = 18;
        }
        let player_sprite = project.sprites[0].id.clone();
        let idle_frame = project.sprites[0].frames[0].id.clone();
        let jump_frame = duplicate_frame(&mut project, &player_sprite, &idle_frame).unwrap();
        project.assets.push(solid_asset(
            "leap",
            "Leap",
            AssetKind::Player,
            [40, 160, 70],
            16,
            16,
        ));
        project.assets.push(solid_asset(
            "stride2",
            "Stride two",
            AssetKind::Player,
            [80, 180, 220],
            16,
            16,
        ));
        project.assets.push(solid_asset(
            "step",
            "Step",
            AssetKind::Enemy,
            [230, 120, 40],
            16,
            16,
        ));
        let walk_again = duplicate_frame(&mut project, &player_sprite, &idle_frame).unwrap();
        for frame in &mut project.sprites[0].frames {
            if frame.id == jump_frame {
                frame.parts[0].asset_id = "leap".into();
            }
            if frame.id == walk_again {
                frame.parts[0].asset_id = "stride2".into();
            }
        }
        let player_set = project.anim_sets[0].id.clone();
        let walk = project.anim_sets[0]
            .clips
            .iter()
            .find(|clip| clip.name == "walk")
            .unwrap()
            .id
            .clone();
        append_frame_to_clip(&mut project, &player_set, &walk, &walk_again, 6).unwrap();
        let jump = add_clip(&mut project, &player_set, "jump").unwrap();
        append_frame_to_clip(&mut project, &player_set, &jump, &jump_frame, 8).unwrap();
        let enemy_sprite = create_sprite(&mut project, "Pebble", 16, 16).unwrap();
        let enemy_a = project
            .sprites
            .iter()
            .find(|sprite| sprite.id == enemy_sprite)
            .unwrap()
            .frames[0]
            .id
            .clone();
        paint(&mut project, &enemy_sprite, &enemy_a, "goomba", 0, 0);
        let enemy_b = duplicate_frame(&mut project, &enemy_sprite, &enemy_a).unwrap();
        project
            .sprites
            .iter_mut()
            .find(|sprite| sprite.id == enemy_sprite)
            .unwrap()
            .frames
            .iter_mut()
            .find(|frame| frame.id == enemy_b)
            .unwrap()
            .parts[0]
            .asset_id = "step".into();
        let defeat = duplicate_frame(&mut project, &enemy_sprite, &enemy_a).unwrap();
        project
            .sprites
            .iter_mut()
            .find(|sprite| sprite.id == enemy_sprite)
            .unwrap()
            .frames
            .iter_mut()
            .find(|frame| frame.id == defeat)
            .unwrap()
            .parts[0]
            .asset_id = "gold".into();
        let enemy_set = create_set(&mut project, "Pebble", &enemy_sprite).unwrap();
        let enemy_walk = add_clip(&mut project, &enemy_set, "walk").unwrap();
        let enemy_defeat = add_clip(&mut project, &enemy_set, "defeat").unwrap();
        append_frame_to_clip(&mut project, &enemy_set, &enemy_walk, &enemy_a, 6).unwrap();
        append_frame_to_clip(&mut project, &enemy_set, &enemy_walk, &enemy_b, 6).unwrap();
        append_frame_to_clip(&mut project, &enemy_set, &enemy_defeat, &defeat, 8).unwrap();
        ensure_legacy_blueprints(&mut project);
        let enemy_blueprint = project
            .blueprints
            .iter_mut()
            .find(|blueprint| blueprint.visual_id == "goomba")
            .unwrap();
        enemy_blueprint.anim_set_id = enemy_set;
        project
    }

    fn course_project(animated: bool) -> Project {
        let mut cells = Vec::new();
        for x in 0..24 {
            cells.push(Cell {
                x,
                y: 13,
                asset_id: "ground".into(),
                layer: Layer::Main,
                ..Cell::default()
            });
        }
        cells.push(Cell {
            x: 5,
            y: 12,
            asset_id: "goomba".into(),
            layer: Layer::Main,
            ..Cell::default()
        });
        let mut project = Project::blank("play-anim");
        project.character_physics = CharacterPhysics::Smb1Mario;
        project.power_up_style = PowerUpStyle::Smb1;
        project.enemy_ai = EnemyAiStyle::Smb1Nes;
        project.assets = vec![
            solid_asset(
                "ground",
                "Ground",
                AssetKind::Terrain,
                [70, 140, 70],
                16,
                16,
            ),
            solid_asset("goomba", "Goomba", AssetKind::Enemy, [120, 70, 40], 16, 16),
            solid_asset("gold", "Gold", AssetKind::Enemy, [220, 180, 40], 16, 16),
            solid_asset("idle", "Idle", AssetKind::Player, [200, 40, 40], 16, 16),
            solid_asset("stride", "Stride", AssetKind::Player, [40, 80, 200], 16, 16),
        ];
        project.levels = vec![Level {
            id: "course".into(),
            name: "Play path".into(),
            style_id: String::new(),
            w: 24,
            h: 15,
            sky: "#24343c".into(),
            music_id: None,
            player: Spawn { x: 2.0, y: 12.0 },
            cells,
            enemy_ai: EnemyAiStyle::Smb1Nes,
            entrances: Vec::new(),
            player_blueprint_id: None,
        }];
        if animated {
            let sprite = create_sprite(&mut project, "Explorer", 16, 16).unwrap();
            let idle_frame = project.sprites[0].frames[0].id.clone();
            paint(&mut project, &sprite, &idle_frame, "idle", 0, 0);
            let walk_frame = duplicate_frame(&mut project, &sprite, &idle_frame).unwrap();
            project.sprites[0]
                .frames
                .iter_mut()
                .find(|frame| frame.id == walk_frame)
                .unwrap()
                .parts[0]
                .asset_id = "stride".into();
            let set = create_set(&mut project, "Explorer", &sprite).unwrap();
            let idle = add_clip(&mut project, &set, "idle").unwrap();
            let walk = add_clip(&mut project, &set, "walk").unwrap();
            append_frame_to_clip(&mut project, &set, &idle, &idle_frame, 8).unwrap();
            append_frame_to_clip(&mut project, &set, &walk, &walk_frame, 8).unwrap();
            let player = new_player_blueprint("Explorer", "idle", CharacterPhysics::Smb1Mario);
            let player_id = player.id.clone();
            project.player_blueprints.push(player);
            project.levels[0].player_blueprint_id = Some(player_id.clone());
            bind_player(&mut project, &player_id, &set).unwrap();
        }
        let _ = TILE;
        project
    }
}
