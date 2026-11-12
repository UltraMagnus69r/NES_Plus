//! Original fixed-point character step from the SMB1 numeric spec.
//! Positions and velocities stay in i32 units of 1/4096 pixel. Nothing here is 6502.

use super::collide::{feet_on_ground, sweep_x, sweep_y, RectI, YHit};
use super::fp::{px_to_fp, FP_ONE};
use super::{Body, Input};

pub const MAX_WALK_SPEED: i32 = 6144;
pub const MAX_RUN_SPEED: i32 = 10240;
pub const WALK_ACCEL: i32 = 152;
pub const RUN_ACCEL: i32 = 228;
pub const RELEASE_DECEL: i32 = 208;
pub const SKID_DECEL: i32 = 416;
pub const JUMP_VEL_SLOW: i32 = -16384;
pub const JUMP_VEL_FAST: i32 = -20480;
pub const JUMP_RUN_SPEED: i32 = 9472;
pub const GRAVITY_HOLD_A: i32 = 512;
pub const GRAVITY_NORMAL: i32 = 1792;
pub const MAX_FALL_SPEED: i32 = 18432;
pub const RUN_BUFFER_FRAMES: i32 = 10;

/// One SMB1 tick is exactly 1/60 second. Leftover time stays in the accumulator.
pub const STEP_NS: i64 = 1_000_000_000 / 60;

#[derive(Clone, Debug)]
pub struct Smb1Body {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub vx: i32,
    pub vy: i32,
    pub facing: i32,
    pub on_ground: bool,
    pub jumping: bool,
    pub run_timer: i32,
    pub acc_ns: i64,
    pub ready: bool,
}

impl Default for Smb1Body {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            w: 12 * FP_ONE,
            h: 14 * FP_ONE,
            vx: 0,
            vy: 0,
            facing: 1,
            on_ground: false,
            jumping: false,
            run_timer: 0,
            acc_ns: 0,
            ready: false,
        }
    }
}

impl Smb1Body {
    pub fn sync_from_float(&mut self, player: &Body) {
        self.x = px_to_fp(player.x);
        self.y = px_to_fp(player.y);
        self.w = px_to_fp(player.w).max(FP_ONE);
        self.h = px_to_fp(player.h).max(FP_ONE);
        self.vx = px_to_fp(player.vx / 60.0);
        self.vy = px_to_fp(player.vy / 60.0);
        self.facing = if player.facing < 0 { -1 } else { 1 };
        self.on_ground = player.on_ground;
        self.jumping = false;
        self.run_timer = 0;
        self.acc_ns = 0;
        self.ready = true;
    }

    pub fn write_to(&self, player: &mut Body) {
        player.x = super::fp::fp_to_px(self.x);
        player.y = super::fp::fp_to_px(self.y);
        player.w = super::fp::fp_to_px(self.w);
        player.h = super::fp::fp_to_px(self.h);
        player.vx = super::fp::fp_to_px(self.vx) * 60.0;
        player.vy = super::fp::fp_to_px(self.vy) * 60.0;
        player.on_ground = self.on_ground;
        player.facing = if self.facing < 0 { -1 } else { 1 };
    }

    pub fn apply_pixel_velocity(&mut self, vx_px_s: f32, vy_px_s: f32) {
        self.vx = px_to_fp(vx_px_s / 60.0);
        self.vy = px_to_fp(vy_px_s / 60.0);
    }

    /// How many whole 1/60 slices sit in `dt`. Extra time stays in `acc_ns`.
    pub fn consume(&mut self, dt: f32) -> u32 {
        if dt <= 0.0 {
            return 0;
        }
        let add = (f64::from(dt) * 1_000_000_000.0).round() as i64;
        self.acc_ns = self.acc_ns.saturating_add(add);
        let mut ticks = 0u32;
        while self.acc_ns >= STEP_NS {
            self.acc_ns -= STEP_NS;
            ticks += 1;
            if ticks == 8 {
                break;
            }
        }
        ticks
    }

    fn rect(&self) -> RectI {
        RectI {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

/// One 60 Hz tick. Returns whether a jump started this tick.
pub fn tick(body: &mut Smb1Body, input: &Input, solids: &[RectI]) -> bool {
    step_horizontal(body, input);
    if body.on_ground && !feet_on_ground(body.rect(), solids) {
        body.on_ground = false;
    }
    let mut jumped = false;
    if body.on_ground && input.jump_pressed {
        body.vy = if body.vx.abs() >= JUMP_RUN_SPEED {
            JUMP_VEL_FAST
        } else {
            JUMP_VEL_SLOW
        };
        body.on_ground = false;
        body.jumping = true;
        jumped = true;
    }
    if body.on_ground {
        body.vy = 0;
        body.jumping = false;
    } else {
        let hold = body.vy < 0 && input.jump_held && body.jumping;
        let gravity = if hold { GRAVITY_HOLD_A } else { GRAVITY_NORMAL };
        body.vy = body.vy.saturating_add(gravity).min(MAX_FALL_SPEED);
        if !input.jump_held || body.vy >= 0 {
            body.jumping = false;
        }
    }
    let mut rect = body.rect();
    if sweep_x(&mut rect, body.vx, solids) {
        body.vx = 0;
    }
    body.x = rect.x;
    match sweep_y(&mut rect, body.vy, solids) {
        YHit::Floor => {
            body.vy = 0;
            body.on_ground = true;
            body.jumping = false;
        }
        YHit::Ceiling => {
            body.vy = 0;
            body.on_ground = false;
        }
        YHit::None => {
            if body.vy != 0 {
                body.on_ground = false;
            }
        }
    }
    body.y = rect.y;
    if body.facing >= 0 {
        body.facing = 1;
    } else {
        body.facing = -1;
    }
    jumped
}

fn step_horizontal(body: &mut Smb1Body, input: &Input) {
    if input.run_held {
        body.run_timer = RUN_BUFFER_FRAMES;
    } else if body.run_timer > 0 {
        body.run_timer -= 1;
    }
    let running = body.run_timer > 0;
    let max_speed = if running {
        MAX_RUN_SPEED
    } else {
        MAX_WALK_SPEED
    };
    let accel = if running { RUN_ACCEL } else { WALK_ACCEL };
    let dir = match (input.left, input.right) {
        (true, false) => -1,
        (false, true) => 1,
        _ => 0,
    };
    if body.on_ground && dir != 0 && body.vx != 0 && dir != body.vx.signum() {
        body.vx = approach_zero(body.vx, SKID_DECEL);
        body.facing = dir;
    } else if dir != 0 {
        let same_way = body.vx == 0 || body.vx.signum() == dir;
        if same_way && body.vx.abs() < max_speed {
            body.vx = body.vx.saturating_add(dir.saturating_mul(accel));
            if body.vx.signum() == dir && body.vx.abs() > max_speed {
                body.vx = dir.saturating_mul(max_speed);
            }
        } else if !same_way {
            body.vx = body.vx.saturating_add(dir.saturating_mul(accel));
        }
        if body.on_ground {
            body.facing = dir;
        }
        if body.on_ground && body.vx.signum() == dir && body.vx.abs() > max_speed {
            body.vx = pull_toward_max(body.vx, max_speed);
        }
    } else if body.on_ground {
        if body.vx.abs() > max_speed {
            body.vx = pull_toward_max(body.vx, max_speed);
        } else {
            body.vx = approach_zero(body.vx, RELEASE_DECEL);
        }
    }
    if !body.on_ground {
        body.facing = if body.facing < 0 { -1 } else { 1 };
    }
}

fn approach_zero(vx: i32, decel: i32) -> i32 {
    if vx > 0 {
        if vx <= decel {
            0
        } else {
            vx - decel
        }
    } else if vx < 0 {
        if vx >= -decel {
            0
        } else {
            vx + decel
        }
    } else {
        0
    }
}

fn pull_toward_max(vx: i32, max_speed: i32) -> i32 {
    if vx > max_speed {
        let next = vx - RELEASE_DECEL;
        if next < max_speed {
            max_speed
        } else {
            next
        }
    } else if vx < -max_speed {
        let next = vx + RELEASE_DECEL;
        if next > -max_speed {
            -max_speed
        } else {
            next
        }
    } else {
        vx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_under(body: &Smb1Body) -> Vec<RectI> {
        vec![RectI {
            x: body.x - 64 * FP_ONE,
            y: body.y + body.h,
            w: 256 * FP_ONE,
            h: 16 * FP_ONE,
        }]
    }

    #[test]
    fn slow_and_fast_jump_use_the_spec_velocities() {
        let mut slow = Smb1Body::default();
        slow.on_ground = true;
        slow.ready = true;
        let floor = floor_under(&slow);
        let held = Input {
            jump_pressed: true,
            jump_held: true,
            ..Input::default()
        };
        assert!(tick(&mut slow, &held, &floor));
        assert_eq!(slow.vy, JUMP_VEL_SLOW + GRAVITY_HOLD_A);

        let mut fast = Smb1Body::default();
        fast.on_ground = true;
        // Ground release decel runs before the jump, so start one step above the threshold.
        fast.vx = JUMP_RUN_SPEED + RELEASE_DECEL;
        let floor = floor_under(&fast);
        let tap = Input {
            jump_pressed: true,
            jump_held: false,
            ..Input::default()
        };
        assert!(tick(&mut fast, &tap, &floor));
        assert_eq!(fast.vy, JUMP_VEL_FAST + GRAVITY_NORMAL);
        assert!(!fast.jumping);
    }

    #[test]
    fn run_buffer_holds_for_ten_frames_and_skid_snaps_at_zero() {
        let mut body = Smb1Body::default();
        body.on_ground = true;
        let floor = floor_under(&body);
        let run = Input {
            right: true,
            run_held: true,
            ..Input::default()
        };
        tick(&mut body, &run, &floor);
        assert_eq!(body.run_timer, RUN_BUFFER_FRAMES);
        let release = Input {
            right: true,
            ..Input::default()
        };
        tick(&mut body, &release, &floor);
        assert_eq!(body.run_timer, RUN_BUFFER_FRAMES - 1);

        body.vx = SKID_DECEL - 10;
        body.on_ground = true;
        let skid = Input {
            left: true,
            ..Input::default()
        };
        tick(&mut body, &skid, &floor);
        assert_eq!(body.vx, 0);
        assert_eq!(body.facing, -1);
    }
}
