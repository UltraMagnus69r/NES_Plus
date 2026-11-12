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
use crate::project::{Asset, AssetKind, Cell, PlacementWarp, Project, WarpTarget};
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
    /// Pops out of a question block or a brick as the existing coin-block collect.
    /// It is not a walking actor.
    Coin,
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
    PipeUp,
    PipeLeft,
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

/// Gameplay the maker can assign. The picture is not one of these choices.
/// Index 0 is decoration. The order is stable so the editor can remember a brush.
pub fn gameplay_options() -> Vec<(&'static str, LegacyRole)> {
    let mut options = vec![
        ("Decoration", LegacyRole::Inert),
        ("Solid", fixture(Some(LegacyBlock::Solid), None, None)),
        (
            "Question block",
            fixture(Some(LegacyBlock::Question), None, None),
        ),
        ("Brick", fixture(Some(LegacyBlock::Brick), None, None)),
        ("Coin block", fixture(Some(LegacyBlock::Coin), None, None)),
        ("Mushroom", fixture(None, Some(LegacyItem::Mushroom), None)),
        ("1-Up", fixture(None, Some(LegacyItem::OneUp), None)),
        ("Fire flower", fixture(None, Some(LegacyItem::Flower), None)),
        ("Star", fixture(None, Some(LegacyItem::Star), None)),
        ("Item", fixture(None, Some(LegacyItem::Plain), None)),
        ("Contact", fixture(None, None, Some(LegacyHazard::Contact))),
        ("Spring", fixture(None, None, Some(LegacyHazard::Spring))),
        ("Vine", fixture(None, None, Some(LegacyHazard::Vine))),
        (
            "Fire bar",
            fixture(None, None, Some(LegacyHazard::FireBar { balls: 6 })),
        ),
        (
            "Long fire bar",
            fixture(None, None, Some(LegacyHazard::FireBar { balls: 12 })),
        ),
        (
            "Pipe down",
            fixture(Some(LegacyBlock::Solid), None, Some(LegacyHazard::PipeDown)),
        ),
        (
            "Pipe right",
            fixture(
                Some(LegacyBlock::Solid),
                None,
                Some(LegacyHazard::PipeRight),
            ),
        ),
        (
            "Pipe up",
            fixture(Some(LegacyBlock::Solid), None, Some(LegacyHazard::PipeUp)),
        ),
        (
            "Pipe left",
            fixture(Some(LegacyBlock::Solid), None, Some(LegacyHazard::PipeLeft)),
        ),
        ("Goal", LegacyRole::Goal { warp: None }),
    ];
    for kind in [
        LegacyEnemy::Goomba,
        LegacyEnemy::GreenKoopa,
        LegacyEnemy::Buzzy,
        LegacyEnemy::RedKoopa,
        LegacyEnemy::GreenPara,
        LegacyEnemy::RedPara,
        LegacyEnemy::Piranha,
        LegacyEnemy::Lakitu,
        LegacyEnemy::HammerBro,
        LegacyEnemy::Blaster,
        LegacyEnemy::Blooper,
        LegacyEnemy::CheepGreen,
        LegacyEnemy::CheepRed,
        LegacyEnemy::CheepLeap,
        LegacyEnemy::Podoboo,
        LegacyEnemy::Bowser,
        LegacyEnemy::Spiny,
    ] {
        options.push((enemy_label(kind), enemy_role(kind)));
    }
    options
}

/// Short name for a stored role. A pipe stays "Pipe down" even though it is also solid.
/// A block that holds a reward names both parts.
pub fn gameplay_summary(role: &LegacyRole) -> String {
    match role {
        LegacyRole::Fixture {
            block,
            item,
            hazard,
            ..
        } => describe_fixture(*block, *item, *hazard),
        other => gameplay_label(other).to_string(),
    }
}

/// Short name for a stored role. Archetype, enemy-style override, and warp do
/// not change the name. Combined fixtures use [`gameplay_summary`].
pub fn gameplay_label(role: &LegacyRole) -> &'static str {
    match role {
        LegacyRole::Inert => "Decoration",
        LegacyRole::Goal { .. } => "Goal",
        LegacyRole::Enemy { kind, .. } => enemy_label(*kind),
        LegacyRole::Fixture {
            hazard: Some(hazard),
            ..
        } => hazard_label(*hazard),
        LegacyRole::Fixture {
            item: Some(item), ..
        } => item_label(*item),
        LegacyRole::Fixture {
            block: Some(block), ..
        } => block_label(*block),
        LegacyRole::Fixture { .. } => "Decoration",
    }
}

/// Label of the topmost part at this cell, or `None` when the cell is empty.
/// A part with no blueprint reads as decoration.
pub fn placement_label(project: &Project, level_id: &str, x: i32, y: i32) -> Option<String> {
    let level = project.levels.iter().find(|level| level.id == level_id)?;
    let cell = top_cell(level, x, y)?;
    let name = project
        .assets
        .iter()
        .find(|asset| asset.id == cell.asset_id)
        .map(|asset| asset.name.as_str())
        .unwrap_or("part");
    let gameplay = cell
        .blueprint_id
        .as_deref()
        .and_then(|id| {
            project
                .blueprints
                .iter()
                .find(|blueprint| blueprint.id == id)
        })
        .map(|blueprint| gameplay_summary(&blueprint.legacy_role))
        .unwrap_or_else(|| "Decoration".to_string());
    Some(format!("{name} at {x},{y} — {gameplay}"))
}

/// Point one placement at a blueprint with this role.
///
/// An existing blueprint with the same picture and the same role is reused and
/// not rewritten. Otherwise an [`BlueprintOrigin::Authored`] record is added.
/// Other placements that shared the previous blueprint keep it. The asset name,
/// kind, pixels, and tags are not modified.
pub fn assign_cell(
    project: &mut Project,
    level_id: &str,
    x: i32,
    y: i32,
    role: LegacyRole,
) -> Result<String, String> {
    let level_index = project
        .levels
        .iter()
        .position(|level| level.id == level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    let cell_index = top_cell_index(&project.levels[level_index], x, y)
        .ok_or_else(|| format!("no part at {x},{y}"))?;
    let asset_id = project.levels[level_index].cells[cell_index]
        .asset_id
        .clone();
    let Some(asset) = project.assets.iter().find(|asset| asset.id == asset_id) else {
        return Err(format!("placement at {x},{y} has no asset"));
    };
    if asset.kind.is_audio() {
        return Err("audio is not a gameplay placement".into());
    }
    let visual_id = asset.id.clone();
    let organizational_label = OrganizationalLabel::from_kind(asset.kind);
    let kept_warp = {
        let cell = &project.levels[level_index].cells[cell_index];
        if cell.warp_choice.is_inherit() {
            cell.blueprint_id.as_ref().and_then(|id| {
                project
                    .blueprints
                    .iter()
                    .find(|blueprint| &blueprint.id == id)
                    .and_then(|blueprint| role_warp(&blueprint.legacy_role))
            })
        } else {
            None
        }
    };
    // The destination stays on the placement, so two pipes can share this role.
    let (behavior, explicit_warp) = without_warp(role);
    let id = if let Some(existing) = project
        .blueprints
        .iter()
        .find(|blueprint| blueprint.visual_id == visual_id && blueprint.legacy_role == behavior)
    {
        existing.id.clone()
    } else {
        let id = authored_id(&visual_id, &behavior);
        if !project
            .blueprints
            .iter()
            .any(|blueprint| blueprint.id == id)
        {
            project.blueprints.push(Blueprint {
                id: id.clone(),
                visual_id,
                origin: BlueprintOrigin::Authored,
                organizational_label,
                legacy_role: behavior.clone(),
                notes: String::new(),
            });
        }
        id
    };
    let archetype = match &behavior {
        LegacyRole::Enemy { archetype, .. } => Some(archetype.clone()),
        _ => None,
    };
    let cell = &mut project.levels[level_index].cells[cell_index];
    cell.blueprint_id = Some(id.clone());
    if let Some(archetype) = archetype {
        cell.archetype = archetype;
    }
    if let Some(warp) = explicit_warp {
        cell.warp_choice = PlacementWarp::Destination(warp_target(warp));
    } else if let Some(warp) = kept_warp {
        cell.warp_choice = PlacementWarp::Destination(warp_target(warp));
    }
    Ok(id)
}

/// Parts of the fixture on this placement. `Ok(None)` when it is not a fixture.
pub fn placed_fixture(
    project: &Project,
    level_id: &str,
    x: i32,
    y: i32,
) -> Result<Option<FixtureParts>, String> {
    let role = placed_role(project, level_id, x, y)?;
    Ok(match role {
        LegacyRole::Fixture {
            block,
            item,
            hazard,
            ..
        } => Some(FixtureParts {
            block,
            item,
            hazard,
        }),
        _ => None,
    })
}

/// Build a fixture from the three supported parts.
///
/// A reward is held inside a question block or a brick. A loose item, with no
/// block, stays a placed item. A coin has no actor, so it only exists as that
/// held reward. A solid does not release a reward.
pub fn compose_fixture(
    block: Option<LegacyBlock>,
    item: Option<LegacyItem>,
    hazard: Option<LegacyHazard>,
) -> Result<LegacyRole, String> {
    if item == Some(LegacyItem::Coin)
        && !matches!(block, Some(LegacyBlock::Question | LegacyBlock::Brick))
    {
        return Err(
            "A coin reward needs a question block or a brick. A coin block is already a coin."
                .into(),
        );
    }
    if item.is_some() && matches!(block, Some(LegacyBlock::Solid | LegacyBlock::Coin)) {
        return Err("Only a question block or a brick can hold a reward.".into());
    }
    if block.is_none() && item.is_none() && hazard.is_none() {
        return Ok(LegacyRole::Inert);
    }
    Ok(fixture(block, item, hazard))
}

/// Set or clear this placement's pipe exit. Does not change the blueprint or the picture.
pub fn set_placement_warp(
    project: &mut Project,
    level_id: &str,
    x: i32,
    y: i32,
    choice: PlacementWarp,
) -> Result<(), String> {
    if let PlacementWarp::Destination(target) = &choice {
        validate_warp(project, level_id, target)?;
    }
    let level_index = project
        .levels
        .iter()
        .position(|level| level.id == level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    let cell_index = top_cell_index(&project.levels[level_index], x, y)
        .ok_or_else(|| format!("no part at {x},{y}"))?;
    project.levels[level_index].cells[cell_index].warp_choice = choice;
    Ok(())
}

/// Reject a destination the player should not be sent to.
/// An empty level id means `source_level_id`, the course they are already in.
pub fn validate_warp(
    project: &Project,
    source_level_id: &str,
    target: &WarpTarget,
) -> Result<(), String> {
    let (id, level) = if target.level_id.is_empty() {
        (
            source_level_id,
            project
                .level(source_level_id)
                .ok_or_else(|| format!("no course with id {source_level_id}"))?,
        )
    } else {
        (
            target.level_id.as_str(),
            project.level(&target.level_id).ok_or_else(|| {
                format!(
                    "No course with id {}. The warp was not saved.",
                    target.level_id
                )
            })?,
        )
    };
    if target.x < 0 || target.y < 0 || target.x >= level.w as i32 || target.y >= level.h as i32 {
        return Err(format!(
            "Destination {},{} is outside {} ({} by {}). The warp was not saved.",
            target.x, target.y, id, level.w, level.h
        ));
    }
    Ok(())
}

/// Where a pending warp should put the player. Missing courses and coordinates
/// outside that course are errors. The player is not moved.
pub fn resolve_warp_arrival(
    project: &Project,
    from_level: &str,
    level_id: &str,
    x: i32,
    y: i32,
) -> Result<(String, i32, i32), String> {
    let target = WarpTarget {
        level_id: level_id.to_string(),
        x,
        y,
    };
    validate_warp(project, from_level, &target)?;
    let id = if level_id.is_empty() {
        from_level.to_string()
    } else {
        level_id.to_string()
    };
    Ok((id, x, y))
}

/// Enemy role the roster dropdown can write. Generic archetypes are `None`
/// because those still mean "read the picture name," and authoring must not.
pub fn enemy_from_archetype(kind: &EnemyArchetype) -> Option<LegacyEnemy> {
    Some(match kind {
        EnemyArchetype::Goomba => LegacyEnemy::Goomba,
        EnemyArchetype::GreenKoopa => LegacyEnemy::GreenKoopa,
        EnemyArchetype::BuzzyBeetle => LegacyEnemy::Buzzy,
        EnemyArchetype::RedKoopa => LegacyEnemy::RedKoopa,
        EnemyArchetype::GreenParatroopa => LegacyEnemy::GreenPara,
        EnemyArchetype::RedParatroopa => LegacyEnemy::RedPara,
        EnemyArchetype::GreenPiranha => LegacyEnemy::Piranha,
        EnemyArchetype::Lakitu => LegacyEnemy::Lakitu,
        EnemyArchetype::HammerBro => LegacyEnemy::HammerBro,
        EnemyArchetype::BulletBill => LegacyEnemy::Blaster,
        EnemyArchetype::Blooper => LegacyEnemy::Blooper,
        EnemyArchetype::CheepGreen => LegacyEnemy::CheepGreen,
        EnemyArchetype::CheepRed => LegacyEnemy::CheepRed,
        EnemyArchetype::CheepLeap => LegacyEnemy::CheepLeap,
        EnemyArchetype::Podoboo => LegacyEnemy::Podoboo,
        EnemyArchetype::Bowser => LegacyEnemy::Bowser,
        EnemyArchetype::Spiny => LegacyEnemy::Spiny,
        _ => return None,
    })
}

pub fn enemy_role(kind: LegacyEnemy) -> LegacyRole {
    LegacyRole::Enemy {
        kind,
        archetype: archetype_for(kind),
        enemy_ai: None,
        warp: None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixtureParts {
    pub block: Option<LegacyBlock>,
    pub item: Option<LegacyItem>,
    pub hazard: Option<LegacyHazard>,
}

fn placed_role(project: &Project, level_id: &str, x: i32, y: i32) -> Result<LegacyRole, String> {
    let level = project
        .levels
        .iter()
        .find(|level| level.id == level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    let cell = top_cell(level, x, y).ok_or_else(|| format!("no part at {x},{y}"))?;
    let Some(id) = cell.blueprint_id.as_deref() else {
        return Ok(LegacyRole::Inert);
    };
    project
        .blueprints
        .iter()
        .find(|blueprint| blueprint.id == id)
        .map(|blueprint| blueprint.legacy_role.clone())
        .ok_or_else(|| format!("blueprint \"{id}\" was not found"))
}

fn without_warp(role: LegacyRole) -> (LegacyRole, Option<LegacyWarp>) {
    match role {
        LegacyRole::Fixture {
            block,
            item,
            hazard,
            warp,
        } => (
            LegacyRole::Fixture {
                block,
                item,
                hazard,
                warp: None,
            },
            warp,
        ),
        LegacyRole::Enemy {
            kind,
            archetype,
            enemy_ai,
            warp,
        } => (
            LegacyRole::Enemy {
                kind,
                archetype,
                enemy_ai,
                warp: None,
            },
            warp,
        ),
        LegacyRole::Goal { warp } => (LegacyRole::Goal { warp: None }, warp),
        other => (other, None),
    }
}

fn role_warp(role: &LegacyRole) -> Option<LegacyWarp> {
    match role {
        LegacyRole::Fixture { warp, .. }
        | LegacyRole::Enemy { warp, .. }
        | LegacyRole::Goal { warp } => warp.clone(),
        LegacyRole::Inert => None,
    }
}

fn warp_target(warp: LegacyWarp) -> WarpTarget {
    WarpTarget {
        level_id: warp.level_id,
        x: warp.x,
        y: warp.y,
    }
}

fn describe_fixture(
    block: Option<LegacyBlock>,
    item: Option<LegacyItem>,
    hazard: Option<LegacyHazard>,
) -> String {
    if let Some(
        pipe @ (LegacyHazard::PipeDown
        | LegacyHazard::PipeUp
        | LegacyHazard::PipeLeft
        | LegacyHazard::PipeRight),
    ) = hazard
    {
        return hazard_label(pipe).to_string();
    }
    match (block, item, hazard) {
        (Some(block), Some(item), None) => {
            format!("{} with {}", block_label(block), item_label(item))
        }
        (Some(block), None, Some(hazard)) => {
            format!("{} with {}", block_label(block), hazard_label(hazard))
        }
        (Some(block), Some(item), Some(hazard)) => format!(
            "{} with {} and {}",
            block_label(block),
            item_label(item),
            hazard_label(hazard)
        ),
        (Some(block), None, None) => block_label(block).to_string(),
        (None, Some(item), None) => item_label(item).to_string(),
        (None, None, Some(hazard)) => hazard_label(hazard).to_string(),
        (None, Some(item), Some(hazard)) => {
            format!("{} with {}", item_label(item), hazard_label(hazard))
        }
        (None, None, None) => "Decoration".to_string(),
    }
}

fn fixture(
    block: Option<LegacyBlock>,
    item: Option<LegacyItem>,
    hazard: Option<LegacyHazard>,
) -> LegacyRole {
    LegacyRole::Fixture {
        block,
        item,
        hazard,
        warp: None,
    }
}

fn enemy_label(kind: LegacyEnemy) -> &'static str {
    match kind {
        LegacyEnemy::Goomba => "Goomba",
        LegacyEnemy::GreenKoopa => "Green Koopa",
        LegacyEnemy::Buzzy => "Buzzy Beetle",
        LegacyEnemy::RedKoopa => "Red Koopa",
        LegacyEnemy::GreenPara => "Green Paratroopa",
        LegacyEnemy::RedPara => "Red Paratroopa",
        LegacyEnemy::Piranha => "Green Piranha",
        LegacyEnemy::Lakitu => "Lakitu",
        LegacyEnemy::HammerBro => "Hammer Bro",
        LegacyEnemy::Blaster => "Bullet Bill",
        LegacyEnemy::Blooper => "Blooper",
        LegacyEnemy::CheepGreen => "Cheep Cheep",
        LegacyEnemy::CheepRed => "Red Cheep Cheep",
        LegacyEnemy::CheepLeap => "Leaping Cheep Cheep",
        LegacyEnemy::Podoboo => "Podoboo",
        LegacyEnemy::Bowser => "Bowser",
        LegacyEnemy::Spiny => "Spiny",
        LegacyEnemy::Bill => "Bullet",
        LegacyEnemy::Shell => "Shell",
        LegacyEnemy::Hammer => "Hammer",
        LegacyEnemy::SpinyEgg => "Spiny egg",
    }
}

fn block_label(kind: LegacyBlock) -> &'static str {
    match kind {
        LegacyBlock::Solid => "Solid",
        LegacyBlock::Question => "Question block",
        LegacyBlock::Brick => "Brick",
        LegacyBlock::Coin => "Coin block",
    }
}

fn item_label(kind: LegacyItem) -> &'static str {
    match kind {
        LegacyItem::Mushroom => "Mushroom",
        LegacyItem::OneUp => "1-Up",
        LegacyItem::Flower => "Fire flower",
        LegacyItem::Star => "Star",
        LegacyItem::Plain => "Item",
        LegacyItem::Coin => "Coin",
    }
}

fn hazard_label(kind: LegacyHazard) -> &'static str {
    match kind {
        LegacyHazard::Contact => "Contact",
        LegacyHazard::Spring => "Spring",
        LegacyHazard::Vine => "Vine",
        LegacyHazard::FireBar { balls: 12 } => "Long fire bar",
        LegacyHazard::FireBar { .. } => "Fire bar",
        LegacyHazard::PipeDown => "Pipe down",
        LegacyHazard::PipeUp => "Pipe up",
        LegacyHazard::PipeLeft => "Pipe left",
        LegacyHazard::PipeRight => "Pipe right",
    }
}

fn archetype_for(kind: LegacyEnemy) -> EnemyArchetype {
    match kind {
        LegacyEnemy::Goomba => EnemyArchetype::Goomba,
        LegacyEnemy::GreenKoopa => EnemyArchetype::GreenKoopa,
        LegacyEnemy::Buzzy => EnemyArchetype::BuzzyBeetle,
        LegacyEnemy::RedKoopa => EnemyArchetype::RedKoopa,
        LegacyEnemy::GreenPara => EnemyArchetype::GreenParatroopa,
        LegacyEnemy::RedPara => EnemyArchetype::RedParatroopa,
        LegacyEnemy::Piranha => EnemyArchetype::GreenPiranha,
        LegacyEnemy::Lakitu => EnemyArchetype::Lakitu,
        LegacyEnemy::HammerBro => EnemyArchetype::HammerBro,
        LegacyEnemy::Blaster | LegacyEnemy::Bill => EnemyArchetype::BulletBill,
        LegacyEnemy::Blooper => EnemyArchetype::Blooper,
        LegacyEnemy::CheepGreen => EnemyArchetype::CheepGreen,
        LegacyEnemy::CheepRed => EnemyArchetype::CheepRed,
        LegacyEnemy::CheepLeap => EnemyArchetype::CheepLeap,
        LegacyEnemy::Podoboo => EnemyArchetype::Podoboo,
        LegacyEnemy::Bowser => EnemyArchetype::Bowser,
        LegacyEnemy::Spiny | LegacyEnemy::SpinyEgg => EnemyArchetype::Spiny,
        LegacyEnemy::Shell => EnemyArchetype::ShellWalker,
        LegacyEnemy::Hammer => EnemyArchetype::Shooter,
    }
}

fn top_cell_index(level: &crate::project::Level, x: i32, y: i32) -> Option<usize> {
    level
        .cells
        .iter()
        .enumerate()
        .rev()
        .find(|(_, cell)| cell.x == x && cell.y == y)
        .map(|(index, _)| index)
}

fn top_cell(level: &crate::project::Level, x: i32, y: i32) -> Option<&Cell> {
    top_cell_index(level, x, y).map(|index| &level.cells[index])
}

fn authored_id(visual_id: &str, role: &LegacyRole) -> String {
    let role_key = serde_json::to_string(role).unwrap_or_default();
    let mut material = String::from("authored-v1\n");
    material.push_str(visual_id);
    material.push('\n');
    material.push_str(&role_key);
    format!("bp-{:016x}", fnv1a64(material.as_bytes()))
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
        HazardKind::PipeUp => LegacyHazard::PipeUp,
        HazardKind::PipeLeft => LegacyHazard::PipeLeft,
        HazardKind::PipeRight => LegacyHazard::PipeRight,
    }
}

#[cfg(test)]
#[path = "blueprint_tests.rs"]
mod blueprint_tests;
