//! SMB1 power-up style. Sizes, starman, mushrooms, flowers, and fireballs.
//! A bumped block's item uses vy = -2304 for 32 frames, then becomes active.

use super::super::character_physics::{Body, Power};
use super::super::rules::smb1::collision::{move_body, on_floor, Solid, YHit};
use super::super::rules::smb1::enemies::EnemyActor;
use super::super::{overlaps, Rect, FP, HIT_H, HIT_TALL, HIT_W, INSET, TILE_FP};

pub const SHRINK_FRAMES: u16 = 48;
pub const INVULN_FRAMES: u16 = 120;
pub const STAR_FRAMES: u16 = 600;
pub const RISE_VY: i32 = -2304;
pub const RISE_FRAMES: i32 = 32;
pub const MUSHROOM_VX: i32 = 18432;
pub const MUSHROOM_GRAVITY: i32 = 5120;
pub const STAR_VX: i32 = 23040;
pub const STAR_BOUNCE: i32 = -64512;
pub const STAR_GRAVITY: i32 = 4096;
pub const FIRE_VX: i32 = 64512;
pub const FIRE_GRAVITY: i32 = 8064;
pub const FIRE_BOUNCE: i32 = -46080;
pub const MAX_FIREBALLS: usize = 2;
const MAX_FALL: i32 = 82944;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Mushroom,
    OneUp,
    Flower,
    Star,
    Plain,
}

#[derive(Clone, Debug)]
pub struct ItemActor {
    pub kind: ItemKind,
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub w: i32,
    pub h: i32,
    pub rise: i32,
    pub active: bool,
    pub alive: bool,
    pub cell_x: i32,
    pub cell_y: i32,
    pub origin_y: i32,
    pub placed: bool,
    pub asset_id: String,
}

impl ItemActor {
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Fireball {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub w: i32,
    pub h: i32,
    pub alive: bool,
}

impl Fireball {
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hurt {
    Ignored,
    Shrink,
    Defeat,
}

/// Fire or Super always drops straight to Small, then blinks.
pub fn apply_damage(body: &mut Body) -> Hurt {
    if body.starman_timer > 0 || body.invuln_timer > 0 || body.shrink_timer > 0 {
        return Hurt::Ignored;
    }
    match body.power {
        Power::Small => Hurt::Defeat,
        Power::Super | Power::Fire => {
            let tall = body.h > HIT_H;
            body.power = Power::Small;
            body.shrink_timer = SHRINK_FRAMES;
            body.invuln_timer = INVULN_FRAMES;
            body.crouching = false;
            body.h = HIT_H;
            body.w = HIT_W;
            if tall {
                body.y = body.y.saturating_add(HIT_H);
            }
            Hurt::Shrink
        }
    }
}

pub fn spawn_reward(cell_x: i32, cell_y: i32, kind: ItemKind) -> ItemActor {
    let origin_y = cell_y.saturating_mul(TILE_FP);
    ItemActor {
        kind,
        x: cell_x.saturating_mul(TILE_FP) + INSET,
        y: origin_y,
        vx: 0,
        vy: RISE_VY,
        w: HIT_W,
        h: HIT_H,
        rise: RISE_FRAMES,
        active: false,
        alive: true,
        cell_x,
        cell_y,
        origin_y,
        placed: false,
        asset_id: String::new(),
    }
}

pub fn spawn_from_block(cell_x: i32, cell_y: i32, power: Power) -> ItemActor {
    let kind = if power == Power::Small {
        ItemKind::Mushroom
    } else {
        ItemKind::Flower
    };
    spawn_reward(cell_x, cell_y, kind)
}

pub fn step_item(item: &mut ItemActor, solids: &[Solid]) {
    if !item.alive {
        return;
    }
    if item.rise > 0 {
        item.vy = RISE_VY;
        item.y = item.y.saturating_add(item.vy);
        item.rise -= 1;
        if item.rise == 0 {
            item.active = true;
            if item.y + item.h > item.origin_y {
                item.y = item.origin_y - item.h;
            }
            item.vx = match item.kind {
                ItemKind::Mushroom | ItemKind::OneUp => MUSHROOM_VX,
                ItemKind::Star => STAR_VX,
                ItemKind::Flower | ItemKind::Plain => 0,
            };
            item.vy = 0;
        }
        return;
    }
    if !item.active || matches!(item.kind, ItemKind::Flower | ItemKind::Plain) {
        item.vx = 0;
        item.vy = 0;
        return;
    }
    let gravity = match item.kind {
        ItemKind::Star => STAR_GRAVITY,
        _ => MUSHROOM_GRAVITY,
    };
    if on_floor(item.rect(), solids) && item.vy >= 0 && item.kind != ItemKind::Star {
        item.vy = 0;
    } else {
        item.vy = item.vy.saturating_add(gravity).min(MAX_FALL);
    }
    let mut rect = item.rect();
    let hit = move_body(&mut rect, item.vx, item.vy, solids, false);
    item.x = rect.x;
    item.y = rect.y;
    if hit.wall {
        item.vx = -item.vx;
    }
    if hit.y == YHit::Floor {
        item.vy = if item.kind == ItemKind::Star {
            STAR_BOUNCE
        } else {
            0
        };
    } else if hit.y == YHit::Ceiling {
        item.vy = 0;
    }
}

pub fn try_fire(body: &mut Body, run_held: bool, fireballs: &mut Vec<Fireball>) {
    let pressed = run_held && !body.run_was_held;
    body.run_was_held = run_held;
    if !pressed || body.power != Power::Fire {
        return;
    }
    let alive = fireballs.iter().filter(|ball| ball.alive).count();
    if alive >= MAX_FIREBALLS {
        return;
    }
    let facing = if body.facing < 0 { -1 } else { 1 };
    fireballs.push(Fireball {
        x: if facing > 0 {
            body.x + body.w
        } else {
            body.x - 16 * FP
        },
        y: body.y + body.h / 3,
        vx: facing * FIRE_VX,
        vy: 0,
        w: 16 * FP,
        h: 16 * FP,
        alive: true,
    });
}

pub fn step_fireball(ball: &mut Fireball, solids: &[Solid]) {
    if !ball.alive {
        return;
    }
    ball.vy = ball.vy.saturating_add(FIRE_GRAVITY).min(MAX_FALL);
    let mut rect = ball.rect();
    let hit = move_body(&mut rect, ball.vx, ball.vy, solids, false);
    ball.x = rect.x;
    ball.y = rect.y;
    if hit.wall {
        ball.alive = false;
        return;
    }
    if hit.y == super::super::rules::smb1::collision::YHit::Floor {
        ball.vy = FIRE_BOUNCE;
    }
}

pub fn fireball_hits(ball: &mut Fireball, enemy: &mut EnemyActor) -> bool {
    if !ball.alive || !enemy.alive || !overlaps(ball.rect(), enemy.rect()) {
        return false;
    }
    if enemy.fire_immune() {
        return false;
    }
    ball.alive = false;
    if enemy.kind == super::super::rules::smb1::enemies::Kind::Bowser {
        enemy.hits = enemy.hits.saturating_add(1);
        if enemy.hits >= 5 {
            enemy.alive = false;
        }
    } else {
        enemy.alive = false;
    }
    true
}

pub fn collect(body: &mut Body, kind: ItemKind) -> bool {
    match kind {
        ItemKind::Mushroom => {
            if body.power == Power::Small {
                grow(body);
            }
            true
        }
        ItemKind::Flower => {
            if body.power == Power::Small {
                grow(body);
            }
            body.power = Power::Fire;
            true
        }
        ItemKind::Star => {
            body.starman_timer = STAR_FRAMES;
            true
        }
        ItemKind::OneUp => {
            body.lives = body.lives.saturating_add(1);
            true
        }
        ItemKind::Plain => true,
    }
}

fn grow(body: &mut Body) {
    body.power = Power::Super;
    body.w = HIT_W;
    if body.h < HIT_TALL {
        body.y -= HIT_TALL - body.h;
        body.h = HIT_TALL;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn super_damage_returns_to_small_and_starts_invulnerability() {
        let mut body = Body::fresh(0, 0);
        body.power = Power::Super;
        body.h = HIT_TALL;
        let hurt = apply_damage(&mut body);
        assert_eq!(hurt, Hurt::Shrink);
        assert_eq!(body.power, Power::Small);
        assert_eq!(body.invuln_timer, 120);
        assert_eq!(body.shrink_timer, 48);
        assert_eq!(body.h, HIT_H);
    }
}
