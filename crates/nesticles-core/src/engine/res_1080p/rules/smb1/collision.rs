//! X then Y against 72×72 solids. Corner clip keeps an upward jump moving
//! when only the outer 13.5 px of a block's bottom corner is touched.

use super::super::super::{overlaps, Rect, CORNER_FP, FP, NUDGE_FP, TILE_FP};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Solid,
    Question,
    Brick,
    Coin,
}

#[derive(Clone, Debug)]
pub struct Solid {
    pub x: i32,
    pub y: i32,
    pub cell_x: i32,
    pub cell_y: i32,
    pub block: BlockKind,
    pub used: bool,
    pub broken: bool,
    pub bump: i32,
}

impl Solid {
    pub fn at_cell(cell_x: i32, cell_y: i32, block: BlockKind) -> Self {
        Self {
            x: cell_x.saturating_mul(TILE_FP),
            y: cell_y.saturating_mul(TILE_FP),
            cell_x,
            cell_y,
            block,
            used: false,
            broken: false,
            bump: 0,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: TILE_FP,
            h: TILE_FP,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YHit {
    None,
    Floor,
    Ceiling,
}

#[derive(Clone, Copy, Debug)]
pub struct MoveHit {
    pub wall: bool,
    pub y: YHit,
    pub ceiling: Option<usize>,
    pub floor: Option<usize>,
}

impl Default for MoveHit {
    fn default() -> Self {
        Self {
            wall: false,
            y: YHit::None,
            ceiling: None,
            floor: None,
        }
    }
}

pub fn occupied(solids: &[Solid], cell_x: i32, cell_y: i32) -> bool {
    solids
        .iter()
        .any(|solid| !solid.broken && solid.cell_x == cell_x && solid.cell_y == cell_y)
}

pub fn on_floor(body: Rect, solids: &[Solid]) -> bool {
    let inset = (body.w / 8).max(FP);
    let probe = Rect {
        x: body.x + inset,
        y: body.bottom(),
        w: (body.w - inset.saturating_mul(2)).max(FP),
        h: FP / 2,
    };
    solids
        .iter()
        .any(|solid| !solid.broken && overlaps(probe, solid.rect()))
}

/// Move a body. `corner` enables the player underside clip.
pub fn move_body(body: &mut Rect, vx: i32, vy: i32, solids: &[Solid], corner: bool) -> MoveHit {
    let mut hit = MoveHit::default();
    if vx != 0 {
        body.x = body.x.saturating_add(vx);
        for _ in 0..4 {
            let mut touched = false;
            for solid in solids {
                if solid.broken || !overlaps(*body, solid.rect()) {
                    continue;
                }
                touched = true;
                hit.wall = true;
                if vx > 0 {
                    body.x = solid.x - body.w;
                } else {
                    body.x = solid.x + TILE_FP;
                }
            }
            if !touched {
                break;
            }
        }
    }
    if vy == 0 {
        return hit;
    }
    body.y = body.y.saturating_add(vy);
    if corner && vy < 0 {
        apply_corner_nudge(body, solids);
    }
    for _ in 0..4 {
        let mut touched = false;
        for (index, solid) in solids.iter().enumerate() {
            if solid.broken || !overlaps(*body, solid.rect()) {
                continue;
            }
            if vy < 0 && corner && corner_nudge(*body, solid, solids).is_some() {
                continue;
            }
            touched = true;
            if vy > 0 {
                body.y = solid.y - body.h;
                hit.y = YHit::Floor;
                hit.floor = Some(index);
            } else {
                body.y = solid.y + TILE_FP;
                hit.y = YHit::Ceiling;
                hit.ceiling = Some(index);
            }
        }
        if !touched {
            break;
        }
    }
    hit
}

fn apply_corner_nudge(body: &mut Rect, solids: &[Solid]) {
    for solid in solids {
        if solid.broken || !overlaps(*body, solid.rect()) {
            continue;
        }
        if let Some(nudge) = corner_nudge(*body, solid, solids) {
            body.x = body.x.saturating_add(nudge);
            return;
        }
    }
}

fn corner_nudge(body: Rect, solid: &Solid, solids: &[Solid]) -> Option<i32> {
    let into_left = body.right().saturating_sub(solid.x);
    let into_right = solid.x.saturating_add(TILE_FP).saturating_sub(body.x);
    if into_left > 0 && into_left <= CORNER_FP && into_left < into_right {
        if !occupied(solids, solid.cell_x - 1, solid.cell_y) {
            return Some(-NUDGE_FP);
        }
    }
    if into_right > 0 && into_right <= CORNER_FP && into_right < into_left {
        if !occupied(solids, solid.cell_x + 1, solid.cell_y) {
            return Some(NUDGE_FP);
        }
    }
    None
}

/// 18 px rise over 8 frames, peaking at the middle of the bump.
pub fn bump_offset(frames_left: i32) -> i32 {
    if !(1..=8).contains(&frames_left) {
        return 0;
    }
    let phase = 8 - frames_left;
    let tri = if phase < 4 { phase + 1 } else { 8 - phase };
    tri.saturating_mul(NUDGE_FP)
}
