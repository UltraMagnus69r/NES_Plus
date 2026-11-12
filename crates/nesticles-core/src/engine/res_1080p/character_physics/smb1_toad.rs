//! SMB1 Toad. Faster sprint, snappy grip, and a heavy jump.

use super::Params;

pub const MAX_WALK_SPEED: i32 = 32256;
pub const MAX_RUN_SPEED: i32 = 55296;
pub const WALK_ACCEL: i32 = 958;
pub const RUN_ACCEL: i32 = 1436;
pub const RELEASE_DECEL: i32 = 1310;
pub const SKID_DECEL: i32 = 2620;
pub const JUMP_VEL_SLOW: i32 = -69632;
pub const JUMP_VEL_FAST: i32 = -86016;
pub const JUMP_FAST_AT: i32 = 49152;
pub const GRAVITY_HOLD: i32 = 2765;
pub const GRAVITY_NORMAL: i32 = 9216;
pub const MAX_FALL_SPEED: i32 = 92160;

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
        gravity_hold: GRAVITY_HOLD,
        gravity_normal: GRAVITY_NORMAL,
        max_fall: MAX_FALL_SPEED,
        hover: false,
    }
}
