//! SMB1-style enemy step. Original fixed-point motion, not a character port.
//! Dormant until the body enters the camera plus a small margin. Velocities are per 1/60 tick.

use super::{resolve_style, CameraPx, EnemyAiStyle, EnemyArchetype, EnemyRuntime};
use crate::physics::collide::{feet_on_ground, sweep_x, sweep_y, RectI, YHit};
use crate::physics::fp::{fp_to_px, FP_ONE};

const WALKER_SPEED: i32 = 2048;
const GRAVITY: i32 = 1792;
const MAX_FALL: i32 = 18432;
const VIEW_MARGIN: f32 = 32.0;
const FLAG_GROUNDED: u32 = 1 << 0;
const FLAG_AWAKE: u32 = 1 << 1;
const FLAG_SHOT: u32 = 1 << 2;
const FLAG_RISING: u32 = 1 << 3;

pub fn step_enemies(
    runtime: &mut [EnemyRuntime],
    alive: &[bool],
    solids: &[RectI],
    camera: CameraPx,
    level_style: EnemyAiStyle,
    global_style: EnemyAiStyle,
) {
    for (index, enemy) in runtime.iter_mut().enumerate() {
        if !alive.get(index).copied().unwrap_or(false) {
            continue;
        }
        let style = resolve_style(enemy.style_override, level_style, global_style);
        match style {
            EnemyAiStyle::Smb1Nes => step_one(enemy, solids, camera),
        }
    }
}

fn step_one(enemy: &mut EnemyRuntime, solids: &[RectI], camera: CameraPx) {
    if !enemy.awake {
        if !in_view(enemy, camera) {
            return;
        }
        enemy.awake = true;
        enemy.flags |= FLAG_AWAKE;
    }
    let motion = motion_for(&enemy.archetype);
    let custom_len = match &enemy.archetype {
        EnemyArchetype::Custom(name) => Some(name.len()),
        _ => None,
    };
    if let Some(len) = custom_len {
        enemy.timers[3] = i32::try_from(len).unwrap_or(i32::MAX);
    }
    if enemy.patrol_dir == 0 {
        enemy.patrol_dir = -1;
    }
    enemy.timers[0] = enemy.timers[0].saturating_add(1);
    let mut speed = motion.speed;
    if motion.shooter {
        let frame = enemy.timers[0].rem_euclid(90);
        if frame >= 72 {
            speed = 0;
            enemy.flags |= FLAG_SHOT;
            if frame == 72 {
                enemy.phase = enemy.phase.saturating_add(1);
            }
        } else {
            enemy.flags &= !FLAG_SHOT;
        }
    }
    if !motion.fly {
        enemy.on_ground = feet_on_ground(rect(enemy), solids);
    }
    if motion.jumper && enemy.on_ground && enemy.timers[1] >= 45 {
        enemy.vy = -12288;
        enemy.on_ground = false;
        enemy.timers[1] = 0;
        enemy.phase = enemy.phase.saturating_add(1);
    } else if motion.jumper {
        enemy.timers[1] = enemy.timers[1].saturating_add(1);
    }
    if motion.boss {
        enemy.timers[2] = enemy.timers[2].saturating_add(1);
        if enemy.timers[2] >= 120 {
            enemy.timers[2] = 0;
            enemy.phase = enemy.phase.saturating_add(1);
        }
    }
    let sign: i32 = if enemy.patrol_dir < 0 { -1 } else { 1 };
    enemy.vx = sign.saturating_mul(speed);
    if motion.fly {
        enemy.timers[2] = enemy.timers[2].saturating_add(1);
        enemy.phase = enemy.timers[2] / 32;
        enemy.vy = if enemy.phase % 2 == 0 { -1024 } else { 1024 };
        if enemy.vy < 0 {
            enemy.flags |= FLAG_RISING;
        } else {
            enemy.flags &= !FLAG_RISING;
        }
        enemy.on_ground = false;
    } else if enemy.on_ground {
        enemy.vy = 0;
        enemy.flags &= !FLAG_RISING;
    } else {
        enemy.vy = enemy.vy.saturating_add(motion.gravity).min(motion.max_fall);
        if enemy.vy < 0 {
            enemy.flags |= FLAG_RISING;
        } else {
            enemy.flags &= !FLAG_RISING;
        }
    }
    let mut body = rect(enemy);
    if sweep_x(&mut body, enemy.vx, solids) && speed != 0 {
        enemy.patrol_dir = -sign;
        enemy.vx = enemy.patrol_dir.saturating_mul(speed);
        enemy.phase = enemy.phase.saturating_add(1);
    }
    enemy.x = body.x;
    match sweep_y(&mut body, enemy.vy, solids) {
        YHit::Floor => {
            enemy.vy = 0;
            enemy.on_ground = true;
        }
        YHit::Ceiling => {
            enemy.vy = 0;
            enemy.on_ground = false;
        }
        YHit::None => {
            if enemy.vy != 0 {
                enemy.on_ground = false;
            }
        }
    }
    enemy.y = body.y;
    if enemy.on_ground {
        enemy.flags |= FLAG_GROUNDED;
    } else {
        enemy.flags &= !FLAG_GROUNDED;
    }
}

struct Motion {
    speed: i32,
    gravity: i32,
    max_fall: i32,
    fly: bool,
    jumper: bool,
    shooter: bool,
    boss: bool,
}

fn motion_for(archetype: &EnemyArchetype) -> Motion {
    let mut motion = Motion {
        speed: WALKER_SPEED,
        gravity: GRAVITY,
        max_fall: MAX_FALL,
        fly: false,
        jumper: false,
        shooter: false,
        boss: false,
    };
    match archetype {
        EnemyArchetype::Walker => {}
        EnemyArchetype::ShellWalker => motion.speed = 4096,
        EnemyArchetype::Jumper => motion.jumper = true,
        EnemyArchetype::Flyer => {
            motion.fly = true;
            motion.gravity = 0;
        }
        EnemyArchetype::Swimmer => {
            motion.speed = 1024;
            motion.gravity = 512;
            motion.max_fall = 6144;
        }
        EnemyArchetype::Shooter => {
            motion.speed = 1536;
            motion.shooter = true;
        }
        EnemyArchetype::Boss => {
            motion.speed = 1024;
            motion.boss = true;
        }
        EnemyArchetype::Goomba
        | EnemyArchetype::GreenKoopa
        | EnemyArchetype::BuzzyBeetle
        | EnemyArchetype::RedKoopa
        | EnemyArchetype::GreenParatroopa
        | EnemyArchetype::RedParatroopa
        | EnemyArchetype::GreenPiranha
        | EnemyArchetype::Lakitu
        | EnemyArchetype::HammerBro
        | EnemyArchetype::BulletBill
        | EnemyArchetype::Blooper
        | EnemyArchetype::CheepGreen
        | EnemyArchetype::CheepRed
        | EnemyArchetype::CheepLeap
        | EnemyArchetype::Podoboo
        | EnemyArchetype::Bowser
        | EnemyArchetype::Spiny => {}
        EnemyArchetype::Custom(name) => {
            let extra = (name.len() as i32).saturating_mul(64).min(512);
            motion.speed = WALKER_SPEED + extra;
        }
    }
    motion
}

fn rect(enemy: &EnemyRuntime) -> RectI {
    RectI {
        x: enemy.x,
        y: enemy.y,
        w: enemy.w,
        h: enemy.h,
    }
}

fn in_view(enemy: &EnemyRuntime, camera: CameraPx) -> bool {
    let ex = fp_to_px(enemy.x);
    let ey = fp_to_px(enemy.y);
    let ew = fp_to_px(enemy.w.max(FP_ONE));
    let eh = fp_to_px(enemy.h.max(FP_ONE));
    let left = camera.x - VIEW_MARGIN;
    let top = camera.y - VIEW_MARGIN;
    let right = camera.x + camera.w + VIEW_MARGIN;
    let bottom = camera.y + camera.h + VIEW_MARGIN;
    ex < right && ex + ew > left && ey < bottom && ey + eh > top
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::fp::FP_ONE;

    #[test]
    fn each_archetype_wakes_and_moves_inside_the_camera() {
        let solids = vec![RectI {
            x: 0,
            y: 20 * FP_ONE,
            w: 400 * FP_ONE,
            h: 16 * FP_ONE,
        }];
        let camera = CameraPx {
            x: 0.0,
            y: 0.0,
            w: 256.0,
            h: 240.0,
        };
        let kinds = [
            EnemyArchetype::Walker,
            EnemyArchetype::ShellWalker,
            EnemyArchetype::Jumper,
            EnemyArchetype::Flyer,
            EnemyArchetype::Swimmer,
            EnemyArchetype::Shooter,
            EnemyArchetype::Boss,
            EnemyArchetype::Custom("scout".into()),
        ];
        let mut runtime: Vec<_> = kinds
            .into_iter()
            .map(|kind| {
                EnemyRuntime::spawn(
                    40 * FP_ONE,
                    6 * FP_ONE,
                    14 * FP_ONE,
                    14 * FP_ONE,
                    kind,
                    None,
                    2,
                    0,
                )
            })
            .collect();
        let alive = vec![true; runtime.len()];
        for _ in 0..60 {
            step_enemies(
                &mut runtime,
                &alive,
                &solids,
                camera,
                EnemyAiStyle::Smb1Nes,
                EnemyAiStyle::Smb1Nes,
            );
        }
        assert!(runtime.iter().all(|enemy| enemy.awake));
        assert_eq!(runtime[0].vx, -WALKER_SPEED);
        assert_eq!(runtime[1].vx.abs(), 4096);
        assert!(runtime[7].timers[3] > 0);
        assert!(runtime[3].flags & FLAG_RISING == FLAG_RISING || runtime[3].vy > 0);
    }
}
