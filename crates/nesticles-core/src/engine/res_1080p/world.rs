//! One 1080p course. The stored grid stays 16px cells; this converts each cell
//! into a 72×72 tile while the field is running.
//!
//! `from_level` builds actors from an explicit blueprint when the cell has one.
//! Asset names and kinds do not override that record. A cell with no blueprint
//! still uses [`interpret_cell`] only while the project has not been cut over.
//! After [`crate::blueprint::ensure_legacy_blueprints`], a missing id is decorative.
//! `tick` does not classify cells again.

use super::character_physics::{apply_stomp, params_for, step_motion, Body, Power};
use super::powerups::smb1::{
    apply_damage, collect, spawn_from_block, spawn_reward, step_fireball, step_item, try_fire,
    Fireball, Hurt, ItemActor, ItemKind,
};
use super::rules::smb1::collision::{
    bump_offset, move_body, on_floor, BlockContents, BlockKind, MoveHit, Solid, YHit,
};
use super::rules::smb1::enemies::{self, kind_of, EnemyActor, Kind};
use super::rules::smb1::hazards::{feet_overlap, step_firebars, Hazard, HazardKind};
use super::{
    overlaps, CharacterId, Rect, FP, HIT_H, HIT_TALL, HIT_W, INSET, SCREEN_H_FP, SCREEN_W_FP,
    TILE_FP,
};
use crate::blueprint::{LegacyBlock, LegacyEnemy, LegacyHazard, LegacyItem, LegacyRole};
use crate::engine::Pad;
use crate::project::PlacementWarp;
use crate::project::{
    Asset, AssetKind, BlockTag, Cell, HazardTag, ItemTag, Layer, Level, Project, WarpTarget,
};
use crate::transition::{named_exit, NamedExit};

#[derive(Clone, Debug, Default)]
pub struct Events {
    pub jump: bool,
    pub stomp: bool,
    pub hurt: bool,
    pub goal: bool,
    pub collect: bool,
}

#[derive(Clone, Debug)]
pub struct World {
    pub character: CharacterId,
    pub body: Body,
    pub solids: Vec<Solid>,
    pub enemies: Vec<EnemyActor>,
    pub hazards: Vec<Hazard>,
    pub items: Vec<ItemActor>,
    pub fireballs: Vec<Fireball>,
    pub goals: Vec<Rect>,
    pub camera_x: i32,
    pub camera_y: i32,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub level_w: i32,
    pub level_h: i32,
    pub won: bool,
    pub events: Events,
    pub warp_lock: i32,
    /// False after an arrival until the player leaves every pipe opening.
    pub pipe_armed: bool,
    pub pending_level: String,
    pub pending_x: i32,
    pub pending_y: i32,
    pub pending_facing: i32,
    pub pending: bool,
    pub source_enemies: usize,
    pub bump_marks: Vec<(i32, i32, i32)>,
    pub hidden: Vec<(i32, i32)>,
    pub hide_player: bool,
    /// Explicit blueprint ids that did not resolve. The cell reference is left
    /// on the project. No actor is built for that placement, and the asset name
    /// is not consulted.
    pub blueprint_faults: Vec<String>,
}

impl World {
    pub fn from_level(project: &Project, level: &Level, character: CharacterId) -> Self {
        let mut solids = Vec::new();
        let mut enemies = Vec::new();
        let mut hazards = Vec::new();
        let mut items = Vec::new();
        let mut goals = Vec::new();
        let mut blueprint_faults = Vec::new();
        for cell in &level.cells {
            let Some(asset) = project
                .assets
                .iter()
                .find(|asset| asset.id == cell.asset_id)
            else {
                if cell.blueprint_id.is_some() {
                    blueprint_faults.push(missing_asset_fault(cell));
                }
                continue;
            };
            let read = match resolve_placement(project, cell, asset) {
                Placement::Skip => continue,
                Placement::Fault(message) => {
                    blueprint_faults.push(message);
                    continue;
                }
                Placement::Read(mut read) => {
                    apply_placement_warp(&mut read, cell);
                    match named_exit(project, &level.id, cell) {
                        NamedExit::Absent => {}
                        NamedExit::Ready(exit) => {
                            read.warp_set = true;
                            read.warp_level = exit.level_id;
                            read.warp_x = exit.x;
                            read.warp_y = exit.y;
                            read.arrival_facing = exit.facing;
                        }
                        NamedExit::Fault(message) => {
                            read.warp_set = false;
                            read.warp_level.clear();
                            read.warp_x = 0;
                            read.warp_y = 0;
                            blueprint_faults.push(message);
                        }
                    }
                    read
                }
            };
            if read.audio {
                continue;
            }
            if read.podoboo {
                enemies.push(EnemyActor::at_cell(
                    Kind::Podoboo,
                    cell.x,
                    cell.y,
                    asset.id.clone(),
                ));
                continue;
            }
            if let Some(kind) = read.enemy {
                enemies.push(EnemyActor::at_cell(kind, cell.x, cell.y, asset.id.clone()));
                continue;
            }
            if read.goal {
                goals.push(cell_rect(cell.x, cell.y));
                continue;
            }
            let holds_reward = matches!(read.block, Some(BlockKind::Question | BlockKind::Brick));
            if let Some(kind) = read.item {
                if !holds_reward {
                    items.push(placed_item(cell.x, cell.y, &asset.id, kind));
                }
            }
            if let Some(block) = read.block {
                let mut solid = Solid::at_cell(cell.x, cell.y, block);
                if holds_reward {
                    solid.contents = if read.coin {
                        Some(BlockContents::Coin)
                    } else {
                        read.item.map(BlockContents::Item)
                    };
                }
                solids.push(solid);
            }
            if let Some(kind) = read.hazard {
                hazards.push(Hazard {
                    kind,
                    x: cell.x.saturating_mul(TILE_FP),
                    y: cell.y.saturating_mul(TILE_FP),
                    w: TILE_FP,
                    h: TILE_FP,
                    cell_x: cell.x,
                    cell_y: cell.y,
                    awake: false,
                    warp_level: read.warp_level.clone(),
                    warp_x: read.warp_x,
                    warp_y: read.warp_y,
                    warp_set: read.warp_set,
                    arrival_facing: read.arrival_facing,
                    angle: 0,
                    balls: read.balls,
                });
            }
        }
        let spawn_x = units_from_cell(level.player.x) + INSET;
        let spawn_y = units_from_cell(level.player.y);
        let mut body = Body::fresh(spawn_x, spawn_y);
        body.on_ground = on_floor(player_rect(&body), &solids);
        let source_enemies = enemies.len();
        let mut world = Self {
            character,
            body,
            solids,
            enemies,
            hazards,
            items,
            fireballs: Vec::new(),
            goals,
            camera_x: 0,
            camera_y: 0,
            spawn_x,
            spawn_y,
            level_w: (level.w as i32).saturating_mul(TILE_FP),
            level_h: (level.h as i32).saturating_mul(TILE_FP),
            won: false,
            events: Events::default(),
            warp_lock: 0,
            pipe_armed: true,
            pending_level: String::new(),
            pending_x: 0,
            pending_y: 0,
            pending_facing: 1,
            pending: false,
            source_enemies,
            bump_marks: Vec::new(),
            hidden: Vec::new(),
            hide_player: false,
            blueprint_faults,
        };
        world.update_camera();
        world
    }

    pub fn place(&mut self, cell_x: i32, cell_y: i32) {
        self.place_facing(cell_x, cell_y, self.body.facing);
    }

    /// Put the player on a cell. Velocity from the trip is cleared. Power and
    /// the rest of the running world stay as they are. The pipe stays shut
    /// until the player walks off its opening, so a return pipe cannot fire
    /// on the arrival frame.
    pub fn place_facing(&mut self, cell_x: i32, cell_y: i32, facing: i32) {
        self.body.x = cell_x.saturating_mul(TILE_FP) + INSET;
        self.body.y = cell_y.saturating_mul(TILE_FP);
        self.body.vx = 0;
        self.body.vy = 0;
        self.body.facing = if facing < 0 { -1 } else { 1 };
        self.body.climbing = false;
        self.body.spring_timer = 0;
        self.warp_lock = 30;
        self.pipe_armed = false;
        self.pending = false;
        self.update_camera();
    }

    pub fn tick(&mut self, pad: &Pad) {
        self.events = Events::default();
        if self.body.starman_timer > 0 {
            self.body.starman_timer -= 1;
        }
        if self.body.invuln_timer > 0 {
            self.body.invuln_timer -= 1;
        }
        if self.body.shrink_timer > 0 {
            self.body.shrink_timer -= 1;
        }
        self.hide_player = self.body.invuln_timer > 0 && (self.body.invuln_timer / 4) % 2 == 0;
        if self.won {
            return;
        }
        self.update_camera();
        let prev_y = self.body.y;
        let prev_vy = self.body.vy;
        self.step_player(pad);
        for item in &mut self.items {
            step_item(item, &self.solids);
        }
        enemies::tick(
            &mut self.enemies,
            &self.solids,
            self.body.x,
            self.body.y,
            self.camera_x,
            self.camera_y,
        );
        self.step_shots();
        self.contacts(pad, prev_y, prev_vy);
        self.pickups();
        self.hazard_touch();
        self.goals();
        if self.body.y > SCREEN_H_FP {
            self.defeat();
        }
        self.tick_bumps();
    }

    fn step_player(&mut self, pad: &Pad) {
        self.sync_hitbox(pad);
        let params = params_for(self.character);
        let falling = self.body.vy > 0;
        if self.body.climbing {
            if pad.jump_pressed && !pad.up {
                let facing = if self.body.facing < 0 { -1 } else { 1 };
                self.body.climbing = false;
                self.body.vx = facing * 18432;
                self.body.vy = -55296;
                self.body.is_jumping = true;
                self.body.on_ground = false;
                self.events.jump = true;
            } else {
                self.body.vx = 0;
                self.body.vy = if pad.up && !pad.down {
                    -18432
                } else if pad.down && !pad.up {
                    36864
                } else {
                    0
                };
            }
        } else if self.body.spring_timer > 0 {
            self.body.spring_timer -= 1;
            self.body.vx = 0;
            if self.body.spring_timer == 0 {
                self.body.vy = if pad.jump_held { -129024 } else { -73728 };
                self.body.on_ground = false;
                self.body.is_jumping = true;
                self.events.jump = true;
            } else {
                self.body.vy = 0;
            }
        } else if step_motion(&mut self.body, pad, params) {
            self.events.jump = true;
        }
        let mut rect = player_rect(&self.body);
        let hit = move_body(&mut rect, self.body.vx, self.body.vy, &self.solids, true);
        self.body.x = rect.x;
        self.body.y = rect.y;
        if self.body.spring_timer > 0 {
            self.body.on_ground = true;
            self.body.vy = 0;
        } else {
            self.apply_y(hit, params.hover);
        }
        if falling || hit.y == YHit::Floor {
            self.try_spring();
        }
        if self.body.climbing && !self.touching(HazardKind::Vine) {
            self.body.climbing = false;
        }
        self.try_vine(pad);
        self.try_pipe(pad);
        try_fire(&mut self.body, pad.run_held, &mut self.fireballs);
    }

    fn apply_y(&mut self, hit: MoveHit, hover: bool) {
        match hit.y {
            YHit::Floor => {
                self.body.on_ground = true;
                self.body.vy = 0;
                self.body.is_jumping = false;
                if hover {
                    self.body.hover_timer = 90;
                }
            }
            YHit::Ceiling => {
                self.body.vy = 0;
                self.body.on_ground = false;
                self.body.is_jumping = false;
                self.hit_block(hit.ceiling);
            }
            YHit::None => {
                if self.body.vy != 0 || !on_floor(player_rect(&self.body), &self.solids) {
                    self.body.on_ground = false;
                }
            }
        }
    }

    fn hit_block(&mut self, index: Option<usize>) {
        let Some(index) = index else {
            return;
        };
        if self.solids[index].broken {
            return;
        }
        let power = self.body.power;
        let cell_x = self.solids[index].cell_x;
        let cell_y = self.solids[index].cell_y;
        match self.solids[index].block {
            BlockKind::Brick => {
                if self.solids[index].contents.is_some() {
                    let contents = self.solids[index].contents.take();
                    self.solids[index].used = true;
                    self.solids[index].bump = 8;
                    if let Some(contents) = contents {
                        self.release_contents(cell_x, cell_y, contents);
                    }
                    self.defeat_on(cell_x, cell_y);
                } else if matches!(power, Power::Super | Power::Fire) {
                    self.solids[index].broken = true;
                    self.hidden.push((cell_x, cell_y));
                    self.defeat_on(cell_x, cell_y);
                } else if self.solids[index].bump == 0 {
                    self.solids[index].bump = 8;
                    self.defeat_on(cell_x, cell_y);
                }
            }
            BlockKind::Question if !self.solids[index].used => {
                let contents = self.solids[index].contents.take();
                self.solids[index].used = true;
                self.solids[index].block = BlockKind::Solid;
                self.solids[index].bump = 8;
                if let Some(contents) = contents {
                    self.release_contents(cell_x, cell_y, contents);
                } else {
                    self.items.push(spawn_from_block(cell_x, cell_y, power));
                }
                self.defeat_on(cell_x, cell_y);
            }
            BlockKind::Coin if !self.solids[index].used => {
                self.solids[index].used = true;
                self.solids[index].bump = 8;
                self.events.collect = true;
                self.defeat_on(cell_x, cell_y);
            }
            _ => {}
        }
    }

    fn release_contents(&mut self, cell_x: i32, cell_y: i32, contents: BlockContents) {
        match contents {
            BlockContents::Coin => self.events.collect = true,
            BlockContents::Item(kind) => {
                self.items.push(spawn_reward(cell_x, cell_y, kind));
            }
        }
    }

    fn defeat_on(&mut self, cell_x: i32, cell_y: i32) {
        let top = cell_y.saturating_mul(TILE_FP);
        let left = cell_x.saturating_mul(TILE_FP);
        let right = left + TILE_FP;
        for enemy in &mut self.enemies {
            if !enemy.alive {
                continue;
            }
            let feet = enemy.y + enemy.h;
            let over = feet >= top - 8 * FP && feet <= top + 12 * FP;
            if over && enemy.x < right && enemy.x + enemy.w > left {
                enemy.alive = false;
            }
        }
    }

    fn try_spring(&mut self) {
        if self.body.spring_timer > 0 || self.body.climbing {
            return;
        }
        let body = player_rect(&self.body);
        for hazard in &self.hazards {
            if hazard.kind == HazardKind::Spring && feet_overlap(body, hazard) {
                self.body.spring_timer = 3;
                self.body.vy = 0;
                self.body.vx = 0;
                self.body.on_ground = true;
                self.body.y = hazard.y - self.body.h;
                return;
            }
        }
    }

    fn try_vine(&mut self, pad: &Pad) {
        if self.body.climbing || (!pad.up && !pad.down) {
            return;
        }
        let body = player_rect(&self.body);
        for hazard in &self.hazards {
            if hazard.kind == HazardKind::Vine && overlaps(body, hazard.rect()) {
                self.body.climbing = true;
                self.body.vx = 0;
                self.body.vy = 0;
                self.body.on_ground = false;
                self.body.is_jumping = false;
                self.body.x = hazard.x + TILE_FP / 2 - self.body.w / 2;
                return;
            }
        }
    }

    fn try_pipe(&mut self, pad: &Pad) {
        if self.warp_lock > 0 {
            self.warp_lock -= 1;
        }
        let body = player_rect(&self.body);
        let in_opening = self.hazards.iter().any(|hazard| {
            hazard.is_pipe() && hazard.warp_set && center_in(body, hazard.entry_rect())
        });
        if !self.pipe_armed {
            if !in_opening {
                self.pipe_armed = true;
            } else {
                return;
            }
        }
        if self.warp_lock > 0 {
            return;
        }
        let mut chosen: Option<(String, i32, i32, i32)> = None;
        for hazard in &self.hazards {
            if !hazard.warp_set || !center_in(body, hazard.entry_rect()) {
                continue;
            }
            let open = match hazard.kind {
                HazardKind::PipeDown => pad.down,
                HazardKind::PipeUp => pad.up,
                HazardKind::PipeLeft => pad.left,
                HazardKind::PipeRight => pad.right,
                _ => false,
            };
            if !open {
                continue;
            }
            chosen = Some((
                hazard.warp_level.clone(),
                hazard.warp_x,
                hazard.warp_y,
                hazard.arrival_facing,
            ));
            break;
        }
        let Some((level, x, y, facing)) = chosen else {
            return;
        };
        if level.is_empty() {
            let cells_w = self.level_w / TILE_FP;
            let cells_h = self.level_h / TILE_FP;
            if x < 0 || y < 0 || x >= cells_w || y >= cells_h {
                return;
            }
        }
        self.warp_lock = 30;
        self.pipe_armed = false;
        self.body.vx = 0;
        self.body.vy = 0;
        self.body.facing = if facing < 0 { -1 } else { 1 };
        if level.is_empty() {
            self.body.x = x.saturating_mul(TILE_FP) + INSET;
            self.body.y = y.saturating_mul(TILE_FP);
            self.update_camera();
        } else {
            self.pending = true;
            self.pending_level = level;
            self.pending_x = x;
            self.pending_y = y;
            self.pending_facing = if facing < 0 { -1 } else { 1 };
        }
    }

    fn touching(&self, kind: HazardKind) -> bool {
        let body = player_rect(&self.body);
        self.hazards
            .iter()
            .any(|hazard| hazard.kind == kind && overlaps(body, hazard.rect()))
    }

    fn step_shots(&mut self) {
        for ball in &mut self.fireballs {
            step_fireball(ball, &self.solids);
        }
        let mut balls = std::mem::take(&mut self.fireballs);
        for ball in &mut balls {
            for enemy in &mut self.enemies {
                super::powerups::smb1::fireball_hits(ball, enemy);
            }
        }
        balls.retain(|ball| ball.alive);
        self.fireballs = balls;
        self.enemies.retain(|enemy| enemy.source || enemy.alive);
    }

    fn contacts(&mut self, pad: &Pad, prev_y: i32, prev_vy: i32) {
        let star = self.body.starman_timer > 0;
        let player = player_rect(&self.body);
        let height = self.body.h;
        let player_x = self.body.x;
        let mut hurt = false;
        let mut stomped = false;
        for enemy in &mut self.enemies {
            if !enemy.alive || !enemy.awake || !overlaps(player, enemy.rect()) {
                continue;
            }
            if star {
                enemy.alive = false;
                stomped = true;
                continue;
            }
            if enemy.always_hurts() {
                hurt = true;
                continue;
            }
            let from_above = prev_vy > 0 && prev_y + height <= enemy.y + enemy.h / 2;
            if enemy.kind == Kind::Shell {
                if from_above {
                    enemy.kicked = false;
                    enemy.vx = 0;
                    enemy.timer = 0;
                    stomped = true;
                } else if !enemy.kicked {
                    let dir = if player_x + HIT_W / 2 < enemy.x + enemy.w / 2 {
                        1
                    } else {
                        -1
                    };
                    enemy.kicked = true;
                    enemy.facing = dir;
                    enemy.vx = dir * 55296;
                    enemy.timer = 0;
                } else {
                    hurt = true;
                }
                continue;
            }
            if from_above {
                crush(enemy);
                stomped = true;
            } else {
                hurt = true;
            }
        }
        if stomped {
            apply_stomp(&mut self.body, pad.jump_held);
            self.events.stomp = true;
        }
        if hurt {
            self.hurt();
        }
    }

    fn pickups(&mut self) {
        let player = player_rect(&self.body);
        let mut picked = Vec::new();
        for item in &mut self.items {
            if item.alive && item.active && overlaps(player, item.rect()) {
                item.alive = false;
                picked.push(item.kind);
            }
        }
        for kind in picked {
            collect(&mut self.body, kind);
            self.events.collect = true;
        }
    }

    fn hazard_touch(&mut self) {
        let body = player_rect(&self.body);
        let mut hurt = step_firebars(&mut self.hazards, body, self.camera_x, self.camera_y);
        for hazard in &self.hazards {
            if hazard.kind == HazardKind::Contact && overlaps(body, hazard.rect()) {
                hurt = true;
            }
        }
        if hurt {
            self.hurt();
        }
    }

    fn goals(&mut self) {
        let body = player_rect(&self.body);
        if self.goals.iter().any(|goal| overlaps(body, *goal)) {
            self.won = true;
            self.events.goal = true;
            self.body.vx = 0;
            self.body.vy = 0;
        }
    }

    fn hurt(&mut self) {
        match apply_damage(&mut self.body) {
            Hurt::Ignored => {}
            Hurt::Shrink => self.events.hurt = true,
            Hurt::Defeat => self.defeat(),
        }
    }

    fn defeat(&mut self) {
        self.body.power = Power::Small;
        self.body.w = HIT_W;
        self.body.h = HIT_H;
        self.body.x = self.spawn_x;
        self.body.y = self.spawn_y;
        self.body.vx = 0;
        self.body.vy = 0;
        self.body.on_ground = true;
        self.body.is_jumping = false;
        self.body.climbing = false;
        self.body.spring_timer = 0;
        self.body.crouching = false;
        self.body.starman_timer = 0;
        self.body.invuln_timer = 0;
        self.body.shrink_timer = 0;
        self.body.lives = (self.body.lives - 1).max(0);
        self.fireballs.clear();
        self.events.hurt = true;
    }

    fn sync_hitbox(&mut self, pad: &Pad) {
        let tall = matches!(self.body.power, Power::Super | Power::Fire);
        let crouch = tall && pad.down && self.body.on_ground && !self.body.climbing;
        self.body.crouching = crouch;
        let h = if tall && !crouch { HIT_TALL } else { HIT_H };
        if h != self.body.h {
            self.body.y -= h - self.body.h;
            self.body.h = h;
        }
        self.body.w = HIT_W;
    }

    fn tick_bumps(&mut self) {
        self.bump_marks.clear();
        for solid in &mut self.solids {
            if solid.bump > 0 {
                self.bump_marks
                    .push((solid.cell_x, solid.cell_y, bump_offset(solid.bump)));
                solid.bump -= 1;
            }
        }
    }

    fn update_camera(&mut self) {
        let max_x = (self.level_w - SCREEN_W_FP).max(0);
        let max_y = (self.level_h - SCREEN_H_FP).max(0);
        let focus_x = self.body.x - SCREEN_W_FP * 35 / 100;
        let focus_y = self.body.y - SCREEN_H_FP * 55 / 100;
        self.camera_x = focus_x.clamp(0, max_x);
        self.camera_y = focus_y.clamp(0, max_y);
    }
}

fn crush(enemy: &mut EnemyActor) {
    match enemy.kind {
        Kind::GreenPara => {
            enemy.kind = Kind::GreenKoopa;
            enemy.vy = 0;
        }
        Kind::RedPara => {
            enemy.kind = Kind::RedKoopa;
            enemy.vy = 0;
        }
        Kind::GreenKoopa | Kind::RedKoopa | Kind::Buzzy => {
            enemy.buzzy = enemy.kind == Kind::Buzzy;
            enemy.kind = Kind::Shell;
            enemy.kicked = false;
            enemy.vx = 0;
            enemy.timer = 0;
        }
        _ => enemy.alive = false,
    }
}

fn player_rect(body: &Body) -> Rect {
    Rect {
        x: body.x,
        y: body.y,
        w: body.w,
        h: body.h,
    }
}

/// What `World::from_level` builds from one cell. Blueprint migration records this
/// reading; it is not a second ruleset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CellRead {
    pub audio: bool,
    pub podoboo: bool,
    pub enemy: Option<Kind>,
    pub goal: bool,
    pub item: Option<ItemKind>,
    pub block: Option<BlockKind>,
    pub hazard: Option<HazardKind>,
    pub balls: i32,
    pub coin: bool,
    pub arrival_facing: i32,
    pub warp_set: bool,
    pub warp_level: String,
    pub warp_x: i32,
    pub warp_y: i32,
}

pub(crate) fn interpret_cell(asset: &Asset, cell: &crate::project::Cell) -> CellRead {
    let (warp_set, warp_level, warp_x, warp_y) = warp_of(cell, asset);
    let mut read = CellRead {
        audio: false,
        podoboo: false,
        enemy: None,
        goal: false,
        item: None,
        block: None,
        hazard: None,
        balls: 0,
        coin: false,
        arrival_facing: 1,
        warp_set,
        warp_level,
        warp_x,
        warp_y,
    };
    if asset.kind.is_audio() {
        read.audio = true;
        return read;
    }
    if is_podoboo(asset) {
        read.podoboo = true;
        return read;
    }
    if asset.kind == AssetKind::Enemy {
        read.enemy = Some(kind_of(&cell.archetype, &asset.name));
        return read;
    }
    if asset.kind == AssetKind::Goal {
        read.goal = true;
        return read;
    }
    if asset.kind == AssetKind::Item {
        read.item = Some(resolve_item(asset));
    }
    let solid_terrain =
        asset.kind == AssetKind::Terrain && (asset.solid || cell.layer == Layer::Main);
    let hazard_kind = resolve_hazard(asset);
    let pipe = matches!(
        hazard_kind,
        Some(HazardKind::PipeDown | HazardKind::PipeRight)
    );
    if solid_terrain {
        read.block = Some(resolve_block(asset));
    } else if pipe {
        read.block = Some(BlockKind::Solid);
    }
    if let Some(kind) = hazard_kind {
        read.hazard = Some(kind);
        if kind == HazardKind::FireBar {
            read.balls = firebar_balls(asset);
        }
    }
    read
}

fn cell_rect(x: i32, y: i32) -> Rect {
    Rect {
        x: x.saturating_mul(TILE_FP),
        y: y.saturating_mul(TILE_FP),
        w: TILE_FP,
        h: TILE_FP,
    }
}

fn units_from_cell(cell: f32) -> i32 {
    (f64::from(cell) * f64::from(TILE_FP)).round() as i32
}

fn placed_item(cell_x: i32, cell_y: i32, asset_id: &str, kind: ItemKind) -> ItemActor {
    let origin_y = cell_y.saturating_mul(TILE_FP);
    let vx = match kind {
        ItemKind::Mushroom | ItemKind::OneUp => 18432,
        ItemKind::Star => 23040,
        ItemKind::Flower | ItemKind::Plain => 0,
    };
    ItemActor {
        kind,
        x: cell_x.saturating_mul(TILE_FP) + INSET,
        y: origin_y,
        vx,
        vy: 0,
        w: HIT_W,
        h: HIT_H,
        rise: 0,
        active: true,
        alive: true,
        cell_x,
        cell_y,
        origin_y,
        placed: true,
        asset_id: asset_id.to_string(),
    }
}

enum Placement {
    /// No actor. Decorative cells and inert roles use this.
    Skip,
    Read(CellRead),
    /// The blueprint id stays on the cell. Name inference is not a fallback.
    Fault(String),
}

/// Blueprint first. Name and kind inference remains only for a project that
/// has never been cut over, and only when the cell has no blueprint id.
fn resolve_placement(project: &Project, cell: &Cell, asset: &Asset) -> Placement {
    let Some(id) = cell.blueprint_id.as_deref() else {
        if project.blueprint_cutover {
            return Placement::Skip;
        }
        return Placement::Read(interpret_cell(asset, cell));
    };
    let Some(blueprint) = project
        .blueprints
        .iter()
        .find(|blueprint| blueprint.id == id)
    else {
        return Placement::Fault(format!(
            "cell ({}, {}) blueprint \"{id}\" was not found; gameplay for this placement was omitted",
            cell.x, cell.y
        ));
    };
    // A terrain cell keeps its blueprint when auto-tiling swaps the picture.
    // The role stays authoritative. Other placements still require the
    // blueprint's visual, so a stamp cannot silently borrow another part's behavior.
    if cell.terrain.is_none() && blueprint.visual_id != cell.asset_id {
        return Placement::Fault(format!(
            "cell ({}, {}) blueprint \"{id}\" points at visual \"{}\" but the placement uses asset \"{}\"; gameplay for this placement was omitted",
            cell.x, cell.y, blueprint.visual_id, cell.asset_id
        ));
    }
    Placement::Read(read_from_role(&blueprint.legacy_role))
}

fn missing_asset_fault(cell: &Cell) -> String {
    format!(
        "cell ({}, {}) blueprint \"{}\" has no asset \"{}\"; gameplay for this placement was omitted",
        cell.x,
        cell.y,
        cell.blueprint_id.as_deref().unwrap_or(""),
        cell.asset_id
    )
}

fn read_from_role(role: &LegacyRole) -> CellRead {
    let mut read = CellRead {
        audio: false,
        podoboo: false,
        enemy: None,
        goal: false,
        item: None,
        block: None,
        hazard: None,
        balls: 0,
        coin: false,
        arrival_facing: 1,
        warp_set: false,
        warp_level: String::new(),
        warp_x: 0,
        warp_y: 0,
    };
    match role {
        LegacyRole::Inert => read.audio = true,
        LegacyRole::Enemy { kind, warp, .. } => {
            if *kind == LegacyEnemy::Podoboo {
                read.podoboo = true;
            } else {
                read.enemy = Some(kind_from_legacy(*kind));
            }
            apply_warp(&mut read, warp);
        }
        LegacyRole::Goal { warp } => {
            read.goal = true;
            apply_warp(&mut read, warp);
        }
        LegacyRole::Fixture {
            block,
            item,
            hazard,
            warp,
        } => {
            read.block = block.map(block_from_legacy);
            if let Some(item) = item {
                if *item == LegacyItem::Coin {
                    read.coin = true;
                } else {
                    read.item = Some(item_from_legacy(*item));
                }
            }
            if let Some(hazard) = hazard {
                let (kind, balls) = hazard_from_legacy(*hazard);
                read.hazard = Some(kind);
                read.balls = balls;
            }
            apply_warp(&mut read, warp);
        }
    }
    read
}

fn center_in(body: Rect, region: Rect) -> bool {
    let cx = body.x.saturating_add(body.w / 2);
    let cy = body.y.saturating_add(body.h / 2);
    cx >= region.x && cx < region.right() && cy >= region.y && cy < region.bottom()
}

fn apply_placement_warp(read: &mut CellRead, cell: &Cell) {
    match &cell.warp_choice {
        PlacementWarp::Inherit => {}
        PlacementWarp::Cleared => {
            read.warp_set = false;
            read.warp_level.clear();
            read.warp_x = 0;
            read.warp_y = 0;
        }
        PlacementWarp::Destination(target) => {
            read.warp_set = true;
            read.warp_level = target.level_id.clone();
            read.warp_x = target.x;
            read.warp_y = target.y;
        }
    }
}

fn apply_warp(read: &mut CellRead, warp: &Option<crate::blueprint::LegacyWarp>) {
    if let Some(warp) = warp {
        read.warp_set = true;
        read.warp_level = warp.level_id.clone();
        read.warp_x = warp.x;
        read.warp_y = warp.y;
    }
}

fn kind_from_legacy(kind: LegacyEnemy) -> Kind {
    match kind {
        LegacyEnemy::Goomba => Kind::Goomba,
        LegacyEnemy::GreenKoopa => Kind::GreenKoopa,
        LegacyEnemy::Buzzy => Kind::Buzzy,
        LegacyEnemy::RedKoopa => Kind::RedKoopa,
        LegacyEnemy::GreenPara => Kind::GreenPara,
        LegacyEnemy::RedPara => Kind::RedPara,
        LegacyEnemy::Piranha => Kind::Piranha,
        LegacyEnemy::Lakitu => Kind::Lakitu,
        LegacyEnemy::HammerBro => Kind::HammerBro,
        LegacyEnemy::Blaster => Kind::Blaster,
        LegacyEnemy::Blooper => Kind::Blooper,
        LegacyEnemy::CheepGreen => Kind::CheepGreen,
        LegacyEnemy::CheepRed => Kind::CheepRed,
        LegacyEnemy::CheepLeap => Kind::CheepLeap,
        LegacyEnemy::Podoboo => Kind::Podoboo,
        LegacyEnemy::Bowser => Kind::Bowser,
        LegacyEnemy::Spiny => Kind::Spiny,
        LegacyEnemy::Bill => Kind::Bill,
        LegacyEnemy::Shell => Kind::Shell,
        LegacyEnemy::Hammer => Kind::Hammer,
        LegacyEnemy::SpinyEgg => Kind::SpinyEgg,
    }
}

fn block_from_legacy(kind: LegacyBlock) -> BlockKind {
    match kind {
        LegacyBlock::Solid => BlockKind::Solid,
        LegacyBlock::Question => BlockKind::Question,
        LegacyBlock::Brick => BlockKind::Brick,
        LegacyBlock::Coin => BlockKind::Coin,
    }
}

fn item_from_legacy(kind: LegacyItem) -> ItemKind {
    match kind {
        LegacyItem::Mushroom => ItemKind::Mushroom,
        LegacyItem::OneUp => ItemKind::OneUp,
        LegacyItem::Flower => ItemKind::Flower,
        LegacyItem::Star => ItemKind::Star,
        LegacyItem::Plain => ItemKind::Plain,
        LegacyItem::Coin => ItemKind::Plain,
    }
}

fn hazard_from_legacy(kind: LegacyHazard) -> (HazardKind, i32) {
    match kind {
        LegacyHazard::Contact => (HazardKind::Contact, 0),
        LegacyHazard::Spring => (HazardKind::Spring, 0),
        LegacyHazard::Vine => (HazardKind::Vine, 0),
        LegacyHazard::FireBar { balls } => (HazardKind::FireBar, balls),
        LegacyHazard::PipeDown => (HazardKind::PipeDown, 0),
        LegacyHazard::PipeUp => (HazardKind::PipeUp, 0),
        LegacyHazard::PipeLeft => (HazardKind::PipeLeft, 0),
        LegacyHazard::PipeRight => (HazardKind::PipeRight, 0),
    }
}

fn warp_of(cell: &crate::project::Cell, asset: &Asset) -> (bool, String, i32, i32) {
    let warp = cell.warp.as_ref().or(asset.warp.as_ref());
    match warp {
        Some(WarpTarget { level_id, x, y }) => (true, level_id.clone(), *x, *y),
        None => (false, String::new(), 0, 0),
    }
}

fn resolve_block(asset: &Asset) -> BlockKind {
    match asset.block_tag {
        BlockTag::Solid => BlockKind::Solid,
        BlockTag::Question => BlockKind::Question,
        BlockTag::Brick => BlockKind::Brick,
        BlockTag::Coin => BlockKind::Coin,
        BlockTag::Auto => block_name(&asset.name),
    }
}

fn block_name(name: &str) -> BlockKind {
    let n = name.to_ascii_lowercase();
    if n.contains("question") || n.contains("? block") || n.contains("q-block") {
        BlockKind::Question
    } else if n.contains("coin") && (n.contains("block") || n.contains("brick")) {
        BlockKind::Coin
    } else if n.contains("brick") {
        BlockKind::Brick
    } else {
        BlockKind::Solid
    }
}

fn resolve_item(asset: &Asset) -> ItemKind {
    match asset.item_tag {
        ItemTag::Mushroom => ItemKind::Mushroom,
        ItemTag::OneUp => ItemKind::OneUp,
        ItemTag::Flower => ItemKind::Flower,
        ItemTag::Star => ItemKind::Star,
        ItemTag::Plain => ItemKind::Plain,
        ItemTag::Auto => item_name(&asset.name),
    }
}

fn item_name(name: &str) -> ItemKind {
    let n = name.to_ascii_lowercase();
    if n.contains("1-up") || n.contains("1up") || n.contains("one-up") || n.contains("one up") {
        ItemKind::OneUp
    } else if n.contains("flower") || (n.contains("fire") && !n.contains("bar")) {
        ItemKind::Flower
    } else if n.contains("star") {
        ItemKind::Star
    } else if n.contains("mushroom") || n.contains("shroom") {
        ItemKind::Mushroom
    } else {
        ItemKind::Plain
    }
}

fn is_podoboo(asset: &Asset) -> bool {
    asset.hazard_tag == HazardTag::Podoboo || asset.name.to_ascii_lowercase().contains("podoboo")
}

fn resolve_hazard(asset: &Asset) -> Option<HazardKind> {
    if asset.hazard_tag == HazardTag::Podoboo {
        return None;
    }
    match asset.hazard_tag {
        HazardTag::Contact => Some(HazardKind::Contact),
        HazardTag::Spring => Some(HazardKind::Spring),
        HazardTag::Vine => Some(HazardKind::Vine),
        HazardTag::FireBar | HazardTag::FireBarLong => Some(HazardKind::FireBar),
        HazardTag::PipeDown => Some(HazardKind::PipeDown),
        HazardTag::PipeRight => Some(HazardKind::PipeRight),
        HazardTag::Podoboo => None,
        HazardTag::Auto => {
            let named = hazard_name(&asset.name);
            if named.is_some() {
                named
            } else if asset.kind == AssetKind::Hazard {
                Some(HazardKind::Contact)
            } else {
                None
            }
        }
    }
}

fn hazard_name(name: &str) -> Option<HazardKind> {
    let n = name.to_ascii_lowercase();
    if n.contains("spring") {
        Some(HazardKind::Spring)
    } else if n.contains("vine") {
        Some(HazardKind::Vine)
    } else if n.contains("fire") && n.contains("bar") {
        Some(HazardKind::FireBar)
    } else if n.contains("pipe") {
        if n.contains("horizontal") || n.contains("side") || n.contains("right") {
            Some(HazardKind::PipeRight)
        } else {
            Some(HazardKind::PipeDown)
        }
    } else {
        None
    }
}

fn firebar_balls(asset: &Asset) -> i32 {
    if asset.hazard_tag == HazardTag::FireBarLong || asset.name.contains("12") {
        12
    } else {
        6
    }
}
