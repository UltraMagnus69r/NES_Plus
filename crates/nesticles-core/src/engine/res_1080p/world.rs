//! One 1080p course. The stored grid stays 16px cells; this converts each cell
//! into a 72×72 tile while the field is running.

use super::character_physics::{apply_stomp, params_for, step_motion, Body, Power};
use super::powerups::smb1::{
    apply_damage, collect, spawn_from_block, step_fireball, step_item, try_fire, Fireball, Hurt,
    ItemActor, ItemKind,
};
use super::rules::smb1::collision::{
    bump_offset, move_body, on_floor, BlockKind, MoveHit, Solid, YHit,
};
use super::rules::smb1::enemies::{self, kind_of, EnemyActor, Kind};
use super::rules::smb1::hazards::{feet_overlap, step_firebars, Hazard, HazardKind};
use super::{
    overlaps, CharacterId, Rect, FP, HIT_H, HIT_TALL, HIT_W, INSET, SCREEN_H_FP, SCREEN_W_FP,
    TILE_FP,
};
use crate::engine::Pad;
use crate::project::{
    Asset, AssetKind, BlockTag, HazardTag, ItemTag, Layer, Level, Project, WarpTarget,
};

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
    pub pending_level: String,
    pub pending_x: i32,
    pub pending_y: i32,
    pub pending: bool,
    pub source_enemies: usize,
    pub bump_marks: Vec<(i32, i32, i32)>,
    pub hidden: Vec<(i32, i32)>,
    pub hide_player: bool,
}

impl World {
    pub fn from_level(project: &Project, level: &Level, character: CharacterId) -> Self {
        let mut solids = Vec::new();
        let mut enemies = Vec::new();
        let mut hazards = Vec::new();
        let mut items = Vec::new();
        let mut goals = Vec::new();
        for cell in &level.cells {
            let Some(asset) = project
                .assets
                .iter()
                .find(|asset| asset.id == cell.asset_id)
            else {
                continue;
            };
            if asset.kind.is_audio() {
                continue;
            }
            let (warp_set, warp_level, warp_x, warp_y) = warp_of(cell, asset);
            if is_podoboo(asset) {
                enemies.push(EnemyActor::at_cell(
                    Kind::Podoboo,
                    cell.x,
                    cell.y,
                    asset.id.clone(),
                ));
                continue;
            }
            if asset.kind == AssetKind::Enemy {
                enemies.push(EnemyActor::at_cell(
                    kind_of(&cell.archetype, &asset.name),
                    cell.x,
                    cell.y,
                    asset.id.clone(),
                ));
                continue;
            }
            if asset.kind == AssetKind::Goal {
                goals.push(cell_rect(cell.x, cell.y));
                continue;
            }
            if asset.kind == AssetKind::Item {
                items.push(placed_item(cell.x, cell.y, asset));
            }
            let solid_terrain =
                asset.kind == AssetKind::Terrain && (asset.solid || cell.layer == Layer::Main);
            let hazard_kind = resolve_hazard(asset);
            let pipe = matches!(
                hazard_kind,
                Some(HazardKind::PipeDown | HazardKind::PipeRight)
            );
            if solid_terrain || pipe {
                let block = if solid_terrain {
                    resolve_block(asset)
                } else {
                    BlockKind::Solid
                };
                solids.push(Solid::at_cell(cell.x, cell.y, block));
            }
            if let Some(kind) = hazard_kind {
                let balls = if kind == HazardKind::FireBar {
                    firebar_balls(asset)
                } else {
                    0
                };
                hazards.push(Hazard {
                    kind,
                    x: cell.x.saturating_mul(TILE_FP),
                    y: cell.y.saturating_mul(TILE_FP),
                    w: TILE_FP,
                    h: TILE_FP,
                    cell_x: cell.x,
                    cell_y: cell.y,
                    awake: false,
                    warp_level,
                    warp_x,
                    warp_y,
                    warp_set,
                    angle: 0,
                    balls,
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
            pending_level: String::new(),
            pending_x: 0,
            pending_y: 0,
            pending: false,
            source_enemies,
            bump_marks: Vec::new(),
            hidden: Vec::new(),
            hide_player: false,
        };
        world.update_camera();
        world
    }

    pub fn place(&mut self, cell_x: i32, cell_y: i32) {
        self.body.x = cell_x.saturating_mul(TILE_FP) + INSET;
        self.body.y = cell_y.saturating_mul(TILE_FP);
        self.body.vx = 0;
        self.body.vy = 0;
        self.body.climbing = false;
        self.body.spring_timer = 0;
        self.warp_lock = 30;
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
                if matches!(power, Power::Super | Power::Fire) {
                    self.solids[index].broken = true;
                    self.hidden.push((cell_x, cell_y));
                    self.defeat_on(cell_x, cell_y);
                } else if self.solids[index].bump == 0 {
                    self.solids[index].bump = 8;
                    self.defeat_on(cell_x, cell_y);
                }
            }
            BlockKind::Question if !self.solids[index].used => {
                self.solids[index].used = true;
                self.solids[index].block = BlockKind::Solid;
                self.solids[index].bump = 8;
                self.items.push(spawn_from_block(cell_x, cell_y, power));
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
            return;
        }
        if !pad.down && !pad.right {
            return;
        }
        let body = player_rect(&self.body);
        let mut chosen: Option<(String, i32, i32)> = None;
        for hazard in &self.hazards {
            let vertical = hazard.kind == HazardKind::PipeDown && pad.down;
            let horizontal = hazard.kind == HazardKind::PipeRight && pad.right;
            if !(vertical || horizontal) || !hazard.warp_set {
                continue;
            }
            if overlaps(body, hazard.rect()) {
                chosen = Some((hazard.warp_level.clone(), hazard.warp_x, hazard.warp_y));
                break;
            }
        }
        let Some((level, x, y)) = chosen else {
            return;
        };
        self.warp_lock = 30;
        self.body.vx = 0;
        self.body.vy = 0;
        if level.is_empty() {
            self.body.x = x.saturating_mul(TILE_FP) + INSET;
            self.body.y = y.saturating_mul(TILE_FP);
        } else {
            self.pending = true;
            self.pending_level = level;
            self.pending_x = x;
            self.pending_y = y;
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

fn placed_item(cell_x: i32, cell_y: i32, asset: &Asset) -> ItemActor {
    let kind = resolve_item(asset);
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
        asset_id: asset.id.clone(),
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
