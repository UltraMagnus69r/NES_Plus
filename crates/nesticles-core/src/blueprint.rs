//! Gameplay records for Maker placements.
//!
//! A blueprint points at one visual [`crate::project::Asset`] and stores the
//! behavior that placement should execute. Many cells may reference one blueprint.
//! Pixel bytes stay on the asset.
//!
//! [`ensure_legacy_blueprints`] captures the legacy name and tag reading once.
//! After that, [`crate::engine::res_1080p::World::from_level`] executes the stored
//! role. [`crate::engine::res_1080p::World::tick`] does not re-read asset names.
//! A cell added after the cutover, with no blueprint id, is decorative.

use crate::enemy_ai::{EnemyAiStyle, EnemyArchetype};
use crate::engine::res_1080p::interpret_cell;
use crate::engine::res_1080p::powerups::smb1::ItemKind;
use crate::engine::res_1080p::rules::smb1::collision::BlockKind;
use crate::engine::res_1080p::rules::smb1::enemies::Kind;
use crate::engine::res_1080p::rules::smb1::hazards::HazardKind;
use crate::project::{Asset, AssetKind, Cell, Project};
use serde::{Deserialize, Serialize};

/// Where a blueprint came from. Migration never rewrites an authored record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlueprintOrigin {
    #[default]
    LegacyInfer,
    Authored,
}

/// Maker-facing classification. It is not executed by the sim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationalLabel {
    #[default]
    Unclassified,
    Player,
    Enemy,
    Terrain,
    Background,
    Foreground,
    Item,
    Hazard,
    Goal,
    Sfx,
    Music,
}

impl OrganizationalLabel {
    pub fn from_kind(kind: AssetKind) -> Self {
        match kind {
            AssetKind::Player => Self::Player,
            AssetKind::Enemy => Self::Enemy,
            AssetKind::Terrain => Self::Terrain,
            AssetKind::Background => Self::Background,
            AssetKind::Foreground => Self::Foreground,
            AssetKind::Item => Self::Item,
            AssetKind::Hazard => Self::Hazard,
            AssetKind::Goal => Self::Goal,
            AssetKind::Sfx => Self::Sfx,
            AssetKind::Music => Self::Music,
        }
    }
}

/// Enemy identity `kind_of` / `kind_from_name` can produce, plus the actors the
/// roster only spawns after a tick has started.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyEnemy {
    Goomba,
    GreenKoopa,
    Buzzy,
    RedKoopa,
    GreenPara,
    RedPara,
    Piranha,
    Lakitu,
    HammerBro,
    Blaster,
    Blooper,
    CheepGreen,
    CheepRed,
    CheepLeap,
    Podoboo,
    Bowser,
    Spiny,
    /// Not produced by name or archetype inference. Bullets are spawned.
    Bill,
    /// Not produced by name or archetype inference. A stomp creates a shell.
    Shell,
    /// Not produced by name or archetype inference. Hammer Bros spawn these.
    Hammer,
    /// Not produced by name or archetype inference. Lakitu spawns these.
    SpinyEgg,
}

/// Block identity from `block_tag` or `block_name`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyBlock {
    Solid,
    Question,
    Brick,
    Coin,
}

/// Item identity from `item_tag` or `item_name`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyItem {
    Mushroom,
    OneUp,
    Flower,
    Star,
    Plain,
}

/// Hazard identity from `hazard_tag` or `hazard_name`.
/// `balls` is the fire-bar length the sim actually builds (6, or 12 when the
/// tag is long or the name contains "12").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyHazard {
    Contact,
    Spring,
    Vine,
    FireBar {
        #[serde(default = "default_firebar_balls")]
        balls: i32,
    },
    PipeDown,
    PipeRight,
}

fn default_firebar_balls() -> i32 {
    6
}

/// Effective pipe exit. Cell warp wins over the asset warp, matching `warp_of`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LegacyWarp {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

/// Behavior captured from the live readers. New fields belong on these variants
/// with serde defaults; this is not a component list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LegacyRole {
    /// Audio, or a stamp that builds no actor. The sim skips it.
    #[default]
    Inert,
    Enemy {
        kind: LegacyEnemy,
        /// Archetype stored on the cell. Generic archetypes still fall through
        /// to the asset name, but they are not the same placement.
        archetype: EnemyArchetype,
        /// Per-cell brain override. `None` inherits the course, then the project.
        #[serde(default)]
        enemy_ai: Option<EnemyAiStyle>,
        #[serde(default)]
        warp: Option<LegacyWarp>,
    },
    Goal {
        #[serde(default)]
        warp: Option<LegacyWarp>,
    },
    /// Solid, item, and hazard can stack the way `World::from_level` stacks them.
    /// A pipe is a solid plus a pipe hazard. A terrain fire bar is both.
    Fixture {
        #[serde(default)]
        block: Option<LegacyBlock>,
        #[serde(default)]
        item: Option<LegacyItem>,
        #[serde(default)]
        hazard: Option<LegacyHazard>,
        #[serde(default)]
        warp: Option<LegacyWarp>,
    },
}

impl LegacyRole {
    pub fn enemy_kind(&self) -> Option<LegacyEnemy> {
        match self {
            Self::Enemy { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    pub fn block(&self) -> Option<LegacyBlock> {
        match self {
            Self::Fixture { block, .. } => *block,
            _ => None,
        }
    }
}

/// One behavior definition for a visual asset. Pixels stay on the asset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blueprint {
    #[serde(default)]
    pub id: String,
    /// [`Asset::id`]. Not a copy of `rgba_base64`.
    #[serde(default)]
    pub visual_id: String,
    #[serde(default)]
    pub origin: BlueprintOrigin,
    #[serde(default)]
    pub organizational_label: OrganizationalLabel,
    #[serde(default)]
    pub legacy_role: LegacyRole,
    #[serde(default)]
    pub notes: String,
}

/// The legacy reading `World::from_level` uses for this asset and placement.
///
/// Explicit tags win over names because that is what `resolve_block`,
/// `resolve_item`, and `resolve_hazard` already do. Enemy archetype wins over
/// the asset name except for the generic archetypes, which call `kind_from_name`.
/// A Podoboo tag or a name containing "podoboo" is an enemy, not a contact hazard.
pub fn infer_legacy(asset: &Asset, cell: &Cell) -> LegacyRole {
    let read = interpret_cell(asset, cell);
    let warp = if read.warp_set {
        Some(LegacyWarp {
            level_id: read.warp_level,
            x: read.warp_x,
            y: read.warp_y,
        })
    } else {
        None
    };
    if read.audio {
        return role_or_inert(None, None, None, warp);
    }
    if read.podoboo {
        return LegacyRole::Enemy {
            kind: LegacyEnemy::Podoboo,
            archetype: cell.archetype.clone(),
            enemy_ai: cell.enemy_ai,
            warp,
        };
    }
    if let Some(kind) = read.enemy {
        return LegacyRole::Enemy {
            kind: legacy_enemy(kind),
            archetype: cell.archetype.clone(),
            enemy_ai: cell.enemy_ai,
            warp,
        };
    }
    if read.goal {
        return LegacyRole::Goal { warp };
    }
    role_or_inert(
        read.block.map(legacy_block),
        read.item.map(legacy_item),
        read.hazard.map(|kind| legacy_hazard(kind, read.balls)),
        warp,
    )
}

fn role_or_inert(
    block: Option<LegacyBlock>,
    item: Option<LegacyItem>,
    hazard: Option<LegacyHazard>,
    warp: Option<LegacyWarp>,
) -> LegacyRole {
    if block.is_none() && item.is_none() && hazard.is_none() && warp.is_none() {
        LegacyRole::Inert
    } else {
        LegacyRole::Fixture {
            block,
            item,
            hazard,
            warp,
        }
    }
}

/// Capture legacy behavior for placements that do not yet have a blueprint.
///
/// Call this when a project is loaded into the editor. Do not call it from
/// `World::tick`. It does not write a file.
///
/// - The first call records every current cell that has an asset and no
///   `blueprint_id`, then sets [`Project::blueprint_cutover`].
/// - Cells that already have `blueprint_id` are left alone.
/// - An existing blueprint with the computed id is reused and not rewritten,
///   including when its origin is [`BlueprintOrigin::Authored`].
/// - Ids are a hash of the visual id plus the inferred role, so a second run
///   and a later load do not mint new ids.
/// - Asset names, kinds, tags, and gameplay settings are not modified.
/// - After the cutover, a later call returns immediately. Cells placed after
///   that stay unassigned, and the sim treats them as decorative instead of
///   reading their names.
pub fn ensure_legacy_blueprints(project: &mut Project) {
    if project.blueprint_cutover {
        return;
    }
    let mut pending = Vec::new();
    for (level_index, level) in project.levels.iter().enumerate() {
        for (cell_index, cell) in level.cells.iter().enumerate() {
            if cell.blueprint_id.is_some() {
                continue;
            }
            let Some(asset) = project
                .assets
                .iter()
                .find(|asset| asset.id == cell.asset_id)
            else {
                continue;
            };
            let legacy_role = infer_legacy(asset, cell);
            let id = deterministic_id(&asset.id, &legacy_role);
            pending.push((
                level_index,
                cell_index,
                Blueprint {
                    id,
                    visual_id: asset.id.clone(),
                    origin: BlueprintOrigin::LegacyInfer,
                    organizational_label: OrganizationalLabel::from_kind(asset.kind),
                    legacy_role,
                    notes: String::new(),
                },
            ));
        }
    }
    for (level_index, cell_index, blueprint) in pending {
        let id = blueprint.id.clone();
        if !project
            .blueprints
            .iter()
            .any(|existing| existing.id == blueprint.id)
        {
            project.blueprints.push(blueprint);
        }
        project.levels[level_index].cells[cell_index].blueprint_id = Some(id);
    }
    project.blueprint_cutover = true;
}

/// Parse project JSON and capture legacy blueprints in memory.
///
/// The returned project is the editor copy. This function does not write the
/// source file, the ROM, or any other path.
pub fn load_project_json(text: &str) -> Result<Project, serde_json::Error> {
    let mut project = serde_json::from_str(text)?;
    ensure_legacy_blueprints(&mut project);
    Ok(project)
}

fn deterministic_id(visual_id: &str, role: &LegacyRole) -> String {
    let role_key = serde_json::to_string(role).unwrap_or_default();
    let mut material = String::from("legacy-v1\n");
    material.push_str(visual_id);
    material.push('\n');
    material.push_str(&role_key);
    format!("bp-{:016x}", fnv1a64(material.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn legacy_enemy(kind: Kind) -> LegacyEnemy {
    match kind {
        Kind::Goomba => LegacyEnemy::Goomba,
        Kind::GreenKoopa => LegacyEnemy::GreenKoopa,
        Kind::Buzzy => LegacyEnemy::Buzzy,
        Kind::RedKoopa => LegacyEnemy::RedKoopa,
        Kind::GreenPara => LegacyEnemy::GreenPara,
        Kind::RedPara => LegacyEnemy::RedPara,
        Kind::Piranha => LegacyEnemy::Piranha,
        Kind::Lakitu => LegacyEnemy::Lakitu,
        Kind::HammerBro => LegacyEnemy::HammerBro,
        Kind::Blaster => LegacyEnemy::Blaster,
        Kind::Blooper => LegacyEnemy::Blooper,
        Kind::CheepGreen => LegacyEnemy::CheepGreen,
        Kind::CheepRed => LegacyEnemy::CheepRed,
        Kind::CheepLeap => LegacyEnemy::CheepLeap,
        Kind::Podoboo => LegacyEnemy::Podoboo,
        Kind::Bowser => LegacyEnemy::Bowser,
        Kind::Spiny => LegacyEnemy::Spiny,
        Kind::Bill => LegacyEnemy::Bill,
        Kind::Shell => LegacyEnemy::Shell,
        Kind::Hammer => LegacyEnemy::Hammer,
        Kind::SpinyEgg => LegacyEnemy::SpinyEgg,
    }
}

fn legacy_block(kind: BlockKind) -> LegacyBlock {
    match kind {
        BlockKind::Solid => LegacyBlock::Solid,
        BlockKind::Question => LegacyBlock::Question,
        BlockKind::Brick => LegacyBlock::Brick,
        BlockKind::Coin => LegacyBlock::Coin,
    }
}

fn legacy_item(kind: ItemKind) -> LegacyItem {
    match kind {
        ItemKind::Mushroom => LegacyItem::Mushroom,
        ItemKind::OneUp => LegacyItem::OneUp,
        ItemKind::Flower => LegacyItem::Flower,
        ItemKind::Star => LegacyItem::Star,
        ItemKind::Plain => LegacyItem::Plain,
    }
}

fn legacy_hazard(kind: HazardKind, balls: i32) -> LegacyHazard {
    match kind {
        HazardKind::Contact => LegacyHazard::Contact,
        HazardKind::Spring => LegacyHazard::Spring,
        HazardKind::Vine => LegacyHazard::Vine,
        HazardKind::FireBar => LegacyHazard::FireBar { balls },
        HazardKind::PipeDown => LegacyHazard::PipeDown,
        HazardKind::PipeRight => LegacyHazard::PipeRight,
    }
}

#[cfg(test)]
#[path = "blueprint_tests.rs"]
mod blueprint_tests;
