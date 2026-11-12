//! Original platformer physics shared by the desktop player and the browser player.
//! Nothing here is extracted from a cartridge.

use crate::project::{AssetKind, Layer, Level, Project};
use serde::{Deserialize, Serialize};

pub const TILE: f32 = 16.0;
const GRAVITY: f32 = 1500.0;
const ACCEL: f32 = 2800.0;
const MAX_RUN: f32 = 210.0;
const FRICTION: f32 = 2200.0;
const JUMP_V: f32 = -460.0;
const STOMP_V: f32 = -320.0;
const COYOTE: f32 = 0.10;
const JUMP_BUFFER: f32 = 0.12;
const MAX_FALL: f32 = 820.0;
const ENEMY_SPEED: f32 = 48.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub jump_held: bool,
    pub jump_pressed: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Aabb {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Aabb {
    pub fn intersects(self, other: Aabb) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    pub fn contains_point(self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Debug)]
pub struct Body {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub vx: f32,
    pub vy: f32,
    pub on_ground: bool,
    pub facing: i32,
    pub coyote: f32,
    pub jump_buf: f32,
    pub jump_cut_ok: bool,
}

impl Default for Body {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 12.0,
            h: 14.0,
            vx: 0.0,
            vy: 0.0,
            on_ground: false,
            facing: 1,
            coyote: 0.0,
            jump_buf: 0.0,
            jump_cut_ok: false,
        }
    }
}

impl Body {
    fn aabb(&self) -> Aabb {
        Aabb {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub dir: i32,
    pub alive: bool,
    pub asset_id: String,
}

impl Enemy {
    fn aabb(&self) -> Aabb {
        Aabb {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Events {
    pub jump: bool,
    pub stomp: bool,
    pub hurt: bool,
    pub goal: bool,
    pub collect: bool,
}

#[derive(Clone, Debug)]
pub struct Sim {
    pub player: Body,
    pub enemies: Vec<Enemy>,
    pub solids: Vec<Aabb>,
    pub hazards: Vec<Aabb>,
    pub goals: Vec<Aabb>,
    pub items: Vec<(Aabb, bool)>,
    pub spawn_x: f32,
    pub spawn_y: f32,
    pub level_w: f32,
    pub level_h: f32,
    pub won: bool,
    pub events: Events,
}

impl Sim {
    pub fn blank() -> Self {
        Self {
            player: Body::default(),
            enemies: Vec::new(),
            solids: Vec::new(),
            hazards: Vec::new(),
            goals: Vec::new(),
            items: Vec::new(),
            spawn_x: 0.0,
            spawn_y: 0.0,
            level_w: 512.0,
            level_h: 240.0,
            won: false,
            events: Events::default(),
        }
    }

    pub fn from_level(project: &Project, level: &Level) -> Self {
        let mut sim = Sim::blank();
        sim.level_w = level.w as f32 * TILE;
        sim.level_h = level.h as f32 * TILE;
        let asset = |id: &str| project.assets.iter().find(|a| a.id == id);
        for cell in &level.cells {
            let Some(asset) = asset(&cell.asset_id) else {
                continue;
            };
            let rect = Aabb {
                x: cell.x as f32 * TILE,
                y: cell.y as f32 * TILE,
                w: TILE,
                h: TILE,
            };
            match asset.kind {
                AssetKind::Terrain if asset.solid || cell.layer == Layer::Main => {
                    sim.solids.push(rect);
                }
                AssetKind::Enemy => {
                    sim.enemies.push(Enemy {
                        x: rect.x + 1.0,
                        y: rect.y + 2.0,
                        w: 14.0,
                        h: 14.0,
                        dir: if cell.x % 2 == 0 { 1 } else { -1 },
                        alive: true,
                        asset_id: asset.id.clone(),
                    });
                }
                AssetKind::Hazard => sim.hazards.push(rect),
                AssetKind::Goal => sim.goals.push(rect),
                AssetKind::Item => sim.items.push((rect, true)),
                _ => {}
            }
        }
        sim.player.x = level.player.x * TILE + 2.0;
        sim.player.y = level.player.y * TILE + 2.0;
        sim.player.w = 12.0;
        sim.player.h = 14.0;
        sim.spawn_x = sim.player.x;
        sim.spawn_y = sim.player.y;
        let feet = sim.player.y + sim.player.h + 0.5;
        let mid = sim.player.x + sim.player.w * 0.5;
        if sim.solids.iter().any(|s| s.contains_point(mid, feet)) {
            sim.player.on_ground = true;
            sim.player.coyote = COYOTE;
        }
        sim
    }

    pub fn step(&mut self, input: &Input, dt: f32) {
        self.events = Events::default();
        if self.won {
            return;
        }
        let dt = dt.clamp(0.0, 0.05);
        let player = &mut self.player;
        if input.left && !input.right {
            player.vx -= ACCEL * dt;
            player.facing = -1;
        } else if input.right && !input.left {
            player.vx += ACCEL * dt;
            player.facing = 1;
        } else if player.on_ground {
            let f = FRICTION * dt;
            if player.vx.abs() <= f {
                player.vx = 0.0;
            } else {
                player.vx -= player.vx.signum() * f;
            }
        }
        player.vx = player.vx.clamp(-MAX_RUN, MAX_RUN);

        if input.jump_pressed {
            player.jump_buf = JUMP_BUFFER;
        } else {
            player.jump_buf = (player.jump_buf - dt).max(0.0);
        }
        if player.jump_buf > 0.0 && (player.on_ground || player.coyote > 0.0) {
            player.vy = JUMP_V;
            player.on_ground = false;
            player.coyote = 0.0;
            player.jump_buf = 0.0;
            player.jump_cut_ok = true;
            self.events.jump = true;
        }
        if !input.jump_held && player.vy < 0.0 && player.jump_cut_ok {
            player.vy *= 0.45;
            player.jump_cut_ok = false;
        }

        player.vy = (player.vy + GRAVITY * dt).min(MAX_FALL);
        let prev_y = player.y;
        let prev_vy = player.vy;

        let dx = player.vx * dt;
        player.x += dx;
        resolve_axis(&mut player.x, player.y, player.w, player.h, &self.solids, true, dx);
        if dx != 0.0 && blocked_x(player.x, player.y, player.w, player.h, &self.solids, dx) {
            player.vx = 0.0;
        }

        let dy = player.vy * dt;
        player.y += dy;
        let landed = resolve_axis(&mut player.y, player.x, player.h, player.w, &self.solids, false, dy);
        if landed && dy > 0.0 {
            player.vy = 0.0;
            player.on_ground = true;
            player.coyote = COYOTE;
        } else if landed && dy < 0.0 {
            player.vy = 0.0;
            player.on_ground = false;
        } else {
            player.on_ground = false;
        }
        if !player.on_ground {
            player.coyote = (player.coyote - dt).max(0.0);
        }

        patrol_enemies(&mut self.enemies, &self.solids, dt);

        let mut stomped = false;
        let prev_bottom = prev_y + self.player.h;
        let player_box = self.player.aabb();
        for enemy in &mut self.enemies {
            if !enemy.alive {
                continue;
            }
            if player_box.intersects(enemy.aabb()) {
                let falling = prev_vy > 40.0;
                if falling && prev_bottom <= enemy.y + enemy.h * 0.55 {
                    enemy.alive = false;
                    stomped = true;
                    self.events.stomp = true;
                }
            }
        }
        if stomped {
            self.player.vy = STOMP_V;
            self.player.on_ground = false;
            self.player.jump_cut_ok = true;
        } else if self.enemies.iter().any(|e| e.alive && player_box.intersects(e.aabb())) {
            self.respawn();
            return;
        }

        let player_box = self.player.aabb();
        if self.hazards.iter().any(|h| player_box.intersects(*h)) {
            self.respawn();
            return;
        }
        if self.player.y > self.level_h + 8.0 {
            self.respawn();
            return;
        }
        for (rect, alive) in &mut self.items {
            if *alive && player_box.intersects(*rect) {
                *alive = false;
                self.events.collect = true;
            }
        }
        if self.goals.iter().any(|g| player_box.intersects(*g)) {
            self.won = true;
            self.events.goal = true;
            self.player.vx = 0.0;
        }
    }

    fn respawn(&mut self) {
        self.player.x = self.spawn_x;
        self.player.y = self.spawn_y;
        self.player.vx = 0.0;
        self.player.vy = 0.0;
        self.player.on_ground = true;
        self.player.coyote = COYOTE;
        self.events.hurt = true;
    }
}

fn patrol_enemies(enemies: &mut [Enemy], solids: &[Aabb], dt: f32) {
    for enemy in enemies.iter_mut() {
        if !enemy.alive {
            continue;
        }
        let dir = if enemy.dir >= 0 { 1.0 } else { -1.0 };
        let next = enemy.x + dir * ENEMY_SPEED * dt;
        let moved = Aabb {
            x: next,
            y: enemy.y,
            w: enemy.w,
            h: enemy.h,
        };
        let hit_wall = solids.iter().any(|s| moved.intersects(*s));
        let probe_x = if dir > 0.0 {
            next + enemy.w + 0.5
        } else {
            next - 0.5
        };
        let probe_y = enemy.y + enemy.h + 1.0;
        let ground_ahead = solids.iter().any(|s| s.contains_point(probe_x, probe_y));
        if hit_wall || !ground_ahead {
            enemy.dir = -enemy.dir;
        } else {
            enemy.x = next;
        }
    }
}

/// Resolve penetration on one axis.
/// When `horizontal` is true, `pos` is x and `other` is y, `size` is w, `other_size` is h.
/// When false, `pos` is y and the arguments are swapped so the same overlap math applies.
fn resolve_axis(
    pos: &mut f32,
    other: f32,
    size: f32,
    other_size: f32,
    solids: &[Aabb],
    horizontal: bool,
    delta: f32,
) -> bool {
    let mut hit = false;
    for solid in solids {
        let (x, y, w, h) = if horizontal {
            (*pos, other, size, other_size)
        } else {
            (other, *pos, other_size, size)
        };
        let body = Aabb { x, y, w, h };
        if !body.intersects(*solid) {
            continue;
        }
        hit = true;
        if horizontal {
            if delta > 0.0 {
                *pos = solid.x - size;
            } else if delta < 0.0 {
                *pos = solid.x + solid.w;
            } else {
                let push_left = (x + w) - solid.x;
                let push_right = (solid.x + solid.w) - x;
                if push_left < push_right {
                    *pos -= push_left;
                } else {
                    *pos += push_right;
                }
            }
        } else if delta > 0.0 {
            *pos = solid.y - size;
        } else if delta < 0.0 {
            *pos = solid.y + solid.h;
        } else {
            let push_up = (y + h) - solid.y;
            let push_down = (solid.y + solid.h) - y;
            if push_up < push_down {
                *pos -= push_up;
            } else {
                *pos += push_down;
            }
        }
    }
    hit
}

fn blocked_x(x: f32, y: f32, w: f32, h: f32, solids: &[Aabb], delta: f32) -> bool {
    let probe = if delta > 0.0 { x + w + 0.1 } else { x - 0.1 };
    let mid_y = y + h * 0.5;
    solids.iter().any(|s| s.contains_point(probe, mid_y))
}
