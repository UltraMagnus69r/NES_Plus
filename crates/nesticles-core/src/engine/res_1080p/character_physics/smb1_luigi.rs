//! SMB1 Luigi. Slippery ground, a higher jump, and lower gravity.

use super::Params;

pub const MAX_WALK_SPEED: i32 = 27648;
pub const MAX_RUN_SPEED: i32 = 46080;
pub const WALK_ACCEL: i32 = 492;
pub const RUN_ACCEL: i32 = 738;
pub const RELEASE_DECEL: i32 = 432;
pub const SKID_DECEL: i32 = 864;
pub const JUMP_VEL_SLOW: i32 = -81920;
pub const JUMP_VEL_FAST: i32 = -99328;
pub const JUMP_FAST_AT: i32 = 42624;
pub const GRAVITY_HOLD: i32 = 1843;
pub const GRAVITY_NORMAL: i32 = 6451;
pub const MAX_FALL_SPEED: i32 = 73728;

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
