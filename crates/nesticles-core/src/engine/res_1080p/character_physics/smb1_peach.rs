//! SMB1 Peach. Slower sprint, plus a short hover after the jump apex.

use super::Params;

pub const MAX_WALK_SPEED: i32 = 24576;
pub const MAX_RUN_SPEED: i32 = 39936;
pub const WALK_ACCEL: i32 = 615;
pub const RUN_ACCEL: i32 = 922;
pub const RELEASE_DECEL: i32 = 842;
pub const SKID_DECEL: i32 = 1684;
pub const JUMP_VEL_SLOW: i32 = -67584;
pub const JUMP_VEL_FAST: i32 = -82944;
pub const JUMP_FAST_AT: i32 = 36864;
pub const GRAVITY_HOLD: i32 = 2150;
pub const GRAVITY_NORMAL: i32 = 7373;
pub const MAX_FALL_SPEED: i32 = 78336;

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
        hover: true,
    }
}
