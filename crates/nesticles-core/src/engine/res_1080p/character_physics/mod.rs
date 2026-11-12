//! Shared SMB1 character step. Each sibling file is only that character's constants.
//! Positions, velocities, and timers are i32 in 4096-units. One call is one 60 Hz tick.

mod smb1_luigi;
mod smb1_mario;
mod smb1_peach;
mod smb1_toad;

use super::{CharacterId, HIT_H, HIT_W};
use crate::engine::Pad;

pub use smb1_luigi::params as luigi_params;
pub use smb1_mario::{
    params as mario_params, GRAVITY_HOLD_A, GRAVITY_NORMAL, MAX_RUN_SPEED, MAX_WALK_SPEED,
};
pub use smb1_peach::params as peach_params;
pub use smb1_toad::params as toad_params;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    Small,
    Super,
    Fire,
}

impl Default for Power {
    fn default() -> Self {
        Self::Small
    }
}

#[derive(Clone, Debug)]
pub struct Body {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub vx: i32,
    pub vy: i32,
    pub facing: i32,
    pub on_ground: bool,
    pub is_jumping: bool,
    pub run_timer: i32,
    pub hover_timer: i32,
    pub crouching: bool,
    pub power: Power,
    pub starman_timer: u16,
    pub invuln_timer: u16,
    pub shrink_timer: u16,
    pub lives: i32,
    pub run_was_held: bool,
    pub climbing: bool,
    pub spring_timer: i32,
}

impl Body {
    pub fn fresh(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            w: HIT_W,
            h: HIT_H,
            vx: 0,
            vy: 0,
            facing: 1,
            on_ground: false,
            is_jumping: false,
            run_timer: 0,
            hover_timer: 0,
            crouching: false,
            power: Power::Small,
            starman_timer: 0,
            invuln_timer: 0,
            shrink_timer: 0,
            lives: 3,
            run_was_held: false,
            climbing: false,
            spring_timer: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub max_walk: i32,
    pub max_run: i32,
    pub walk_accel: i32,
    pub run_accel: i32,
    pub release_decel: i32,
    pub skid_decel: i32,
    pub jump_slow: i32,
    pub jump_fast: i32,
    pub jump_fast_at: i32,
    pub gravity_hold: i32,
    pub gravity_normal: i32,
    pub max_fall: i32,
    pub hover: bool,
}

pub fn params_for(id: CharacterId) -> Params {
    match id {
        CharacterId::Mario => smb1_mario::params(),
        CharacterId::Luigi => smb1_luigi::params(),
        CharacterId::Toad => smb1_toad::params(),
        CharacterId::Peach => smb1_peach::params(),
    }
}

/// Horizontal speed, jump impulse, hold gravity, and Peach hover. Does not move `x`/`y`.
pub fn step_motion(body: &mut Body, input: &Pad, params: Params) -> bool {
    if input.run_held {
        body.run_timer = 10;
    } else if body.run_timer > 0 {
        body.run_timer -= 1;
    }
    let running = body.run_timer > 0;
    let max_speed = if running {
        params.max_run
    } else {
        params.max_walk
    };
    let accel = if running {
        params.run_accel
    } else {
        params.walk_accel
    };
    let dir = match (input.left, input.right) {
        (true, false) => -1,
        (false, true) => 1,
        _ => 0,
    };
    step_horizontal(body, dir, accel, max_speed, params);
    let mut jumped = false;
    if body.on_ground {
        if params.hover {
            body.hover_timer = 90;
        }
        body.vy = 0;
        body.is_jumping = false;
        if input.jump_pressed {
            let fast = body.vx.abs() >= params.jump_fast_at;
            body.vy = if fast {
                params.jump_fast
            } else {
                params.jump_slow
            };
            body.on_ground = false;
            body.is_jumping = true;
            jumped = true;
        }
    }
    if !body.on_ground {
        let hovering = params.hover && body.vy >= 0 && input.jump_held && body.hover_timer > 0;
        if hovering {
            let phase = (90 - body.hover_timer) / 4;
            body.vy = if phase % 2 == 0 { 1024 } else { -1024 };
            body.hover_timer = body.hover_timer.saturating_sub(1);
        } else {
            if params.hover && !input.jump_held {
                body.hover_timer = 0;
            }
            let hold = body.vy < 0 && input.jump_held && body.is_jumping;
            let gravity = if hold {
                params.gravity_hold
            } else {
                params.gravity_normal
            };
            body.vy = body.vy.saturating_add(gravity).min(params.max_fall);
            if !input.jump_held || body.vy >= 0 {
                body.is_jumping = false;
            }
        }
    }
    if body.facing == 0 {
        body.facing = 1;
    }
    jumped
}

fn step_horizontal(body: &mut Body, dir: i32, accel: i32, max_speed: i32, params: Params) {
    if body.on_ground && dir != 0 && body.vx != 0 && body.vx.signum() != dir {
        body.vx = approach_zero(body.vx, params.skid_decel);
        body.facing = dir;
        return;
    }
    if dir != 0 {
        let same = body.vx == 0 || body.vx.signum() == dir;
        if same {
            if body.vx.abs() < max_speed {
                body.vx = body.vx.saturating_add(dir.saturating_mul(accel));
                if body.vx.signum() == dir && body.vx.abs() > max_speed {
                    body.vx = dir.saturating_mul(max_speed);
                }
            } else if body.on_ground {
                body.vx = pull_toward(body.vx, max_speed, params.release_decel);
            }
        } else {
            body.vx = body.vx.saturating_add(dir.saturating_mul(accel));
            if body.vx.signum() == dir && body.vx.abs() > max_speed {
                body.vx = dir.saturating_mul(max_speed);
            }
        }
        if body.on_ground {
            body.facing = dir;
        }
    } else if body.on_ground {
        if body.vx.abs() > max_speed {
            body.vx = pull_toward(body.vx, max_speed, params.release_decel);
        } else {
            body.vx = approach_zero(body.vx, params.release_decel);
        }
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

fn pull_toward(vx: i32, limit: i32, decel: i32) -> i32 {
    if vx > limit {
        let next = vx - decel;
        if next < limit {
            limit
        } else {
            next
        }
    } else if vx < -limit {
        let next = vx + decel;
        if next > -limit {
            -limit
        } else {
            next
        }
    } else {
        vx
    }
}

/// Stomp bounce. Jump held uses the higher hop.
pub fn apply_stomp(body: &mut Body, jump_held: bool) {
    body.vy = if jump_held { -92160 } else { -73728 };
    body.on_ground = false;
    body.is_jumping = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mario_walk_caps_without_run_and_run_buffer_reaches_sprint() {
        let params = smb1_mario::params();
        let mut body = Body::fresh(0, 0);
        body.on_ground = true;
        let walk = Pad {
            right: true,
            ..Pad::default()
        };
        for _ in 0..120 {
            step_motion(&mut body, &walk, params);
            assert!(body.vx <= MAX_WALK_SPEED);
            assert_eq!(body.run_timer, 0);
        }
        assert_eq!(body.vx, MAX_WALK_SPEED);

        let mut body = Body::fresh(0, 0);
        body.on_ground = true;
        let run = Pad {
            right: true,
            run_held: true,
            ..Pad::default()
        };
        for _ in 0..120 {
            step_motion(&mut body, &run, params);
            assert!(body.vx <= MAX_RUN_SPEED);
        }
        assert_eq!(body.vx, MAX_RUN_SPEED);
        assert!(body.vx > MAX_WALK_SPEED);
        assert_eq!(body.run_timer, 10);
        let release = Pad {
            right: true,
            ..Pad::default()
        };
        step_motion(&mut body, &release, params);
        assert_eq!(body.run_timer, 9);
        assert_eq!(body.vx, MAX_RUN_SPEED);
    }

    #[test]
    fn releasing_jump_while_rising_uses_normal_gravity() {
        let params = smb1_mario::params();
        let mut body = Body::fresh(0, 0);
        body.on_ground = false;
        body.is_jumping = true;
        body.vy = -20000;
        let release = Pad::default();
        step_motion(&mut body, &release, params);
        assert_eq!(body.vy, -20000 + GRAVITY_NORMAL);
        assert_ne!(GRAVITY_HOLD_A, GRAVITY_NORMAL);
    }

    #[test]
    fn luigi_jump_impulse_is_higher_than_mario() {
        let mut mario = Body::fresh(0, 0);
        mario.on_ground = true;
        mario.vx = 0;
        let mut luigi = mario.clone();
        let jump = Pad {
            jump_pressed: true,
            jump_held: true,
            ..Pad::default()
        };
        step_motion(&mut mario, &jump, smb1_mario::params());
        step_motion(&mut luigi, &jump, smb1_luigi::params());
        assert!(luigi.vy < mario.vy, "luigi {} mario {}", luigi.vy, mario.vy);
    }

    #[test]
    fn peach_hover_holds_near_zero_and_counts_down() {
        let mut body = Body::fresh(0, 0);
        body.on_ground = false;
        body.vy = 800;
        body.hover_timer = 90;
        let held = Pad {
            jump_held: true,
            ..Pad::default()
        };
        step_motion(&mut body, &held, smb1_peach::params());
        assert!(body.vy.abs() <= 1024, "vy {}", body.vy);
        assert_eq!(body.hover_timer, 89);
    }

    #[test]
    fn stomp_sets_vy_to_the_smb1_bounce() {
        let mut body = Body::fresh(0, 0);
        body.vy = 4096;
        apply_stomp(&mut body, false);
        assert_eq!(body.vy, -73728);
    }
}
