//! Springs, vines, pipes, pits, and rotating fire bars.
//! Dynamic bars stay dormant until the 1920×1080 camera reaches them.

use super::super::super::trig::{cos_mdeg, sin_mdeg};
use super::super::super::{overlaps, Rect, FP, SCREEN_H_FP, SCREEN_W_FP, TILE_FP, VIEW_MARGIN};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HazardKind {
    Contact,
    Spring,
    Vine,
    FireBar,
    PipeDown,
    PipeRight,
}

#[derive(Clone, Debug)]
pub struct Hazard {
    pub kind: HazardKind,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub cell_x: i32,
    pub cell_y: i32,
    pub awake: bool,
    pub warp_level: String,
    pub warp_x: i32,
    pub warp_y: i32,
    pub warp_set: bool,
    pub angle: i32,
    pub balls: i32,
}

impl Hazard {
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ball {
    pub x: i32,
    pub y: i32,
}

pub fn sees(x: i32, y: i32, w: i32, h: i32, cam_x: i32, cam_y: i32) -> bool {
    let left = cam_x - VIEW_MARGIN;
    let top = cam_y - VIEW_MARGIN;
    let right = cam_x + SCREEN_W_FP + VIEW_MARGIN;
    let bottom = cam_y + SCREEN_H_FP + VIEW_MARGIN;
    x < right && x.saturating_add(w) > left && y < bottom && y.saturating_add(h) > top
}

/// Advance awake fire bars by about 2.8 degrees. Returns whether any ball touches `body`.
pub fn step_firebars(hazards: &mut [Hazard], body: Rect, cam_x: i32, cam_y: i32) -> bool {
    let mut hit = false;
    for hazard in hazards.iter_mut() {
        if hazard.kind != HazardKind::FireBar {
            continue;
        }
        if !hazard.awake {
            if sees(hazard.x, hazard.y, hazard.w, hazard.h, cam_x, cam_y) {
                hazard.awake = true;
            } else {
                continue;
            }
        }
        hazard.angle = hazard.angle.wrapping_add(2_800);
        if bars_touch(hazard, body) {
            hit = true;
        }
    }
    hit
}

pub fn bars_touch(hazard: &Hazard, body: Rect) -> bool {
    let mut balls = [Ball { x: 0, y: 0 }; 12];
    let count = ball_positions(hazard, &mut balls);
    let size = 24 * FP;
    balls.iter().take(count).any(|ball| {
        let rect = Rect {
            x: ball.x - size / 2,
            y: ball.y - size / 2,
            w: size,
            h: size,
        };
        overlaps(body, rect)
    })
}

pub fn ball_positions(hazard: &Hazard, out: &mut [Ball]) -> usize {
    let balls = hazard.balls.clamp(1, out.len() as i32);
    let length = if balls >= 12 { 432 } else { 216 };
    let spacing = length / balls;
    let cx = hazard.x + TILE_FP / 2;
    let cy = hazard.y + TILE_FP / 2;
    let s = sin_mdeg(hazard.angle) as i64;
    let c = cos_mdeg(hazard.angle) as i64;
    let n = balls as usize;
    for (i, slot) in out.iter_mut().take(n).enumerate() {
        let radius = (i as i32 + 1).saturating_mul(spacing).saturating_mul(FP);
        *slot = Ball {
            x: cx + (i64::from(radius) * c / 4096) as i32,
            y: cy + (i64::from(radius) * s / 4096) as i32,
        };
    }
    n
}

pub fn feet_overlap(body: Rect, hazard: &Hazard) -> bool {
    let feet = Rect {
        x: body.x + body.w / 8,
        y: body.bottom() - FP,
        w: (body.w - body.w / 4).max(FP),
        h: FP * 2,
    };
    overlaps(feet, hazard.rect())
}
