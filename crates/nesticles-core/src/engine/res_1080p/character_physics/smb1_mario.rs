//! SMB1 Mario. Constants are already in 4096-units (1 screen pixel = 4096).

use super::Params;

pub const MAX_WALK_SPEED: i32 = 27648;
pub const MAX_RUN_SPEED: i32 = 46080;
pub const WALK_ACCEL: i32 = 684;
pub const RUN_ACCEL: i32 = 1026;
pub const RELEASE_DECEL: i32 = 936;
pub const SKID_DECEL: i32 = 1872;
pub const JUMP_VEL_SLOW: i32 = -73728;
pub const JUMP_VEL_FAST: i32 = -92160;
pub const JUMP_FAST_AT: i32 = 42624;
pub const GRAVITY_HOLD_A: i32 = 2304;
pub const GRAVITY_NORMAL: i32 = 8064;
pub const MAX_FALL_SPEED: i32 = 82944;

pub fn params() -> Params {
    Params {
        max_walk: MAX_WALK_SPEED,
        max_run: MAX_RUN_SPEED,
        walk_accel: WALK_ACCEL,
        run_accel: RUN_ACCEL,
        release_decel: RELEASE_DECEL,
        skid_decel: SKID_DECEL,
        jump_slow: JUMP_VEL_SLOW,
        jump_fast: JUMP_VEL_FAST,
        jump_fast_at: JUMP_FAST_AT,
        gravity_hold: GRAVITY_HOLD_A,
        gravity_normal: GRAVITY_NORMAL,
        max_fall: MAX_FALL_SPEED,
        hover: false,
    }
}
