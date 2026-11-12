//! Connected terrain for Maker Engine courses.
//!
//! A terrain brush is a material: which pictures may be used, which other
//! brushes it joins, and the gameplay role given to cells painted from now on.
//! Auto-tiling only chooses the picture. It does not change a cell's blueprint.
//!
//! # Connection
//!
//! Two terrain cells connect when both belong to brushes whose connection
//! group is the same non-empty name. An empty group means "only this brush,"
//! using the brush id. Sharing a picture does not connect brushes. A different
//! group never connects, which is the explicit rule for keeping materials apart.
//!
//! # Four-neighbor masks
//!
//! Cardinal bits are north = 1, east = 2, south = 4, west = 8. The mask is the
//! sum of the connected sides. Every mask has one graphic, so ties do not exist:
//!
//! | Mask | Sides | Graphic |
//! | --- | --- | --- |
//! | 0 | none | Isolated tile |
//! | 1 | N | Bottom end |
//! | 2 | E | Left end |
//! | 3 | N+E | Bottom-left corner |
//! | 4 | S | Top end |
//! | 5 | N+S | Vertical middle |
//! | 6 | E+S | Top-left corner |
//! | 7 | N+E+S | Left edge |
//! | 8 | W | Right end |
//! | 9 | N+W | Bottom-right corner |
//! | 10 | E+W | Middle |
//! | 11 | N+E+W | Bottom edge |
//! | 12 | S+W | Top-right corner |
//! | 13 | N+S+W | Right edge |
//! | 14 | E+S+W | Top edge |
//! | 15 | N+E+S+W | Fill |
//!
//! A left end is connected east and not west. A top end is connected south and
//! not north. Corners and edges are the same table, not a second set of names.
//!
//! # Eight-neighbor inner corners
//!
//! Diagonal cells are inspected only when both flanking sides are already
//! connected. A tile that only touches at a corner does not connect and does
//! not become an end or a corner. When both sides are connected and the
//! diagonal is absent or belongs to another group, the cell uses an inner
//! corner. That rule is more specific than the cardinal graphic, so it wins.
//!
//! If more than one inner corner matches, the order is fixed: top-left, then
//! top-right, then bottom-left, then bottom-right. Resolution never walks a
//! hash map and never depends on the direction of the stroke.
//!
//! # Missing pictures
//!
//! 1. The graphic assigned to the exact shape.
//! 2. The brush's fallback graphic.
//! 3. The first assigned graphic in this order: fill, middle, isolated tile,
//!    then the remaining shapes in [`ALL_SLOTS`] order.
//!
//! The editor reports every cell that did not have an exact graphic. Nesticles
//! does not invent pixels for a missing shape.
//!
//! # Painting
//!
//! [`pointer_cell`] uses the same 20-pixel grid as the Courses board. Strokes
//! outside the course are dropped. Freehand movement uses [`grid_line`], which
//! stays on cells that share a side. A diagonal step inserts the orthogonal
//! cell between its ends, so a fast drag does not leave a cardinal gap.
//! Horizontal and vertical tools keep the anchor's other axis. A rectangle is
//! the inclusive span of the two corners.
//!
//! One stroke recomputes the edited cells and the eight cells around each of
//! them. Four-neighbor brushes do not read diagonals, so those extra cells
//! keep the graphic they already resolved to. The rest of the course is not
//! visited. A stroke does not decode images and does not save the project.
//!
//! # Undo
//!
//! [`TerrainEdit`] stores the course cells before and after one finished
//! stroke, including neighbor graphics, groups, blueprints, and overrides.
//! History lives in the editor, not in the project file. Opening a project
//! clears it. The stack keeps the newest [`HISTORY_LIMIT`] strokes.
//!
//! # Placement
//!
//! Terrain is an extra cell marked with [`TerrainCell`]. Painting and erasing
//! touch only those cells. An enemy, item, hazard, pipe, or goal in the same
//! grid square stays. Replacing a terrain cell clears a manual picture and
//! keeps that cell's blueprint. A new cell receives the blueprint passed by
//! the caller, which is the brush's current role. Editing the brush role
//! later does not walk cells that are already painted.
//!
//! A manual override keeps its picture while neighbors are recomputed. Erasing
//! the cell removes the override. Painting the cell again clears it and
//! returns the cell to automatic selection.

use crate::blueprint::LegacyRole;
use crate::project::{Cell, Layer, Level};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Courses board cell, in screen pixels. Pointer conversion divides by this.
pub const EDITOR_CELL_PX: f32 = 20.0;

/// Finished strokes kept in memory. Older strokes are dropped.
pub const HISTORY_LIMIT: usize = 32;

const NORTH: u8 = 1;
const EAST: u8 = 2;
const SOUTH: u8 = 4;
const WEST: u8 = 8;
const INNER_NORTH_WEST: u8 = 1;
const INNER_NORTH_EAST: u8 = 2;
const INNER_SOUTH_WEST: u8 = 4;
const INNER_SOUTH_EAST: u8 = 8;

/// How many neighboring cells a brush reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NeighborMode {
    /// North, east, south, and west.
    #[default]
    Four,
    /// The four sides plus inner corners from the diagonals.
    Eight,
}

/// One connected shape. The names are the maker-facing shapes, not bit masks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainSlot {
    Isolated,
    HorizontalLeft,
    HorizontalMiddle,
    HorizontalRight,
    VerticalTop,
    VerticalMiddle,
    VerticalBottom,
    TopEdge,
    BottomEdge,
    LeftEdge,
    RightEdge,
    TopLeftOuter,
    TopRightOuter,
    BottomLeftOuter,
    BottomRightOuter,
    Interior,
    InnerTopLeft,
    InnerTopRight,
    InnerBottomLeft,
    InnerBottomRight,
}

/// Shape order used by the brush editor and by the general fallback.
pub const ALL_SLOTS: [TerrainSlot; 20] = [
    TerrainSlot::Isolated,
    TerrainSlot::HorizontalLeft,
    TerrainSlot::HorizontalMiddle,
    TerrainSlot::HorizontalRight,
    TerrainSlot::VerticalTop,
    TerrainSlot::VerticalMiddle,
    TerrainSlot::VerticalBottom,
    TerrainSlot::TopEdge,
    TerrainSlot::BottomEdge,
    TerrainSlot::LeftEdge,
    TerrainSlot::RightEdge,
    TerrainSlot::TopLeftOuter,
    TerrainSlot::TopRightOuter,
    TerrainSlot::BottomLeftOuter,
    TerrainSlot::BottomRightOuter,
    TerrainSlot::Interior,
    TerrainSlot::InnerTopLeft,
    TerrainSlot::InnerTopRight,
    TerrainSlot::InnerBottomLeft,
    TerrainSlot::InnerBottomRight,
];

const CARDINAL_SLOTS: [TerrainSlot; 16] = [
    TerrainSlot::Isolated,
    TerrainSlot::VerticalBottom,
    TerrainSlot::HorizontalLeft,
    TerrainSlot::BottomLeftOuter,
    TerrainSlot::VerticalTop,
    TerrainSlot::VerticalMiddle,
    TerrainSlot::TopLeftOuter,
    TerrainSlot::LeftEdge,
    TerrainSlot::HorizontalRight,
    TerrainSlot::BottomRightOuter,
    TerrainSlot::HorizontalMiddle,
    TerrainSlot::BottomEdge,
    TerrainSlot::TopRightOuter,
    TerrainSlot::RightEdge,
    TerrainSlot::TopEdge,
    TerrainSlot::Interior,
];

const FALLBACK_ORDER: [TerrainSlot; 20] = [
    TerrainSlot::Interior,
    TerrainSlot::HorizontalMiddle,
    TerrainSlot::Isolated,
    TerrainSlot::HorizontalLeft,
    TerrainSlot::HorizontalRight,
    TerrainSlot::VerticalTop,
    TerrainSlot::VerticalMiddle,
    TerrainSlot::VerticalBottom,
    TerrainSlot::TopEdge,
    TerrainSlot::BottomEdge,
    TerrainSlot::LeftEdge,
    TerrainSlot::RightEdge,
    TerrainSlot::TopLeftOuter,
    TerrainSlot::TopRightOuter,
    TerrainSlot::BottomLeftOuter,
    TerrainSlot::BottomRightOuter,
    TerrainSlot::InnerTopLeft,
    TerrainSlot::InnerTopRight,
    TerrainSlot::InnerBottomLeft,
    TerrainSlot::InnerBottomRight,
];

/// How the pointer lays down cells for one stroke.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TerrainTool {
    Single,
    #[default]
    Freehand,
    Horizontal,
    Vertical,
    Rectangle,
    Erase,
    Inspect,
}

pub const ALL_TOOLS: [TerrainTool; 7] = [
    TerrainTool::Single,
    TerrainTool::Freehand,
    TerrainTool::Horizontal,
    TerrainTool::Vertical,
    TerrainTool::Rectangle,
    TerrainTool::Erase,
    TerrainTool::Inspect,
];

/// One connected material. Pictures are asset ids, not copies of the pixels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBrush {
    pub id: String,
    pub name: String,
    /// Cells in the same group connect. Empty means this brush only joins itself.
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub mode: NeighborMode,
    /// Shape to asset id. An asset may be used for more than one shape.
    #[serde(default)]
    pub variants: BTreeMap<TerrainSlot, String>,
    /// Used when the exact shape has no graphic.
    #[serde(default)]
    pub fallback_asset: String,
    /// Gameplay for cells painted after this change. Existing cells keep theirs.
    #[serde(default)]
    pub default_role: LegacyRole,
    /// Optional maker note. It is not read as gameplay.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Default for TerrainBrush {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            group: String::new(),
            mode: NeighborMode::Four,
            variants: BTreeMap::new(),
            fallback_asset: String::new(),
            default_role: LegacyRole::Inert,
            notes: String::new(),
        }
    }
}

/// Membership of one terrain cell. The picture stays on [`Cell::asset_id`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainCell {
    pub brush_id: String,
    /// When set, neighbor updates do not replace [`Cell::asset_id`].
    #[serde(default)]
    pub manual: bool,
    /// Picture chosen by the maker. Present only while `manual` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_asset: Option<String>,
}

/// One finished stroke. `before` and `after` are the whole course cell list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainEdit {
    pub level_index: usize,
    pub before: Vec<Cell>,
    pub after: Vec<Cell>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingVariant {
    pub x: i32,
    pub y: i32,
    pub slot: TerrainSlot,
    pub asset_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrokeReport {
    pub painted: usize,
    pub erased: usize,
    /// Cells that did not exist as terrain before this stroke.
    pub created: Vec<(i32, i32)>,
    pub missing: Vec<MissingVariant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTile {
    pub slot: TerrainSlot,
    pub asset_id: String,
    pub exact: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainInfo {
    pub brush_id: String,
    pub brush_name: String,
    pub group: String,
    pub slot: TerrainSlot,
    pub automatic: bool,
    pub asset_id: String,
    pub missing_exact: bool,
    pub blueprint_id: Option<String>,
}

/// Grid square under a Courses board pointer. Truncates toward zero, matching
/// `(local / 20) as i32` in the editor. Negative values are outside the course.
pub fn pointer_cell(local_x: f32, local_y: f32) -> (i32, i32) {
    (
        (local_x / EDITOR_CELL_PX) as i32,
        (local_y / EDITOR_CELL_PX) as i32,
    )
}

pub fn slot_label(slot: TerrainSlot) -> &'static str {
    match slot {
        TerrainSlot::Isolated => "Isolated tile",
        TerrainSlot::HorizontalLeft => "Left end",
        TerrainSlot::HorizontalMiddle => "Middle",
        TerrainSlot::HorizontalRight => "Right end",
        TerrainSlot::VerticalTop => "Top end",
        TerrainSlot::VerticalMiddle => "Vertical middle",
        TerrainSlot::VerticalBottom => "Bottom end",
        TerrainSlot::TopEdge => "Top edge",
        TerrainSlot::BottomEdge => "Bottom edge",
        TerrainSlot::LeftEdge => "Left edge",
        TerrainSlot::RightEdge => "Right edge",
        TerrainSlot::TopLeftOuter => "Top-left corner",
        TerrainSlot::TopRightOuter => "Top-right corner",
        TerrainSlot::BottomLeftOuter => "Bottom-left corner",
        TerrainSlot::BottomRightOuter => "Bottom-right corner",
        TerrainSlot::Interior => "Fill",
        TerrainSlot::InnerTopLeft => "Inner corner, top-left",
        TerrainSlot::InnerTopRight => "Inner corner, top-right",
        TerrainSlot::InnerBottomLeft => "Inner corner, bottom-left",
        TerrainSlot::InnerBottomRight => "Inner corner, bottom-right",
    }
}

pub fn slot_key(slot: TerrainSlot) -> &'static str {
    match slot {
        TerrainSlot::Isolated => "isolated",
        TerrainSlot::HorizontalLeft => "horizontal_left",
        TerrainSlot::HorizontalMiddle => "horizontal_middle",
        TerrainSlot::HorizontalRight => "horizontal_right",
        TerrainSlot::VerticalTop => "vertical_top",
        TerrainSlot::VerticalMiddle => "vertical_middle",
        TerrainSlot::VerticalBottom => "vertical_bottom",
        TerrainSlot::TopEdge => "top_edge",
        TerrainSlot::BottomEdge => "bottom_edge",
        TerrainSlot::LeftEdge => "left_edge",
        TerrainSlot::RightEdge => "right_edge",
        TerrainSlot::TopLeftOuter => "top_left_outer",
        TerrainSlot::TopRightOuter => "top_right_outer",
        TerrainSlot::BottomLeftOuter => "bottom_left_outer",
        TerrainSlot::BottomRightOuter => "bottom_right_outer",
        TerrainSlot::Interior => "interior",
        TerrainSlot::InnerTopLeft => "inner_top_left",
        TerrainSlot::InnerTopRight => "inner_top_right",
        TerrainSlot::InnerBottomLeft => "inner_bottom_left",
        TerrainSlot::InnerBottomRight => "inner_bottom_right",
    }
}

pub fn tool_label(tool: TerrainTool) -> &'static str {
    match tool {
        TerrainTool::Single => "Single cell",
        TerrainTool::Freehand => "Freehand drag",
        TerrainTool::Horizontal => "Horizontal line",
        TerrainTool::Vertical => "Vertical line",
        TerrainTool::Rectangle => "Rectangle",
        TerrainTool::Erase => "Erase",
        TerrainTool::Inspect => "Select",
    }
}

pub fn connection_group(brush: &TerrainBrush) -> &str {
    let group = brush.group.trim();
    if group.is_empty() {
        brush.id.as_str()
    } else {
        group
    }
}

pub fn missing_slots(brush: &TerrainBrush) -> Vec<TerrainSlot> {
    ALL_SLOTS
        .into_iter()
        .filter(|slot| {
            brush
                .variants
                .get(slot)
                .map(|asset| asset.is_empty())
                .unwrap_or(true)
        })
        .collect()
}

/// Cells a tool covers before the course bounds are applied.
///
/// Freehand and erase use `samples`, including samples that are outside the
/// course, so a stroke can leave and come back. The other tools use `anchor`
/// and `current`. Horizontal keeps the anchor row. Vertical keeps the anchor
/// column.
pub fn paint_cells(
    tool: TerrainTool,
    anchor: (i32, i32),
    current: (i32, i32),
    samples: &[(i32, i32)],
) -> Vec<(i32, i32)> {
    match tool {
        TerrainTool::Single | TerrainTool::Inspect => vec![anchor],
        TerrainTool::Freehand | TerrainTool::Erase => freehand_cells(samples),
        TerrainTool::Horizontal => {
            let (x0, x1) = ordered(anchor.0, current.0);
            (x0..=x1).map(|x| (x, anchor.1)).collect()
        }
        TerrainTool::Vertical => {
            let (y0, y1) = ordered(anchor.1, current.1);
            (y0..=y1).map(|y| (anchor.0, y)).collect()
        }
        TerrainTool::Rectangle => {
            let (x0, x1) = ordered(anchor.0, current.0);
            let (y0, y1) = ordered(anchor.1, current.1);
            let mut cells = Vec::new();
            for y in y0..=y1 {
                for x in x0..=x1 {
                    cells.push((x, y));
                }
            }
            cells
        }
    }
}

pub fn retain_inside(cells: Vec<(i32, i32)>, width: u32, height: u32) -> Vec<(i32, i32)> {
    cells
        .into_iter()
        .filter(|(x, y)| inside_size(*x, *y, width, height))
        .collect()
}

/// Grid line that always steps to a cell sharing a side.
///
/// When the pointer crosses a diagonal, the orthogonal cell on the horizontal
/// step is inserted before the diagonal destination. That is the coverage
/// policy for fast drags. The result includes both ends and has no duplicates.
pub fn grid_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut cells = vec![(x0, y0)];
    let dx = i64::from((x1 - x0).abs());
    let dy = i64::from((y1 - y0).abs());
    let sx = (x1 - x0).signum();
    let sy = (y1 - y0).signum();
    if dx == 0 && dy == 0 {
        return cells;
    }
    let mut x = x0;
    let mut y = y0;
    let mut err = dx - dy;
    while x != x1 || y != y1 {
        let doubled = err * 2;
        let step_x = doubled > -dy;
        let step_y = doubled < dx;
        if step_x {
            err -= dy;
            x += sx;
        }
        if step_x && step_y {
            cells.push((x, y));
        }
        if step_y {
            err += dx;
            y += sy;
        }
        let next = (x, y);
        if cells.last().copied() != Some(next) {
            cells.push(next);
        }
    }
    cells
}

pub fn freehand_cells(samples: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let Some((first, rest)) = samples.split_first() else {
        return Vec::new();
    };
    let mut cells = vec![*first];
    let mut previous = *first;
    for sample in rest {
        if *sample == previous {
            continue;
        }
        for cell in grid_line(previous.0, previous.1, sample.0, sample.1)
            .into_iter()
            .skip(1)
        {
            cells.push(cell);
        }
        previous = *sample;
    }
    cells
}

pub fn apply_stroke(
    level: &mut Level,
    brushes: &[TerrainBrush],
    brush_id: &str,
    coords: &[(i32, i32)],
    erase: bool,
    new_blueprint: Option<&str>,
) -> Result<StrokeReport, String> {
    if !erase {
        let brush = brushes
            .iter()
            .find(|brush| brush.id == brush_id)
            .ok_or_else(|| format!("No terrain brush {brush_id}."))?;
        if resolve_tile(brush, TerrainSlot::Isolated).is_none() {
            return Err(
                "This terrain brush has no tile graphic. Assign a picture before painting.".into(),
            );
        }
    }
    let mut unique = Vec::new();
    let mut seen = BTreeSet::new();
    for &(x, y) in coords {
        if inside_level(level, x, y) && seen.insert((x, y)) {
            unique.push((x, y));
        }
    }
    let mut dirty = BTreeSet::new();
    for &(x, y) in &unique {
        mark_neighborhood(&mut dirty, x, y);
    }
    let mut created = Vec::new();
    let mut painted = 0;
    let mut erased = 0;
    if erase {
        for &(x, y) in &unique {
            if remove_terrain(level, x, y) {
                erased += 1;
            }
        }
    } else {
        for &(x, y) in &unique {
            if let Some(index) = terrain_index(level, x, y) {
                let cell = &mut level.cells[index];
                if let Some(terrain) = cell.terrain.as_mut() {
                    terrain.brush_id = brush_id.to_string();
                    terrain.manual = false;
                    terrain.override_asset = None;
                }
                painted += 1;
            } else {
                level.cells.push(Cell {
                    x,
                    y,
                    asset_id: String::new(),
                    layer: Layer::Main,
                    blueprint_id: new_blueprint.map(str::to_string),
                    terrain: Some(TerrainCell {
                        brush_id: brush_id.to_string(),
                        manual: false,
                        override_asset: None,
                    }),
                    ..Cell::default()
                });
                created.push((x, y));
                painted += 1;
            }
        }
    }
    let missing = recompute_many(level, brushes, &dirty);
    Ok(StrokeReport {
        painted,
        erased,
        created,
        missing,
    })
}

pub fn set_manual_override(
    level: &mut Level,
    x: i32,
    y: i32,
    asset_id: &str,
) -> Result<(), String> {
    let index =
        terrain_index(level, x, y).ok_or_else(|| format!("No terrain at {x},{y} to override."))?;
    let cell = &mut level.cells[index];
    let terrain = cell
        .terrain
        .as_mut()
        .ok_or_else(|| format!("No terrain at {x},{y} to override."))?;
    terrain.manual = true;
    terrain.override_asset = Some(asset_id.to_string());
    cell.asset_id = asset_id.to_string();
    Ok(())
}

pub fn clear_manual_override(
    level: &mut Level,
    brushes: &[TerrainBrush],
    x: i32,
    y: i32,
) -> Result<ResolvedTile, String> {
    let index = terrain_index(level, x, y).ok_or_else(|| format!("No terrain at {x},{y}."))?;
    {
        let terrain = level.cells[index]
            .terrain
            .as_mut()
            .ok_or_else(|| format!("No terrain at {x},{y}."))?;
        terrain.manual = false;
        terrain.override_asset = None;
    }
    recompute_cell(level, brushes, x, y)
        .ok_or_else(|| format!("Terrain at {x},{y} has no graphic to restore."))
}

pub fn refresh_brush_art(
    level: &mut Level,
    brushes: &[TerrainBrush],
    brush_id: &str,
) -> Vec<MissingVariant> {
    let coords: Vec<(i32, i32)> = level
        .cells
        .iter()
        .filter(|cell| {
            cell.terrain
                .as_ref()
                .is_some_and(|terrain| terrain.brush_id == brush_id && !terrain.manual)
        })
        .map(|cell| (cell.x, cell.y))
        .collect();
    let mut missing = Vec::new();
    for (x, y) in coords {
        if let Some(tile) = recompute_cell(level, brushes, x, y) {
            if !tile.exact {
                missing.push(MissingVariant {
                    x,
                    y,
                    slot: tile.slot,
                    asset_id: tile.asset_id,
                });
            }
        }
    }
    missing
}

pub fn refresh_connections(level: &mut Level, brushes: &[TerrainBrush]) -> Vec<MissingVariant> {
    let coords: Vec<(i32, i32)> = level
        .cells
        .iter()
        .filter(|cell| cell.terrain.as_ref().is_some_and(|terrain| !terrain.manual))
        .map(|cell| (cell.x, cell.y))
        .collect();
    let mut missing = Vec::new();
    for (x, y) in coords {
        if let Some(tile) = recompute_cell(level, brushes, x, y) {
            if !tile.exact {
                missing.push(MissingVariant {
                    x,
                    y,
                    slot: tile.slot,
                    asset_id: tile.asset_id,
                });
            }
        }
    }
    missing
}

pub fn inspect_terrain(
    level: &Level,
    brushes: &[TerrainBrush],
    x: i32,
    y: i32,
) -> Option<TerrainInfo> {
    let cell = terrain_ref(level, x, y)?;
    let terrain = cell.terrain.as_ref()?;
    let brush = brushes.iter().find(|brush| brush.id == terrain.brush_id)?;
    let (cardinal, inner) = neighborhood(level, brushes, x, y);
    let slot = slot_for(brush.mode, cardinal, inner);
    let exact = resolve_tile(brush, slot).is_some_and(|tile| tile.exact);
    Some(TerrainInfo {
        brush_id: brush.id.clone(),
        brush_name: brush.name.clone(),
        group: connection_group(brush).to_string(),
        slot,
        automatic: !terrain.manual,
        asset_id: cell.asset_id.clone(),
        missing_exact: !exact,
        blueprint_id: cell.blueprint_id.clone(),
    })
}

pub fn commit_edit(undo: &mut Vec<TerrainEdit>, redo: &mut Vec<TerrainEdit>, edit: TerrainEdit) {
    if edit.before == edit.after {
        return;
    }
    undo.push(edit);
    if undo.len() > HISTORY_LIMIT {
        let extra = undo.len() - HISTORY_LIMIT;
        undo.drain(0..extra);
    }
    redo.clear();
}

pub fn undo_edit(
    undo: &mut Vec<TerrainEdit>,
    redo: &mut Vec<TerrainEdit>,
    levels: &mut [Level],
) -> bool {
    let Some(edit) = undo.pop() else {
        return false;
    };
    let Some(level) = levels.get_mut(edit.level_index) else {
        return false;
    };
    level.cells = edit.before.clone();
    redo.push(edit);
    true
}

pub fn redo_edit(
    undo: &mut Vec<TerrainEdit>,
    redo: &mut Vec<TerrainEdit>,
    levels: &mut [Level],
) -> bool {
    let Some(edit) = redo.pop() else {
        return false;
    };
    let Some(level) = levels.get_mut(edit.level_index) else {
        return false;
    };
    level.cells = edit.after.clone();
    undo.push(edit);
    true
}

fn ordered(a: i32, b: i32) -> (i32, i32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn inside_size(x: i32, y: i32, width: u32, height: u32) -> bool {
    x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height
}

fn inside_level(level: &Level, x: i32, y: i32) -> bool {
    inside_size(x, y, level.w, level.h)
}

fn terrain_index(level: &Level, x: i32, y: i32) -> Option<usize> {
    level
        .cells
        .iter()
        .position(|cell| cell.x == x && cell.y == y && cell.terrain.is_some())
}

fn terrain_ref(level: &Level, x: i32, y: i32) -> Option<&Cell> {
    terrain_index(level, x, y).map(|index| &level.cells[index])
}

fn remove_terrain(level: &mut Level, x: i32, y: i32) -> bool {
    let Some(index) = terrain_index(level, x, y) else {
        return false;
    };
    level.cells.remove(index);
    true
}

fn mark_neighborhood(dirty: &mut BTreeSet<(i32, i32)>, x: i32, y: i32) {
    for dy in -1..=1 {
        for dx in -1..=1 {
            dirty.insert((x + dx, y + dy));
        }
    }
}

fn same_group(brushes: &[TerrainBrush], left: &TerrainCell, right: &TerrainCell) -> bool {
    let Some(left_brush) = brushes.iter().find(|brush| brush.id == left.brush_id) else {
        return false;
    };
    let Some(right_brush) = brushes.iter().find(|brush| brush.id == right.brush_id) else {
        return false;
    };
    connection_group(left_brush) == connection_group(right_brush)
}

fn connected(level: &Level, brushes: &[TerrainBrush], x: i32, y: i32, nx: i32, ny: i32) -> bool {
    let Some(here) = terrain_ref(level, x, y).and_then(|cell| cell.terrain.as_ref()) else {
        return false;
    };
    let Some(there) = terrain_ref(level, nx, ny).and_then(|cell| cell.terrain.as_ref()) else {
        return false;
    };
    same_group(brushes, here, there)
}

fn neighborhood(level: &Level, brushes: &[TerrainBrush], x: i32, y: i32) -> (u8, u8) {
    let sides = [(0, -1, NORTH), (1, 0, EAST), (0, 1, SOUTH), (-1, 0, WEST)];
    let mut cardinal = 0u8;
    let mut present = [false; 4];
    for (index, (dx, dy, bit)) in sides.iter().copied().enumerate() {
        if connected(level, brushes, x, y, x + dx, y + dy) {
            cardinal |= bit;
            present[index] = true;
        }
    }
    let diagonals = [
        (-1, -1, INNER_NORTH_WEST, 0, 3),
        (1, -1, INNER_NORTH_EAST, 0, 1),
        (-1, 1, INNER_SOUTH_WEST, 2, 3),
        (1, 1, INNER_SOUTH_EAST, 2, 1),
    ];
    let mut inner = 0u8;
    for (dx, dy, bit, side_a, side_b) in diagonals {
        if present[side_a] && present[side_b] && !connected(level, brushes, x, y, x + dx, y + dy) {
            inner |= bit;
        }
    }
    (cardinal, inner)
}

fn slot_for(mode: NeighborMode, cardinal: u8, inner: u8) -> TerrainSlot {
    if mode == NeighborMode::Eight {
        if inner & INNER_NORTH_WEST != 0 {
            return TerrainSlot::InnerTopLeft;
        }
        if inner & INNER_NORTH_EAST != 0 {
            return TerrainSlot::InnerTopRight;
        }
        if inner & INNER_SOUTH_WEST != 0 {
            return TerrainSlot::InnerBottomLeft;
        }
        if inner & INNER_SOUTH_EAST != 0 {
            return TerrainSlot::InnerBottomRight;
        }
    }
    CARDINAL_SLOTS[usize::from(cardinal & 15)]
}

fn mapped_asset(brush: &TerrainBrush, slot: TerrainSlot) -> Option<&str> {
    brush
        .variants
        .get(&slot)
        .map(String::as_str)
        .filter(|asset| !asset.is_empty())
}

fn resolve_tile(brush: &TerrainBrush, slot: TerrainSlot) -> Option<ResolvedTile> {
    if let Some(asset_id) = mapped_asset(brush, slot) {
        return Some(ResolvedTile {
            slot,
            asset_id: asset_id.to_string(),
            exact: true,
        });
    }
    if !brush.fallback_asset.is_empty() {
        return Some(ResolvedTile {
            slot,
            asset_id: brush.fallback_asset.clone(),
            exact: false,
        });
    }
    for candidate in FALLBACK_ORDER {
        if let Some(asset_id) = mapped_asset(brush, candidate) {
            return Some(ResolvedTile {
                slot,
                asset_id: asset_id.to_string(),
                exact: false,
            });
        }
    }
    None
}

fn recompute_cell(
    level: &mut Level,
    brushes: &[TerrainBrush],
    x: i32,
    y: i32,
) -> Option<ResolvedTile> {
    let index = terrain_index(level, x, y)?;
    let manual = level.cells[index]
        .terrain
        .as_ref()
        .is_some_and(|terrain| terrain.manual);
    if manual {
        if let Some(asset) = level.cells[index]
            .terrain
            .as_ref()
            .and_then(|terrain| terrain.override_asset.clone())
        {
            level.cells[index].asset_id = asset;
        }
        return None;
    }
    let brush_id = level.cells[index].terrain.as_ref()?.brush_id.clone();
    let brush = brushes.iter().find(|brush| brush.id == brush_id)?;
    let (cardinal, inner) = neighborhood(level, brushes, x, y);
    let slot = slot_for(brush.mode, cardinal, inner);
    let tile = resolve_tile(brush, slot)?;
    level.cells[index].asset_id = tile.asset_id.clone();
    Some(tile)
}

fn recompute_many(
    level: &mut Level,
    brushes: &[TerrainBrush],
    dirty: &BTreeSet<(i32, i32)>,
) -> Vec<MissingVariant> {
    let mut missing = Vec::new();
    for &(x, y) in dirty {
        let Some(tile) = recompute_cell(level, brushes, x, y) else {
            continue;
        };
        if !tile.exact {
            missing.push(MissingVariant {
                x,
                y,
                slot: tile.slot,
                asset_id: tile.asset_id,
            });
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint::{assign_cell, compose_fixture, LegacyBlock};
    use crate::enemy_ai::EnemyAiStyle;
    use crate::pngutil::b64_encode;
    use crate::project::{Asset, AssetKind, BlockTag, HazardTag, ItemTag, Project, Spawn};
    use std::time::Instant;

    fn brush(id: &str, group: &str, mode: NeighborMode) -> TerrainBrush {
        let mut variants = BTreeMap::new();
        for slot in ALL_SLOTS {
            variants.insert(slot, format!("{id}-{}", slot_key(slot)));
        }
        TerrainBrush {
            id: id.into(),
            name: id.into(),
            group: group.into(),
            mode,
            variants,
            fallback_asset: String::new(),
            default_role: LegacyRole::Inert,
            notes: String::new(),
        }
    }

    fn course(w: u32, h: u32) -> Level {
        Level {
            id: "course".into(),
            name: "Course".into(),
            style_id: String::new(),
            w,
            h,
            sky: "#000".into(),
            music_id: None,
            player: Spawn { x: 2.0, y: 2.0 },
            cells: Vec::new(),
            enemy_ai: EnemyAiStyle::Smb1Nes,
            entrances: Vec::new(),
        }
    }

    fn paint(level: &mut Level, material: &TerrainBrush, coords: &[(i32, i32)]) {
        apply_stroke(
            level,
            std::slice::from_ref(material),
            &material.id,
            coords,
            false,
            Some("bp-solid"),
        )
        .unwrap();
    }

    fn art(level: &Level, x: i32, y: i32) -> String {
        level
            .cells
            .iter()
            .find(|cell| cell.x == x && cell.y == y && cell.terrain.is_some())
            .unwrap_or_else(|| panic!("no terrain at {x},{y}"))
            .asset_id
            .clone()
    }

    fn blueprint_of(level: &Level, x: i32, y: i32) -> Option<String> {
        level
            .cells
            .iter()
            .find(|cell| cell.x == x && cell.y == y && cell.terrain.is_some())
            .and_then(|cell| cell.blueprint_id.clone())
    }

    fn row(x0: i32, x1: i32, y: i32) -> Vec<(i32, i32)> {
        (x0..=x1).map(|x| (x, y)).collect()
    }

    fn expect(id: &str, slot: TerrainSlot) -> String {
        format!("{id}-{}", slot_key(slot))
    }

    #[test]
    fn four_neighbor_masks_match_the_documented_table() {
        let cases = [
            (0, TerrainSlot::Isolated),
            (NORTH, TerrainSlot::VerticalBottom),
            (EAST, TerrainSlot::HorizontalLeft),
            (NORTH | EAST, TerrainSlot::BottomLeftOuter),
            (SOUTH, TerrainSlot::VerticalTop),
            (NORTH | SOUTH, TerrainSlot::VerticalMiddle),
            (EAST | SOUTH, TerrainSlot::TopLeftOuter),
            (NORTH | EAST | SOUTH, TerrainSlot::LeftEdge),
            (WEST, TerrainSlot::HorizontalRight),
            (NORTH | WEST, TerrainSlot::BottomRightOuter),
            (EAST | WEST, TerrainSlot::HorizontalMiddle),
            (NORTH | EAST | WEST, TerrainSlot::BottomEdge),
            (SOUTH | WEST, TerrainSlot::TopRightOuter),
            (NORTH | SOUTH | WEST, TerrainSlot::RightEdge),
            (EAST | SOUTH | WEST, TerrainSlot::TopEdge),
            (NORTH | EAST | SOUTH | WEST, TerrainSlot::Interior),
        ];
        for (mask, slot) in cases {
            assert_eq!(slot_for(NeighborMode::Four, mask, 0), slot, "mask {mask}");
        }
        assert_eq!(
            slot_for(NeighborMode::Eight, 15, INNER_NORTH_WEST),
            TerrainSlot::InnerTopLeft
        );
        assert_eq!(
            slot_for(NeighborMode::Four, 15, INNER_NORTH_WEST),
            TerrainSlot::Interior
        );
        assert_eq!(
            slot_for(NeighborMode::Eight, 15, INNER_NORTH_WEST | INNER_NORTH_EAST),
            TerrainSlot::InnerTopLeft
        );
    }

    #[test]
    fn horizontal_line_resolves_left_middle_and_right() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(32, 15);
        paint(&mut level, &material, &row(5, 12, 10));
        assert_eq!(
            art(&level, 5, 10),
            expect("grass", TerrainSlot::HorizontalLeft)
        );
        for x in 6..12 {
            assert_eq!(
                art(&level, x, 10),
                expect("grass", TerrainSlot::HorizontalMiddle),
                "cell {x}"
            );
        }
        assert_eq!(
            art(&level, 12, 10),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        assert_eq!(
            level
                .cells
                .iter()
                .filter(|cell| cell.terrain.is_some())
                .count(),
            8
        );
    }

    #[test]
    fn extending_a_platform_turns_the_old_right_end_into_a_middle() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(32, 15);
        paint(&mut level, &material, &row(0, 3, 4));
        assert_eq!(
            art(&level, 3, 4),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        let kept = blueprint_of(&level, 3, 4);
        paint(&mut level, &material, &[(4, 4)]);
        assert_eq!(
            art(&level, 3, 4),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        assert_eq!(
            art(&level, 4, 4),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        assert_eq!(blueprint_of(&level, 3, 4), kept);
        assert_eq!(blueprint_of(&level, 0, 4), Some("bp-solid".into()));
    }

    #[test]
    fn erasing_a_middle_cell_exposes_new_endpieces() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(32, 15);
        paint(&mut level, &material, &row(0, 7, 2));
        apply_stroke(
            &mut level,
            &[material.clone()],
            "grass",
            &[(3, 2)],
            true,
            None,
        )
        .unwrap();
        assert_eq!(
            art(&level, 0, 2),
            expect("grass", TerrainSlot::HorizontalLeft)
        );
        assert_eq!(
            art(&level, 1, 2),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        assert_eq!(
            art(&level, 2, 2),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        assert!(terrain_index(&level, 3, 2).is_none());
        assert_eq!(
            art(&level, 4, 2),
            expect("grass", TerrainSlot::HorizontalLeft)
        );
        assert_eq!(
            art(&level, 5, 2),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        assert_eq!(
            art(&level, 6, 2),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        assert_eq!(
            art(&level, 7, 2),
            expect("grass", TerrainSlot::HorizontalRight)
        );
    }

    #[test]
    fn painting_right_to_left_matches_left_to_right() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut forward = course(32, 15);
        paint(&mut forward, &material, &row(0, 7, 1));
        let mut backward = course(32, 15);
        let reversed: Vec<_> = (0..=7).rev().map(|x| (x, 1)).collect();
        paint(&mut backward, &material, &reversed);
        assert_eq!(semantic(&forward), semantic(&backward));
    }

    #[test]
    fn vertical_line_resolves_top_middle_and_bottom() {
        let material = brush("stone", "stone", NeighborMode::Four);
        let mut level = course(16, 20);
        let coords: Vec<_> = (2..=6).map(|y| (4, y)).collect();
        paint(&mut level, &material, &coords);
        assert_eq!(art(&level, 4, 2), expect("stone", TerrainSlot::VerticalTop));
        for y in 3..6 {
            assert_eq!(
                art(&level, 4, y),
                expect("stone", TerrainSlot::VerticalMiddle)
            );
        }
        assert_eq!(
            art(&level, 4, 6),
            expect("stone", TerrainSlot::VerticalBottom)
        );
    }

    #[test]
    fn rectangle_fill_resolves_corners_edges_and_interior() {
        let material = brush("dirt", "dirt", NeighborMode::Four);
        let mut level = course(16, 16);
        let mut coords = Vec::new();
        for y in 1..=3 {
            for x in 2..=4 {
                coords.push((x, y));
            }
        }
        paint(&mut level, &material, &coords);
        assert_eq!(art(&level, 2, 1), expect("dirt", TerrainSlot::TopLeftOuter));
        assert_eq!(art(&level, 3, 1), expect("dirt", TerrainSlot::TopEdge));
        assert_eq!(
            art(&level, 4, 1),
            expect("dirt", TerrainSlot::TopRightOuter)
        );
        assert_eq!(art(&level, 2, 2), expect("dirt", TerrainSlot::LeftEdge));
        assert_eq!(art(&level, 3, 2), expect("dirt", TerrainSlot::Interior));
        assert_eq!(art(&level, 4, 2), expect("dirt", TerrainSlot::RightEdge));
        assert_eq!(
            art(&level, 2, 3),
            expect("dirt", TerrainSlot::BottomLeftOuter)
        );
        assert_eq!(art(&level, 3, 3), expect("dirt", TerrainSlot::BottomEdge));
        assert_eq!(
            art(&level, 4, 3),
            expect("dirt", TerrainSlot::BottomRightOuter)
        );
    }

    #[test]
    fn eight_neighbor_hole_resolves_an_inner_corner() {
        let material = brush("brick", "brick", NeighborMode::Eight);
        let mut level = course(12, 12);
        let mut coords = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                coords.push((x, y));
            }
        }
        paint(&mut level, &material, &coords);
        apply_stroke(
            &mut level,
            &[material.clone()],
            "brick",
            &[(1, 1)],
            true,
            None,
        )
        .unwrap();
        assert_eq!(
            art(&level, 2, 2),
            expect("brick", TerrainSlot::InnerTopLeft)
        );
        let four = brush("brick", "brick", NeighborMode::Four);
        let mut flat = course(12, 12);
        paint(&mut flat, &four, &coords);
        apply_stroke(&mut flat, &[four], "brick", &[(1, 1)], true, None).unwrap();
        assert_eq!(art(&flat, 2, 2), expect("brick", TerrainSlot::Interior));
    }

    #[test]
    fn a_lone_diagonal_does_not_connect() {
        let material = brush("sand", "sand", NeighborMode::Eight);
        let mut level = course(8, 8);
        paint(&mut level, &material, &[(1, 1), (2, 2)]);
        assert_eq!(art(&level, 1, 1), expect("sand", TerrainSlot::Isolated));
        assert_eq!(art(&level, 2, 2), expect("sand", TerrainSlot::Isolated));
    }

    #[test]
    fn different_connection_groups_do_not_join() {
        let grass = brush("grass", "grass", NeighborMode::Four);
        let stone = brush("stone", "stone", NeighborMode::Four);
        let mut shared = stone.clone();
        for slot in ALL_SLOTS {
            shared
                .variants
                .insert(slot, format!("grass-{}", slot_key(slot)));
        }
        let mut level = course(16, 8);
        apply_stroke(
            &mut level,
            &[grass.clone(), shared.clone()],
            "grass",
            &row(0, 2, 3),
            false,
            Some("bp-grass"),
        )
        .unwrap();
        apply_stroke(
            &mut level,
            &[grass.clone(), shared.clone()],
            "stone",
            &row(3, 5, 3),
            false,
            Some("bp-stone"),
        )
        .unwrap();
        assert_eq!(
            art(&level, 2, 3),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        assert_eq!(
            art(&level, 3, 3),
            expect("grass", TerrainSlot::HorizontalLeft)
        );
        assert_eq!(blueprint_of(&level, 2, 3), Some("bp-grass".into()));
        assert_eq!(blueprint_of(&level, 3, 3), Some("bp-stone".into()));
        let mut joined = course(16, 8);
        let mut other = grass.clone();
        other.id = "grass-b".into();
        other.name = "Grass copy".into();
        apply_stroke(
            &mut joined,
            &[grass.clone(), other.clone()],
            "grass",
            &row(0, 1, 1),
            false,
            Some("bp-solid"),
        )
        .unwrap();
        apply_stroke(
            &mut joined,
            &[grass, other],
            "grass-b",
            &row(2, 3, 1),
            false,
            Some("bp-solid"),
        )
        .unwrap();
        assert_eq!(
            art(&joined, 1, 1),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        assert_eq!(
            art(&joined, 2, 1),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
    }

    #[test]
    fn filling_the_gap_matches_painting_the_whole_row() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut split = course(24, 8);
        paint(&mut split, &material, &row(0, 2, 5));
        paint(&mut split, &material, &row(5, 7, 5));
        paint(&mut split, &material, &row(3, 4, 5));
        let mut whole = course(24, 8);
        paint(&mut whole, &material, &row(0, 7, 5));
        assert_eq!(semantic(&split), semantic(&whole));
    }

    #[test]
    fn a_single_cell_uses_the_isolated_variant() {
        let material = brush("snow", "snow", NeighborMode::Four);
        let mut level = course(8, 8);
        paint(&mut level, &material, &[(3, 3)]);
        assert_eq!(art(&level, 3, 3), expect("snow", TerrainSlot::Isolated));
    }

    #[test]
    fn a_missing_variant_uses_the_fallback_and_reports_it() {
        let mut material = TerrainBrush {
            id: "plain".into(),
            name: "Plain".into(),
            group: "plain".into(),
            mode: NeighborMode::Four,
            variants: BTreeMap::new(),
            fallback_asset: "plain-tile".into(),
            default_role: LegacyRole::Inert,
            notes: String::new(),
        };
        material
            .variants
            .insert(TerrainSlot::Isolated, String::new());
        let mut level = course(16, 8);
        let report = apply_stroke(
            &mut level,
            &[material.clone()],
            "plain",
            &row(0, 3, 1),
            false,
            Some("bp-solid"),
        )
        .unwrap();
        assert!(report.missing.len() >= 3, "{:?}", report.missing);
        assert!(report
            .missing
            .iter()
            .any(|item| item.slot == TerrainSlot::HorizontalLeft && item.x == 0));
        assert!(report
            .missing
            .iter()
            .all(|item| item.asset_id == "plain-tile"));
        for x in 0..=3 {
            assert_eq!(art(&level, x, 1), "plain-tile");
        }
        assert!(missing_slots(&material).contains(&TerrainSlot::HorizontalLeft));
        let err = apply_stroke(
            &mut level,
            &[TerrainBrush {
                id: "empty".into(),
                name: "Empty".into(),
                ..TerrainBrush::default()
            }],
            "empty",
            &[(1, 1)],
            false,
            None,
        )
        .unwrap_err();
        assert!(err.contains("no tile graphic"));
    }

    #[test]
    fn a_manual_override_survives_neighbor_edits() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(16, 8);
        paint(&mut level, &material, &row(0, 3, 2));
        set_manual_override(&mut level, 1, 2, "cracked").unwrap();
        paint(&mut level, &material, &[(4, 2)]);
        assert_eq!(art(&level, 1, 2), "cracked");
        assert!(level.cells.iter().any(|cell| {
            cell.x == 1
                && cell.y == 2
                && cell
                    .terrain
                    .as_ref()
                    .is_some_and(|terrain| terrain.manual && terrain.brush_id == "grass")
        }));
        assert_eq!(blueprint_of(&level, 1, 2), Some("bp-solid".into()));
        assert_eq!(
            art(&level, 3, 2),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        paint(&mut level, &material, &[(1, 2)]);
        assert_eq!(
            art(&level, 1, 2),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
    }

    #[test]
    fn clearing_a_manual_override_restores_the_automatic_tile() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(16, 8);
        paint(&mut level, &material, &row(0, 4, 2));
        set_manual_override(&mut level, 2, 2, "cracked").unwrap();
        let restored = clear_manual_override(&mut level, &[material], 2, 2).unwrap();
        assert!(restored.exact);
        assert_eq!(restored.slot, TerrainSlot::HorizontalMiddle);
        assert_eq!(
            art(&level, 2, 2),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        let info =
            inspect_terrain(&level, &[brush("grass", "grass", NeighborMode::Four)], 2, 2).unwrap();
        assert!(info.automatic);
        assert!(!info.missing_exact);
    }

    #[test]
    fn one_undo_and_redo_restore_the_whole_stroke() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(32, 15);
        let before = level.cells.clone();
        paint(&mut level, &material, &row(0, 19, 6));
        let after = level.cells.clone();
        let mut undo = Vec::new();
        let mut redo = Vec::new();
        commit_edit(
            &mut undo,
            &mut redo,
            TerrainEdit {
                level_index: 0,
                before,
                after,
            },
        );
        assert_eq!(undo.len(), 1);
        let mut levels = [level];
        assert!(undo_edit(&mut undo, &mut redo, &mut levels));
        assert!(levels[0].cells.is_empty());
        assert!(redo_edit(&mut undo, &mut redo, &mut levels));
        assert_eq!(
            art(&levels[0], 0, 6),
            expect("grass", TerrainSlot::HorizontalLeft)
        );
        assert_eq!(
            art(&levels[0], 19, 6),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        assert_eq!(
            art(&levels[0], 10, 6),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        for _ in 0..40 {
            commit_edit(
                &mut undo,
                &mut redo,
                TerrainEdit {
                    level_index: 0,
                    before: vec![Cell {
                        x: 0,
                        y: 0,
                        asset_id: "a".into(),
                        ..Cell::default()
                    }],
                    after: vec![Cell {
                        x: 1,
                        y: 0,
                        asset_id: "b".into(),
                        ..Cell::default()
                    }],
                },
            );
        }
        assert_eq!(undo.len(), HISTORY_LIMIT);
    }

    #[test]
    fn save_and_load_keeps_brushes_tiles_gameplay_and_overrides() {
        let grass = brush("grass", "meadow", NeighborMode::Eight);
        let stone = brush("stone", "wall", NeighborMode::Four);
        let mut level = course(16, 8);
        apply_stroke(
            &mut level,
            &[grass.clone(), stone.clone()],
            "grass",
            &row(0, 3, 4),
            false,
            Some("bp-solid"),
        )
        .unwrap();
        set_manual_override(&mut level, 1, 4, "cracked").unwrap();
        level.cells.iter_mut().for_each(|cell| {
            if cell.terrain.is_some() {
                cell.transition_id = Some("exit-keep".into());
            }
        });
        let mut project = Project::blank("Terrain");
        project.terrain_brushes = vec![grass, stone];
        project.levels = vec![level];
        let text = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.terrain_brushes.len(), 2);
        assert_eq!(loaded.terrain_brushes[0].id, "grass");
        assert_eq!(loaded.terrain_brushes[0].group, "meadow");
        assert_eq!(loaded.terrain_brushes[0].mode, NeighborMode::Eight);
        assert_eq!(
            loaded.terrain_brushes[0].variants[&TerrainSlot::HorizontalLeft],
            "grass-horizontal_left"
        );
        let cell = loaded.levels[0]
            .cells
            .iter()
            .find(|cell| cell.x == 1 && cell.y == 4)
            .unwrap();
        assert_eq!(cell.asset_id, "cracked");
        assert!(cell.terrain.as_ref().unwrap().manual);
        assert_eq!(
            cell.terrain.as_ref().unwrap().override_asset.as_deref(),
            Some("cracked")
        );
        assert_eq!(cell.blueprint_id.as_deref(), Some("bp-solid"));
        assert_eq!(cell.transition_id.as_deref(), Some("exit-keep"));
        let again: Project =
            serde_json::from_str(&serde_json::to_string(&loaded).unwrap()).unwrap();
        assert_eq!(again.terrain_brushes, loaded.terrain_brushes);
        assert_eq!(semantic(&again.levels[0]), semantic(&loaded.levels[0]));
    }

    #[test]
    fn a_legacy_project_keeps_its_original_cells() {
        let json = r#"{"name":"Old","levels":[{"id":"c","name":"C","w":8,"h":8,"player":{"x":1.0,"y":1.0},"cells":[{"x":1,"y":2,"asset_id":"sod","blueprint_id":"bp-old"}]}]}"#;
        let project: Project = serde_json::from_str(json).unwrap();
        assert!(project.terrain_brushes.is_empty());
        assert!(project.levels[0].cells[0].terrain.is_none());
        assert_eq!(project.levels[0].cells[0].asset_id, "sod");
        assert_eq!(
            project.levels[0].cells[0].blueprint_id.as_deref(),
            Some("bp-old")
        );
        let text = serde_json::to_string(&project).unwrap();
        assert!(!text.contains("terrain_brushes"));
        assert!(!text.contains("\"terrain\""));
        let loaded = crate::blueprint::load_project_json(json).unwrap();
        assert!(loaded.levels[0].cells[0].terrain.is_none());
        assert_eq!(loaded.levels[0].cells[0].asset_id, "sod");
        assert!(loaded.terrain_brushes.is_empty());
    }

    #[test]
    fn auto_tile_changes_do_not_change_blueprint_ids() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(16, 8);
        paint(&mut level, &material, &row(0, 3, 1));
        let before: Vec<_> = (0..=3).map(|x| blueprint_of(&level, x, 1)).collect();
        paint(&mut level, &material, &[(4, 1)]);
        for x in 0..=3 {
            assert_eq!(blueprint_of(&level, x, 1), before[x as usize]);
        }
        assert_ne!(
            art(&level, 3, 1),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        assert_eq!(
            art(&level, 3, 1),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
    }

    #[test]
    fn painting_terrain_keeps_an_unrelated_entity() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(16, 8);
        level.cells.push(Cell {
            x: 2,
            y: 2,
            asset_id: "goomba".into(),
            blueprint_id: Some("bp-goomba".into()),
            transition_id: Some("pipe-exit".into()),
            ..Cell::default()
        });
        paint(&mut level, &material, &[(1, 2), (2, 2), (3, 2)]);
        let enemy = level
            .cells
            .iter()
            .find(|cell| cell.asset_id == "goomba")
            .unwrap();
        assert_eq!(enemy.blueprint_id.as_deref(), Some("bp-goomba"));
        assert_eq!(enemy.transition_id.as_deref(), Some("pipe-exit"));
        assert!(enemy.terrain.is_none());
        assert!(terrain_index(&level, 2, 2).is_some());
        apply_stroke(&mut level, &[material], "grass", &[(2, 2)], true, None).unwrap();
        let enemy = level
            .cells
            .iter()
            .find(|cell| cell.asset_id == "goomba")
            .unwrap();
        assert_eq!(enemy.blueprint_id.as_deref(), Some("bp-goomba"));
        assert!(terrain_index(&level, 2, 2).is_none());
        assert!(terrain_index(&level, 1, 2).is_some());
    }

    #[test]
    fn different_paint_orders_resolve_to_the_same_terrain() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut by_cells = course(16, 12);
        for y in 0..3 {
            for x in 0..4 {
                paint(&mut by_cells, &material, &[(x, y)]);
            }
        }
        let mut by_rect = course(16, 12);
        let coords: Vec<_> = (0..3).flat_map(|y| (0..4).map(move |x| (x, y))).collect();
        paint(&mut by_rect, &material, &coords);
        assert_eq!(semantic(&by_cells), semantic(&by_rect));
    }

    #[test]
    fn editor_assignment_keeps_the_blueprint_when_the_graphic_changes() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut project = Project::blank("assign");
        project.assets.push(image_asset("grass-horizontal_left"));
        project.assets.push(image_asset("grass-horizontal_middle"));
        project.assets.push(image_asset("grass-horizontal_right"));
        project.terrain_brushes.push(material.clone());
        project.levels.push(course(16, 8));
        let role = compose_fixture(Some(LegacyBlock::Solid), None, None).unwrap();
        let report = apply_stroke(
            &mut project.levels[0],
            &[material.clone()],
            "grass",
            &row(0, 2, 4),
            false,
            None,
        )
        .unwrap();
        for (x, y) in report.created {
            assign_cell(&mut project, "course", x, y, role.clone()).unwrap();
        }
        let right_before = project.levels[0]
            .cells
            .iter()
            .find(|cell| cell.x == 2 && cell.terrain.is_some())
            .unwrap()
            .blueprint_id
            .clone();
        let report = apply_stroke(
            &mut project.levels[0],
            &[material],
            "grass",
            &[(3, 4)],
            false,
            None,
        )
        .unwrap();
        for (x, y) in report.created {
            assign_cell(&mut project, "course", x, y, role.clone()).unwrap();
        }
        let right_after = project.levels[0]
            .cells
            .iter()
            .find(|cell| cell.x == 2 && cell.terrain.is_some())
            .unwrap()
            .blueprint_id
            .clone();
        assert_eq!(right_before, right_after);
        assert_eq!(
            art(&project.levels[0], 2, 4),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        assert!(right_after.is_some());
        let world = crate::engine::res_1080p::World::from_level(
            &project,
            &project.levels[0],
            crate::physics::map_character(project.character_physics),
        );
        assert!(
            world.blueprint_faults.is_empty(),
            "{:?}",
            world.blueprint_faults
        );
        assert_eq!(world.solids.len(), 4);
    }

    #[test]
    fn pointer_strokes_cover_the_grid_without_gaps_or_duplicates() {
        assert_eq!(pointer_cell(0.0, 0.0), (0, 0));
        assert_eq!(pointer_cell(20.0, 0.0), (1, 0));
        assert_eq!(pointer_cell(19.9, 39.0), (0, 1));
        assert_eq!(pointer_cell(-20.0, 10.0), (-1, 0));
        let slow: Vec<_> = (0..8)
            .map(|x| pointer_cell(x as f32 * 20.0 + 1.0, 40.0))
            .collect();
        let slow_cells = retain_inside(freehand_cells(&slow), 32, 15);
        assert_eq!(slow_cells, row(0, 7, 2));
        let fast = retain_inside(
            freehand_cells(&[pointer_cell(1.0, 40.0), pointer_cell(141.0, 40.0)]),
            32,
            15,
        );
        assert_eq!(unique(fast.clone()), row(0, 7, 2));
        let reverse = retain_inside(freehand_cells(&[(7, 1), (0, 1)]), 32, 15);
        assert_eq!(unique(reverse), row(0, 7, 1));
        let vertical = retain_inside(
            paint_cells(TerrainTool::Vertical, (3, 1), (9, 6), &[]),
            32,
            15,
        );
        assert_eq!(vertical, (1..=6).map(|y| (3, y)).collect::<Vec<_>>());
        let diagonal = grid_line(0, 0, 3, 3);
        assert!(diagonal
            .windows(2)
            .all(|pair| shares_side(pair[0], pair[1])));
        assert_eq!(*diagonal.first().unwrap(), (0, 0));
        assert_eq!(*diagonal.last().unwrap(), (3, 3));
        assert_eq!(diagonal.len(), unique(diagonal.clone()).len());
        let mut level = course(32, 15);
        let material = brush("grass", "grass", NeighborMode::Four);
        paint(&mut level, &material, &row(0, 3, 4));
        let overlap = retain_inside(freehand_cells(&[(2, 4), (8, 4)]), 32, 15);
        paint(&mut level, &material, &overlap);
        assert_eq!(
            level
                .cells
                .iter()
                .filter(|cell| cell.terrain.is_some() && cell.y == 4)
                .count(),
            9
        );
        assert_eq!(
            art(&level, 3, 4),
            expect("grass", TerrainSlot::HorizontalMiddle)
        );
        let outside = retain_inside(freehand_cells(&[(-3, 2), (4, 2), (40, 2)]), 8, 8);
        assert!(outside.iter().all(|(x, y)| inside_size(*x, *y, 8, 8)));
        assert!(!outside.iter().any(|(x, _)| *x < 0));
        let reenter = retain_inside(freehand_cells(&[(1, 1), (-2, 1), (3, 1)]), 8, 8);
        assert_eq!(unique(reenter), row(0, 3, 1));
        let repeated = freehand_cells(&[(2, 2), (2, 2), (2, 2)]);
        assert_eq!(repeated, vec![(2, 2)]);
        let jumped = freehand_cells(&[(0, 4), (20, 4)]);
        assert_eq!(jumped, row(0, 20, 4));
        let mut undo = Vec::new();
        let mut redo = Vec::new();
        let before = level.cells.clone();
        let samples: Vec<_> = (0..20).map(|x| (x, 8)).collect();
        let covered = retain_inside(freehand_cells(&samples), 32, 15);
        apply_stroke(
            &mut level,
            &[material],
            "grass",
            &covered,
            false,
            Some("bp-solid"),
        )
        .unwrap();
        commit_edit(
            &mut undo,
            &mut redo,
            TerrainEdit {
                level_index: 0,
                before,
                after: level.cells.clone(),
            },
        );
        assert_eq!(undo.len(), 1);
        assert_eq!(unique(covered).len(), 20);
        let horizontal = paint_cells(TerrainTool::Horizontal, (1, 3), (6, 9), &[]);
        assert!(horizontal.iter().all(|(_, y)| *y == 3));
        let rectangle = paint_cells(TerrainTool::Rectangle, (1, 1), (2, 3), &[]);
        assert_eq!(rectangle.len(), 6);
    }

    #[test]
    fn large_strokes_stay_within_a_practical_bound() {
        let material = brush("grass", "grass", NeighborMode::Four);
        let mut level = course(100, 10);
        let started = Instant::now();
        paint(&mut level, &material, &row(0, 99, 0));
        let stroke = started.elapsed();
        assert_eq!(
            art(&level, 0, 0),
            expect("grass", TerrainSlot::HorizontalLeft)
        );
        assert_eq!(
            art(&level, 99, 0),
            expect("grass", TerrainSlot::HorizontalRight)
        );
        assert_eq!(
            level
                .cells
                .iter()
                .filter(|cell| cell.terrain.is_some())
                .count(),
            100
        );
        let region: Vec<_> = (0..10)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .collect();
        let started = Instant::now();
        paint(&mut level, &material, &region);
        let fill = started.elapsed();
        assert_eq!(
            art(&level, 0, 0),
            expect("grass", TerrainSlot::TopLeftOuter)
        );
        assert_eq!(art(&level, 50, 5), expect("grass", TerrainSlot::Interior));
        assert_eq!(
            level
                .cells
                .iter()
                .filter(|cell| cell.terrain.is_some())
                .count(),
            1000
        );
        let started = Instant::now();
        apply_stroke(&mut level, &[material], "grass", &region, true, None).unwrap();
        let erased = started.elapsed();
        assert!(level.cells.iter().all(|cell| cell.terrain.is_none()));
        let mut filled = course(100, 10);
        let material = brush("grass", "grass", NeighborMode::Four);
        let before = filled.cells.clone();
        paint(&mut filled, &material, &region);
        let after = filled.cells.clone();
        let mut undo = Vec::new();
        let mut redo = Vec::new();
        commit_edit(
            &mut undo,
            &mut redo,
            TerrainEdit {
                level_index: 0,
                before,
                after,
            },
        );
        let mut levels = [filled];
        let started = Instant::now();
        assert!(undo_edit(&mut undo, &mut redo, &mut levels));
        let undo_time = started.elapsed();
        assert!(levels[0].cells.is_empty());
        let started = Instant::now();
        assert!(redo_edit(&mut undo, &mut redo, &mut levels));
        let redo_time = started.elapsed();
        assert_eq!(levels[0].cells.len(), 1000);
        let line = format!(
            "stroke100={stroke:?} region1000={fill:?} erase1000={erased:?} undo={undo_time:?} redo={redo_time:?}"
        );
        eprintln!("phase7 terrain perf: {line}");
        let _ = std::fs::write("/tmp/nesticles-phase7-perf.txt", &line);
        assert!(stroke.as_millis() < 500, "{line}");
        assert!(fill.as_millis() < 500, "{line}");
        assert!(erased.as_millis() < 500, "{line}");
        assert!(undo_time.as_millis() < 500, "{line}");
        assert!(redo_time.as_millis() < 500, "{line}");
    }

    fn semantic(
        level: &Level,
    ) -> Vec<(
        i32,
        i32,
        String,
        String,
        Option<String>,
        bool,
        Option<String>,
    )> {
        let mut rows: Vec<_> = level
            .cells
            .iter()
            .map(|cell| {
                let terrain = cell.terrain.clone().unwrap_or(TerrainCell {
                    brush_id: String::new(),
                    manual: false,
                    override_asset: None,
                });
                (
                    cell.x,
                    cell.y,
                    cell.asset_id.clone(),
                    terrain.brush_id,
                    cell.blueprint_id.clone(),
                    terrain.manual,
                    terrain.override_asset,
                )
            })
            .collect();
        rows.sort();
        rows
    }

    fn unique(mut cells: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
        cells.sort();
        cells.dedup();
        cells
    }

    fn shares_side(a: (i32, i32), b: (i32, i32)) -> bool {
        (a.0 - b.0).abs() + (a.1 - b.1).abs() == 1
    }

    fn image_asset(id: &str) -> Asset {
        Asset {
            id: id.into(),
            name: id.into(),
            kind: AssetKind::Terrain,
            style_id: String::new(),
            w: 1,
            h: 1,
            rgba_base64: b64_encode(&[10, 20, 30, 255]),
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
}
