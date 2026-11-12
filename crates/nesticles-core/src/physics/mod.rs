//! Original platformer physics shared by the desktop player and the browser player.
//! Nothing here is extracted from a cartridge.
//!
//! [`CharacterPhysics::Nesticles`] is the float controller. [`CharacterPhysics::Smb1Nes`] is the
//! fixed-point 60 Hz preset in [`smb1_nes`].

pub(crate) mod collide;
pub(crate) mod fp;
mod smb1_nes;

pub use fp::{play_surface_scale, scale_virtual_px, FP_ONE, VIRTUAL_PX_AT_1080};

use crate::enemy_ai::{step_enemies, CameraPx, EnemyAiStyle, EnemyRuntime};
use crate::project::{AssetKind, Layer, Level, Project};
use collide::{from_aabbs, RectI};
use fp::{fp_to_px, px_to_fp};
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

/// Which character controller is driving play. New styles can be added beside these two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CharacterPhysics {
    #[default]
    Smb1Nes,
    Nesticles,
}

impl CharacterPhysics {
    pub fn label(self) -> &'static str {
        match self {
            Self::Smb1Nes => "Super Mario Bros. 1 (NES)",
            Self::Nesticles => "Nesticles",
        }
    }

    pub const ALL: [CharacterPhysics; 2] = [Self::Smb1Nes, Self::Nesticles];
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub jump_held: bool,
    pub jump_pressed: bool,
    /// Run (B). The SMB1 preset keeps a 10-frame buffer after this releases.
    pub run_held: bool,
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
    /// Visible world rectangle in pixels. Enemy activation uses it while the SMB1 preset runs.
    pub view: Aabb,
    smb1: smb1_nes::Smb1Body,
    enemy_rt: Vec<EnemyRuntime>,
    level_enemy_ai: EnemyAiStyle,
    project_enemy_ai: EnemyAiStyle,
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
            view: Aabb {
                x: 0.0,
                y: 0.0,
                w: 256.0,
                h: 240.0,
            },
            smb1: smb1_nes::Smb1Body::default(),
            enemy_rt: Vec::new(),
            level_enemy_ai: EnemyAiStyle::Smb1Nes,
            project_enemy_ai: EnemyAiStyle::Smb1Nes,
        }
    }

    pub fn from_level(project: &Project, level: &Level) -> Self {
        let mut sim = Sim::blank();
        sim.level_w = level.w as f32 * TILE;
        sim.level_h = level.h as f32 * TILE;
        sim.level_enemy_ai = level.enemy_ai;
        sim.project_enemy_ai = project.enemy_ai;
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
                    let enemy = Enemy {
                        x: rect.x + 1.0,
                        y: rect.y + 2.0,
                        w: 14.0,
                        h: 14.0,
                        dir: if cell.x % 2 == 0 { 1 } else { -1 },
                        alive: true,
                        asset_id: asset.id.clone(),
                    };
                    sim.enemy_rt.push(EnemyRuntime::spawn(
                        px_to_fp(enemy.x),
                        px_to_fp(enemy.y),
                        px_to_fp(enemy.w),
                        px_to_fp(enemy.h),
                        cell.archetype.clone(),
                        cell.enemy_ai,
                        cell.x,
                        cell.y,
                    ));
                    sim.enemies.push(enemy);
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
        sim.smb1.sync_from_float(&sim.player);
        sim.update_view(256.0, 240.0);
        sim
    }

    /// Float Nesticles step. Existing courses and tests use this path.
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
        resolve_axis(
            &mut player.x,
            player.y,
            player.w,
            player.h,
            &self.solids,
            true,
            dx,
        );
        if dx != 0.0 && blocked_x(player.x, player.y, player.w, player.h, &self.solids, dx) {
            player.vx = 0.0;
        }

        let dy = player.vy * dt;
        player.y += dy;
        let landed = resolve_axis(
            &mut player.y,
            player.x,
            player.h,
            player.w,
            &self.solids,
            false,
            dy,
        );
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
        self.resolve_contacts(prev_y, prev_vy);
    }

    /// Dispatch a play step. SMB1 consumes whole 1/60 slices from `dt` and ignores leftover time
    /// until the next call. Nesticles keeps the float controller, including its patrol enemies.
    pub fn step_preset(&mut self, preset: CharacterPhysics, input: &Input, dt: f32) {
        match preset {
            CharacterPhysics::Nesticles => self.step(input, dt),
            CharacterPhysics::Smb1Nes => self.step_smb1(input, dt),
        }
    }

    pub fn bind_character_physics(&mut self, preset: CharacterPhysics) {
        match preset {
            CharacterPhysics::Smb1Nes => {
                self.smb1.sync_from_float(&self.player);
                self.ensure_enemy_runtime();
                for (rt, enemy) in self.enemy_rt.iter_mut().zip(self.enemies.iter()) {
                    rt.x = px_to_fp(enemy.x);
                    rt.y = px_to_fp(enemy.y);
                    rt.vx = 0;
                    rt.vy = 0;
                    rt.awake = false;
                    rt.on_ground = false;
                    rt.patrol_dir = -1;
                }
            }
            CharacterPhysics::Nesticles => {
                self.smb1.ready = false;
            }
        }
    }

    pub fn set_enemy_styles(&mut self, level: EnemyAiStyle, global: EnemyAiStyle) {
        self.level_enemy_ai = level;
        self.project_enemy_ai = global;
    }

    pub fn set_enemy_override(&mut self, cell_x: i32, cell_y: i32, style: Option<EnemyAiStyle>) {
        for enemy in &mut self.enemy_rt {
            if enemy.cell_x == cell_x && enemy.cell_y == cell_y {
                enemy.style_override = style;
            }
        }
    }

    /// Point the camera at the player. A view larger than the course is centered.
    pub fn update_view(&mut self, width: f32, height: f32) {
        let w = width.max(1.0);
        let h = height.max(1.0);
        let x = if w >= self.level_w {
            (self.level_w - w) * 0.5
        } else {
            (self.player.x - w * 0.35).clamp(0.0, self.level_w - w)
        };
        let y = if h >= self.level_h {
            (self.level_h - h) * 0.5
        } else {
            (self.player.y - h * 0.55).clamp(0.0, self.level_h - h)
        };
        self.view = Aabb { x, y, w, h };
    }

    fn step_smb1(&mut self, input: &Input, dt: f32) {
        if self.won {
            return;
        }
        if !self.smb1.ready {
            self.smb1.sync_from_float(&self.player);
        }
        self.ensure_enemy_runtime();
        let ticks = self.smb1.consume(dt);
        let solids = from_aabbs(&self.solids);
        self.events = Events::default();
        let mut input = *input;
        for i in 0..ticks {
            if i > 0 {
                input.jump_pressed = false;
            }
            let prev_y = self.player.y;
            let prev_vy = self.player.vy;
            let jumped = smb1_nes::tick(&mut self.smb1, &input, &solids);
            self.smb1.write_to(&mut self.player);
            let view_w = self.view.w;
            let view_h = self.view.h;
            self.update_view(view_w, view_h);
            self.step_smb1_enemies(&solids);
            if jumped {
                self.events.jump = true;
            }
            self.resolve_contacts(prev_y, prev_vy);
            if self.events.stomp || self.events.goal {
                self.smb1
                    .apply_pixel_velocity(self.player.vx, self.player.vy);
                self.smb1.on_ground = false;
                self.smb1.jumping = false;
                if self.events.goal {
                    self.smb1.vx = 0;
                    self.smb1.vy = 0;
                }
            }
            if self.events.hurt {
                self.smb1.sync_from_float(&self.player);
            }
            if self.events.hurt || self.won {
                break;
            }
        }
    }

    fn step_smb1_enemies(&mut self, solids: &[RectI]) {
        let alive: Vec<bool> = self.enemies.iter().map(|enemy| enemy.alive).collect();
        let camera = CameraPx {
            x: self.view.x,
            y: self.view.y,
            w: self.view.w,
            h: self.view.h,
        };
        step_enemies(
            &mut self.enemy_rt,
            &alive,
            solids,
            camera,
            self.level_enemy_ai,
            self.project_enemy_ai,
        );
        for (enemy, rt) in self.enemies.iter_mut().zip(self.enemy_rt.iter()) {
            if !enemy.alive {
                continue;
            }
            enemy.x = fp_to_px(rt.x);
            enemy.y = fp_to_px(rt.y);
            enemy.dir = if rt.patrol_dir < 0 { -1 } else { 1 };
        }
    }

    fn ensure_enemy_runtime(&mut self) {
        if self.enemy_rt.len() == self.enemies.len() {
            return;
        }
        self.enemy_rt.clear();
        for enemy in &self.enemies {
            self.enemy_rt.push(EnemyRuntime::spawn(
                px_to_fp(enemy.x),
                px_to_fp(enemy.y),
                px_to_fp(enemy.w),
                px_to_fp(enemy.h),
                crate::enemy_ai::EnemyArchetype::Walker,
                None,
                (enemy.x / TILE) as i32,
                (enemy.y / TILE) as i32,
            ));
        }
    }

    fn resolve_contacts(&mut self, prev_y: f32, prev_vy: f32) {
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
        } else if self
            .enemies
            .iter()
            .any(|e| e.alive && player_box.intersects(e.aabb()))
        {
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
        self.smb1.sync_from_float(&self.player);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smb1_lands_on_the_float_sim_floor() {
        let mut sim = Sim::blank();
        sim.solids.push(Aabb {
            x: 0.0,
            y: 160.0,
            w: 400.0,
            h: 16.0,
        });
        sim.player.x = 32.0;
        sim.player.y = 20.0;
        sim.player.w = 12.0;
        sim.player.h = 14.0;
        sim.bind_character_physics(CharacterPhysics::Smb1Nes);
        for _ in 0..180 {
            sim.step_preset(CharacterPhysics::Smb1Nes, &Input::default(), 1.0 / 60.0);
        }
        assert!(sim.player.on_ground, "y={}", sim.player.y);
        let feet = sim.player.y + sim.player.h;
        assert!((feet - 160.0).abs() < 0.02, "feet {feet}");
    }

    #[test]
    fn smb1_walker_stays_dormant_until_the_camera_reaches_it() {
        let mut sim = Sim::blank();
        sim.solids.push(Aabb {
            x: 0.0,
            y: 160.0,
            w: 800.0,
            h: 16.0,
        });
        sim.player.x = 32.0;
        sim.player.y = 146.0;
        sim.player.w = 12.0;
        sim.player.h = 14.0;
        sim.level_w = 800.0;
        sim.level_h = 240.0;
        sim.update_view(256.0, 240.0);
        sim.enemies.push(Enemy {
            x: 480.0,
            y: 146.0,
            w: 14.0,
            h: 14.0,
            dir: 1,
            alive: true,
            asset_id: "far".into(),
        });
        sim.enemies.push(Enemy {
            x: 120.0,
            y: 146.0,
            w: 14.0,
            h: 14.0,
            dir: 1,
            alive: true,
            asset_id: "near".into(),
        });
        sim.ensure_enemy_runtime();
        let far_x = sim.enemies[0].x;
        let near_x = sim.enemies[1].x;
        for _ in 0..30 {
            sim.step_preset(CharacterPhysics::Smb1Nes, &Input::default(), 1.0 / 60.0);
        }
        assert!(
            (sim.enemies[0].x - far_x).abs() < 0.01,
            "dormant enemy moved to {}",
            sim.enemies[0].x
        );
        assert!(
            sim.enemies[1].x < near_x - 10.0,
            "walker should move left, x={}",
            sim.enemies[1].x
        );
        assert!(!sim.enemy_rt[0].awake);
        assert!(sim.enemy_rt[1].awake);
        assert_eq!(sim.enemy_rt[1].vx, -2048);
    }

    #[test]
    fn virtual_pixel_scale_is_four_and_a_half() {
        assert!((scale_virtual_px(16.0) - 72.0).abs() < 0.001);
        let large = play_surface_scale(1280.0, 720.0, 512.0, 240.0);
        assert!((large - VIRTUAL_PX_AT_1080).abs() < 0.001);
        let small = play_surface_scale(480.0, 270.0, 512.0, 240.0);
        assert!(small < 2.0, "small panels fit the course, scale={small}");
    }
}
