//! Workshop project. Serialized as JSON. API keys never live in this struct.

use crate::blueprint::Blueprint;
use crate::enemy_ai::{EnemyAiStyle, EnemyArchetype};
use crate::physics::CharacterPhysics;
use crate::terrain::{TerrainBrush, TerrainCell};
use serde::{Deserialize, Serialize};

/// Power-up rules for play. Old projects omit the field and load SMB1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PowerUpStyle {
    #[default]
    Smb1,
}

impl PowerUpStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Smb1 => "SMB1",
        }
    }

    pub const ALL: [PowerUpStyle; 1] = [Self::Smb1];
}

/// Terrain bump behavior. `Auto` reads the asset name; unlabeled terrain stays solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BlockTag {
    #[default]
    Auto,
    Solid,
    Question,
    Brick,
    Coin,
}

/// Item behavior. `Auto` reads the asset name (mushroom, flower, star, 1-up).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ItemTag {
    #[default]
    Auto,
    Mushroom,
    OneUp,
    Flower,
    Star,
    Plain,
}

/// Hazard behavior. `Auto` reads the asset name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HazardTag {
    #[default]
    Auto,
    Contact,
    Spring,
    Vine,
    FireBar,
    FireBarLong,
    PipeDown,
    PipeRight,
    Podoboo,
}

/// Optional pipe exit. Missing on old cells and assets.
/// An empty `level_id` means the course the player is already in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WarpTarget {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

/// Where this placement sends a pipe. The picture does not own this.
///
/// `Inherit` uses a warp stored on an older blueprint, if that blueprint has one.
/// `Destination` is this placement's own exit. `Cleared` stays off even when the
/// blueprint still has an exit, so clearing cannot fall back to that exit.
/// A later area graph can treat `Destination` as an entrance id plus coordinates.
/// This phase does not add that graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlacementWarp {
    #[default]
    Inherit,
    Destination(WarpTarget),
    Cleared,
}

impl PlacementWarp {
    pub fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit)
    }
}

/// Which way the player faces after appearing at an entrance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Left,
    #[default]
    Right,
}

impl Facing {
    pub fn sign(self) -> i32 {
        match self {
            Self::Left => -1,
            Self::Right => 1,
        }
    }
}

/// Which input opens a pipe. This is gameplay, not a picture name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryDirection {
    Up,
    Down,
    Left,
    Right,
}

impl EntryDirection {
    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "Pipe up",
            Self::Down => "Pipe down",
            Self::Left => "Pipe left",
            Self::Right => "Pipe right",
        }
    }
}

/// Where the player appears. It does not send them anywhere by itself.
/// Coordinates are course cells, the same grid as placements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entrance {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub facing: Facing,
}

/// One directed link from a placed exit to an entrance.
/// A return trip is a second transition, with its own source and entrance.
/// Empty `destination_level_id` means the source course.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub source_level_id: String,
    pub source_x: i32,
    pub source_y: i32,
    #[serde(default)]
    pub destination_level_id: String,
    pub destination_entrance_id: String,
    pub entry: EntryDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
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

impl AssetKind {
    pub fn label(self) -> &'static str {
        match self {
            AssetKind::Player => "player",
            AssetKind::Enemy => "enemy",
            AssetKind::Terrain => "terrain",
            AssetKind::Background => "background",
            AssetKind::Foreground => "foreground",
            AssetKind::Item => "item",
            AssetKind::Hazard => "hazard",
            AssetKind::Goal => "goal",
            AssetKind::Sfx => "sfx",
            AssetKind::Music => "music",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text.trim().to_ascii_lowercase().as_str() {
            "player" => AssetKind::Player,
            "enemy" => AssetKind::Enemy,
            "terrain" | "ground" => AssetKind::Terrain,
            "background" | "back" => AssetKind::Background,
            "foreground" | "front" => AssetKind::Foreground,
            "item" => AssetKind::Item,
            "hazard" => AssetKind::Hazard,
            "goal" => AssetKind::Goal,
            "sfx" | "sound" => AssetKind::Sfx,
            "music" => AssetKind::Music,
            _ => return None,
        })
    }

    pub fn default_layer(self) -> Layer {
        match self {
            AssetKind::Background => Layer::Back,
            AssetKind::Foreground => Layer::Front,
            _ => Layer::Main,
        }
    }

    pub fn default_solid(self) -> bool {
        matches!(self, AssetKind::Terrain)
    }

    pub fn is_audio(self) -> bool {
        matches!(self, AssetKind::Sfx | AssetKind::Music)
    }

    pub const ALL: [AssetKind; 10] = [
        AssetKind::Player,
        AssetKind::Enemy,
        AssetKind::Terrain,
        AssetKind::Background,
        AssetKind::Foreground,
        AssetKind::Item,
        AssetKind::Hazard,
        AssetKind::Goal,
        AssetKind::Sfx,
        AssetKind::Music,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Back,
    Main,
    Front,
}

impl Layer {
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "back" | "background" => Layer::Back,
            "front" | "foreground" => Layer::Front,
            _ => Layer::Main,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Layer::Back => "back",
            Layer::Main => "main",
            Layer::Front => "front",
        }
    }
}

/// One imported cartridge, or the built-in original set. A style is a tray of parts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Style {
    pub id: String,
    pub name: String,
    /// File name of the cartridge, or "original" for parts drawn in Nesticles.
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub kind: AssetKind,
    /// Style this part belongs to. Empty means it shows in every style filter.
    #[serde(default)]
    pub style_id: String,
    pub w: u32,
    pub h: u32,
    #[serde(default)]
    pub rgba_base64: String,
    #[serde(default)]
    pub png_base64: String,
    #[serde(default)]
    pub wav_base64: String,
    #[serde(default)]
    pub solid: bool,
    /// Playback cue: jump, stomp, hurt, goal, music, or empty.
    #[serde(default)]
    pub cue: String,
    /// SMB1 item behavior. Missing on old saves, so the name is used.
    #[serde(default)]
    pub item_tag: ItemTag,
    /// Question, brick, coin block, or solid. Missing stays solid unless the name says otherwise.
    #[serde(default)]
    pub block_tag: BlockTag,
    /// Spring, vine, fire bar, pipe, or contact. Missing uses the name.
    #[serde(default)]
    pub hazard_tag: HazardTag,
    /// Default pipe exit for every cell that uses this part.
    #[serde(default)]
    pub warp: Option<WarpTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub x: i32,
    pub y: i32,
    pub asset_id: String,
    #[serde(default = "default_layer")]
    pub layer: Layer,
    /// None inherits the course style, then the project style.
    #[serde(default)]
    pub enemy_ai: Option<EnemyAiStyle>,
    #[serde(default)]
    pub archetype: EnemyArchetype,
    /// Pipe exit for this cell. Overrides the asset warp when set.
    /// Older courses use this. New authoring writes [`PlacementWarp`] instead.
    #[serde(default)]
    pub warp: Option<WarpTarget>,
    /// This placement's pipe exit. Missing on old courses, which means inherit.
    #[serde(default, skip_serializing_if = "PlacementWarp::is_inherit")]
    pub warp_choice: PlacementWarp,
    /// Behavior record for this placement. Missing on old courses.
    /// When set, `World::from_level` uses that blueprint and ignores the asset name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blueprint_id: Option<String>,
    /// Named exit for this placement. When set, it wins over [`PlacementWarp`].
    /// A missing or invalid exit does not fall back to a coordinate warp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_id: Option<String>,
    /// Connected-terrain membership. Absent on courses saved before Phase 7.
    /// A picture without this field stays an ordinary stamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<TerrainCell>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            asset_id: String::new(),
            layer: Layer::Main,
            enemy_ai: None,
            archetype: EnemyArchetype::Walker,
            warp: None,
            warp_choice: PlacementWarp::Inherit,
            blueprint_id: None,
            transition_id: None,
            terrain: None,
        }
    }
}

fn default_layer() -> Layer {
    Layer::Main
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Spawn {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    pub id: String,
    pub name: String,
    /// Home style, like a maker's selected game style. Parts from other styles can still be placed.
    #[serde(default)]
    pub style_id: String,
    pub w: u32,
    pub h: u32,
    #[serde(default = "default_sky")]
    pub sky: String,
    #[serde(default)]
    pub music_id: Option<String>,
    pub player: Spawn,
    #[serde(default)]
    pub cells: Vec<Cell>,
    /// Enemy brain for this course. Old projects load as SMB1.
    #[serde(default)]
    pub enemy_ai: EnemyAiStyle,
    /// Named places the player can appear. The course start in `player` stays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrances: Vec<Entrance>,
}

fn default_sky() -> String {
    "#24343c".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverNode {
    pub id: String,
    pub name: String,
    pub level_id: String,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub links: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    #[serde(default)]
    pub styles: Vec<Style>,
    #[serde(default)]
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub levels: Vec<Level>,
    #[serde(default)]
    pub nodes: Vec<OverNode>,
    #[serde(default)]
    pub start_node_id: String,
    /// Fallback enemy brain when a course has not chosen one. Default is SMB1.
    #[serde(default)]
    pub enemy_ai: EnemyAiStyle,
    /// Playable character controller. Old saves load as SMB1 - Mario.
    #[serde(default)]
    pub character_physics: CharacterPhysics,
    /// Power-up rules. Old saves load as SMB1.
    #[serde(default)]
    pub power_up_style: PowerUpStyle,
    /// Project schema. Old files omit it and load as 1.
    #[serde(default = "default_format")]
    pub format: u32,
    /// Shared behavior records. Old files omit it and load empty.
    #[serde(default)]
    pub blueprints: Vec<Blueprint>,
    /// Set when [`crate::blueprint::ensure_legacy_blueprints`] has captured the
    /// placements that existed at migration time. Old files omit it and load false.
    /// After it is true, a cell with no `blueprint_id` is decorative: the sim does
    /// not invent gameplay from the asset name or kind.
    #[serde(default, skip_serializing_if = "is_false")]
    pub blueprint_cutover: bool,
    /// Directed exits. Old files omit this and keep coordinate warps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<Transition>,
    /// Connected materials. Old projects omit this and load none.
    /// Opening a project does not turn existing stamps into terrain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain_brushes: Vec<TerrainBrush>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn default_format() -> u32 {
    1
}

impl Project {
    pub fn blank(name: &str) -> Self {
        Self {
            name: name.into(),
            styles: Vec::new(),
            assets: Vec::new(),
            levels: Vec::new(),
            nodes: Vec::new(),
            start_node_id: String::new(),
            enemy_ai: EnemyAiStyle::Smb1Nes,
            character_physics: CharacterPhysics::Smb1Mario,
            power_up_style: PowerUpStyle::Smb1,
            format: default_format(),
            blueprints: Vec::new(),
            blueprint_cutover: false,
            transitions: Vec::new(),
            terrain_brushes: Vec::new(),
        }
    }

    /// Add a style for a cartridge. The same source file does not create a second style.
    pub fn adopt_style(&mut self, name: &str, source: &str) -> String {
        if !source.is_empty() {
            if let Some(found) = self.styles.iter().find(|style| style.source == source) {
                return found.id.clone();
            }
        }
        let id = nid("style-");
        self.styles.push(Style {
            id: id.clone(),
            name: name.into(),
            source: source.into(),
        });
        id
    }

    pub fn style_name(&self, id: &str) -> String {
        self.styles
            .iter()
            .find(|style| style.id == id)
            .map(|style| style.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Unstyled".into())
    }

    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id == id)
    }

    pub fn asset_mut(&mut self, id: &str) -> Option<&mut Asset> {
        self.assets.iter_mut().find(|a| a.id == id)
    }

    pub fn level(&self, id: &str) -> Option<&Level> {
        self.levels.iter().find(|l| l.id == id)
    }

    pub fn level_mut(&mut self, id: &str) -> Option<&mut Level> {
        self.levels.iter_mut().find(|l| l.id == id)
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Project \"{}\"", self.name));
        lines.push("Styles (each imported cartridge is one style; courses may mix them):".into());
        for style in &self.styles {
            lines.push(format!(
                "  {} \"{}\" source={}",
                style.id, style.name, style.source
            ));
        }
        lines.push("Assets:".into());
        for asset in &self.assets {
            let style = self.style_name(&asset.style_id);
            if asset.kind.is_audio() {
                lines.push(format!(
                    "  {} [{}] style=\"{style}\" cue={}",
                    asset.id,
                    asset.kind.label(),
                    asset.cue
                ));
            } else {
                lines.push(format!(
                    "  {} \"{}\" [{}] style=\"{style}\" {}x{}",
                    asset.id,
                    asset.name,
                    asset.kind.label(),
                    asset.w,
                    asset.h
                ));
            }
        }
        for level in &self.levels {
            lines.push(format!(
                "Course {} \"{}\" style=\"{}\" {}x{} start=({}, {}) cells={} {}",
                level.id,
                level.name,
                self.style_name(&level.style_id),
                level.w,
                level.h,
                level.player.x,
                level.player.y,
                level.cells.len(),
                audit_course(self, &level.id).unwrap_or_default()
            ));
        }
        lines.push(format!("Overworld start node: {}", self.start_node_id));
        for node in &self.nodes {
            lines.push(format!(
                "  node {} \"{}\" level={} at ({:.0},{:.0}) links={:?}",
                node.id, node.name, node.level_id, node.x, node.y, node.links
            ));
        }
        lines.join("\n")
    }
}

pub fn audit_course(project: &Project, level_id: &str) -> Result<String, String> {
    let level = project
        .level(level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    let mut ground = false;
    let mut goal = false;
    for cell in &level.cells {
        let Some(asset) = project.asset(&cell.asset_id) else {
            continue;
        };
        if asset.kind == AssetKind::Terrain && (asset.solid || cell.layer == Layer::Main) {
            ground = true;
        }
        if asset.kind == AssetKind::Goal {
            goal = true;
        }
    }
    let start = level.player.x >= 0.0
        && level.player.y >= 0.0
        && level.player.x < level.w as f32
        && level.player.y < level.h as f32;
    Ok(format!(
        "ground={} start={} goal={}",
        if ground { "yes" } else { "no" },
        if start { "yes" } else { "no" },
        if goal { "yes" } else { "no" }
    ))
}

pub fn nid(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{prefix}{millis:x}{n:x}")
}
