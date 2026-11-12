//! Integer AABB resolution on the same tile grid the float sim uses.
//! Solids arrive as pixel AABBs and are scaled by [`FP_ONE`](super::fp::FP_ONE) at this boundary.

use super::fp::{px_to_fp, FP_ONE};
use super::Aabb;

#[derive(Clone, Copy, Debug)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectI {
    pub fn from_aabb(aabb: &Aabb) -> Self {
        Self {
            x: px_to_fp(aabb.x),
            y: px_to_fp(aabb.y),
            w: px_to_fp(aabb.w).max(1),
            h: px_to_fp(aabb.h).max(1),
        }
    }
}

pub fn from_aabbs(solids: &[Aabb]) -> Vec<RectI> {
    solids.iter().map(RectI::from_aabb).collect()
}

pub fn overlaps(a: RectI, b: RectI) -> bool {
    a.x < b.x.saturating_add(b.w)
        && a.x.saturating_add(a.w) > b.x
        && a.y < b.y.saturating_add(b.h)
        && a.y.saturating_add(a.h) > b.y
}

/// Move on X and separate from solids. Returns true when a wall was hit.
pub fn sweep_x(body: &mut RectI, vx: i32, solids: &[RectI]) -> bool {
    if vx == 0 {
        return false;
    }
    body.x = body.x.saturating_add(vx);
    let mut hit = false;
    for solid in solids {
        if !overlaps(*body, *solid) {
            continue;
        }
        hit = true;
        if vx > 0 {
            body.x = solid.x - body.w;
        } else {
            body.x = solid.x + solid.w;
        }
    }
    hit
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YHit {
    None,
    Floor,
    Ceiling,
}

pub fn sweep_y(body: &mut RectI, vy: i32, solids: &[RectI]) -> YHit {
    if vy == 0 {
        return YHit::None;
    }
    body.y = body.y.saturating_add(vy);
    let mut hit = YHit::None;
    for solid in solids {
        if !overlaps(*body, *solid) {
            continue;
        }
        if vy > 0 {
            body.y = solid.y - body.h;
            hit = YHit::Floor;
        } else {
            body.y = solid.y + solid.h;
            hit = YHit::Ceiling;
        }
    }
    hit
}

/// True when a sliver under the feet touches a solid. Edges that only touch do overlap.
pub fn feet_on_ground(body: RectI, solids: &[RectI]) -> bool {
    let inset = FP_ONE.min(body.w / 4).max(1);
    let w = (body.w - inset.saturating_mul(2)).max(1);
    let probe = RectI {
        x: body.x + inset,
        y: body.y + body.h,
        w,
        h: FP_ONE / 2,
    };
    solids.iter().any(|solid| overlaps(probe, *solid))
}
