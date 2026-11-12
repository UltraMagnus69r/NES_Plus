//! 1920×1080 screen space. 72×72 tiles. 4096 units = 1 screen pixel.
//! Character motion, power-ups, enemies, hazards, and collisions step once per 60 Hz tick.

pub mod character_physics;
pub mod powerups;
pub mod rules;
mod trig;
mod world;

pub use world::World;

pub const FP: i32 = 4096;
pub const TILE_PX: i32 = 72;
pub const TILE_FP: i32 = TILE_PX * FP;
pub const SCREEN_W: i32 = 1920;
pub const SCREEN_H: i32 = 1080;
pub const SCREEN_W_FP: i32 = SCREEN_W * FP;
pub const SCREEN_H_FP: i32 = SCREEN_H * FP;
pub const HIT_W: i32 = 54 * FP;
pub const HIT_H: i32 = 72 * FP;
pub const HIT_TALL: i32 = 144 * FP;
pub const INSET: i32 = 9 * FP;
/// Outer bottom-corner width that does not stop an upward jump: 13.5 px.
pub const CORNER_FP: i32 = 55296;
/// Horizontal nudge while a corner clip is clearing: 4.5 px.
pub const NUDGE_FP: i32 = 18432;
pub const VIEW_MARGIN: i32 = TILE_FP;

/// Course pixels (the stored 16px grid) to this screen: 72/16 = 4.5, times 4096.
pub const COURSE_DIV: f32 = 18432.0;

pub fn course_px(units: i32) -> f32 {
    units as f32 / COURSE_DIV
}

pub fn cell_origin(cell: i32) -> i32 {
    cell.saturating_mul(TILE_FP)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn right(self) -> i32 {
        self.x.saturating_add(self.w)
    }

    pub fn bottom(self) -> i32 {
        self.y.saturating_add(self.h)
    }
}

pub fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() && a.right() > b.x && a.y < b.bottom() && a.bottom() > b.y
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterId {
    Mario,
    Luigi,
    Toad,
    Peach,
}

impl Default for CharacterId {
    fn default() -> Self {
        Self::Mario
    }
}
