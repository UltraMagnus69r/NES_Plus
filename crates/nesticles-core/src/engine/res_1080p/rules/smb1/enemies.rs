//! SMB1 roster. Speeds are per 60 Hz tick in 4096-units.
//! Actors stay dormant until they enter the camera plus one tile.

use super::super::super::trig::sin_mdeg;
use super::super::super::{Rect, FP, HIT_H, HIT_W, INSET, TILE_FP};
use super::collision::{move_body, on_floor, Solid, YHit};
use super::hazards::sees;
use crate::enemy_ai::EnemyArchetype;

pub(crate) const WALKER_GRAVITY: i32 = 8064;
pub(crate) const MAX_FALL: i32 = 82944;
pub(crate) const WALKER_VX: i32 = 9216;
const SHELL_VX: i32 = 55296;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Goomba,
    GreenKoopa,
    Buzzy,
    RedKoopa,
    GreenPara,
    RedPara,
    Piranha,
    Lakitu,
    HammerBro,
    Blaster,
    Bill,
    Blooper,
    CheepGreen,
    CheepRed,
    CheepLeap,
    Podoboo,
    Bowser,
    Spiny,
    Shell,
    Hammer,
    SpinyEgg,
}

#[derive(Clone, Debug)]
pub struct EnemyActor {
    pub kind: Kind,
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub w: i32,
    pub h: i32,
    pub facing: i32,
    pub alive: bool,
    pub awake: bool,
    pub on_ground: bool,
    pub timer: i32,
    pub timer_b: i32,
    pub phase: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub hits: i32,
    pub kicked: bool,
    pub buzzy: bool,
    pub source: bool,
    pub asset_id: String,
    pub cell_x: i32,
    pub cell_y: i32,
}

impl EnemyActor {
    pub fn at_cell(kind: Kind, cell_x: i32, cell_y: i32, asset_id: String) -> Self {
        let x = cell_x.saturating_mul(TILE_FP) + INSET;
        let y = cell_y.saturating_mul(TILE_FP);
        Self {
            kind,
            x,
            y,
            vx: 0,
            vy: 0,
            w: HIT_W,
            h: HIT_H,
            facing: -1,
            alive: true,
            awake: false,
            on_ground: false,
            timer: 0,
            timer_b: 0,
            phase: 0,
            home_x: x,
            home_y: y,
            hits: 0,
            kicked: false,
            buzzy: kind == Kind::Buzzy,
            source: true,
            asset_id,
            cell_x,
            cell_y,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }

    pub fn fire_immune(&self) -> bool {
        match self.kind {
            Kind::Buzzy | Kind::Podoboo => true,
            Kind::Shell => self.buzzy,
            _ => false,
        }
    }

    pub fn always_hurts(&self) -> bool {
        matches!(
            self.kind,
            Kind::Spiny | Kind::Podoboo | Kind::Bowser | Kind::Hammer
        )
    }
}

pub fn kind_of(archetype: &EnemyArchetype, name: &str) -> Kind {
    match archetype {
        EnemyArchetype::Goomba => Kind::Goomba,
        EnemyArchetype::GreenKoopa => Kind::GreenKoopa,
        EnemyArchetype::BuzzyBeetle => Kind::Buzzy,
        EnemyArchetype::RedKoopa => Kind::RedKoopa,
        EnemyArchetype::GreenParatroopa => Kind::GreenPara,
        EnemyArchetype::RedParatroopa => Kind::RedPara,
        EnemyArchetype::GreenPiranha => Kind::Piranha,
        EnemyArchetype::Lakitu => Kind::Lakitu,
        EnemyArchetype::HammerBro => Kind::HammerBro,
        EnemyArchetype::BulletBill => Kind::Blaster,
        EnemyArchetype::Blooper => Kind::Blooper,
        EnemyArchetype::CheepGreen => Kind::CheepGreen,
        EnemyArchetype::CheepRed => Kind::CheepRed,
        EnemyArchetype::CheepLeap => Kind::CheepLeap,
        EnemyArchetype::Podoboo => Kind::Podoboo,
        EnemyArchetype::Bowser => Kind::Bowser,
        EnemyArchetype::Spiny => Kind::Spiny,
        EnemyArchetype::Custom(text) => kind_from_name(text),
        _ => kind_from_name(name),
    }
}

pub fn kind_from_name(name: &str) -> Kind {
    let n = name.to_ascii_lowercase();
    if n.contains("buzzy") {
        Kind::Buzzy
    } else if n.contains("spiny") {
        Kind::Spiny
    } else if n.contains("piranha") {
        Kind::Piranha
    } else if n.contains("lakitu") {
        Kind::Lakitu
    } else if n.contains("hammer") {
        Kind::HammerBro
    } else if n.contains("bullet") || n.contains("blaster") {
        Kind::Blaster
    } else if n.contains("blooper") {
        Kind::Blooper
    } else if n.contains("podoboo") {
        Kind::Podoboo
    } else if n.contains("bowser") {
        Kind::Bowser
    } else if n.contains("leap") && n.contains("cheep") {
        Kind::CheepLeap
    } else if n.contains("red") && n.contains("cheep") {
        Kind::CheepRed
    } else if n.contains("cheep") {
        Kind::CheepGreen
    } else if n.contains("red") && n.contains("para") {
        Kind::RedPara
    } else if n.contains("para") {
        Kind::GreenPara
    } else if n.contains("red") && n.contains("koopa") {
        Kind::RedKoopa
    } else if n.contains("koopa") {
        Kind::GreenKoopa
    } else {
        Kind::Goomba
    }
}

pub fn tick(
    enemies: &mut Vec<EnemyActor>,
    solids: &[Solid],
    player_x: i32,
    player_y: i32,
    cam_x: i32,
    cam_y: i32,
) {
    let mut spawned = Vec::new();
    for enemy in enemies.iter_mut() {
        if !enemy.alive {
            continue;
        }
        if !enemy.awake {
            if !sees(enemy.x, enemy.y, enemy.w, enemy.h, cam_x, cam_y) {
                continue;
            }
            wake(enemy, player_x);
        }
        step_one(enemy, solids, player_x, player_y, cam_x, &mut spawned);
    }
    bump_and_shells(enemies);
    enemies.extend(spawned);
}

fn wake(enemy: &mut EnemyActor, player_x: i32) {
    enemy.awake = true;
    let toward = if player_x < enemy.x { -1 } else { 1 };
    match enemy.kind {
        Kind::Goomba | Kind::GreenKoopa | Kind::Buzzy | Kind::RedKoopa | Kind::Spiny => {
            enemy.facing = toward;
            enemy.vx = toward * WALKER_VX;
        }
        Kind::GreenPara => {
            enemy.facing = toward;
            enemy.vx = toward * WALKER_VX;
        }
        Kind::CheepGreen => {
            enemy.facing = -1;
            enemy.vx = -WALKER_VX;
        }
        Kind::CheepRed | Kind::CheepLeap => {
            enemy.facing = -1;
            enemy.vx = -18432;
        }
        Kind::RedPara => {
            enemy.vy = -18432;
        }
        _ => {}
    }
}

fn step_one(
    enemy: &mut EnemyActor,
    solids: &[Solid],
    player_x: i32,
    player_y: i32,
    cam_x: i32,
    spawned: &mut Vec<EnemyActor>,
) {
    match enemy.kind {
        Kind::Goomba | Kind::GreenKoopa | Kind::Buzzy | Kind::Spiny => {
            walk(enemy, solids, WALKER_VX, false, WALKER_GRAVITY);
        }
        Kind::RedKoopa => walk(enemy, solids, WALKER_VX, true, WALKER_GRAVITY),
        Kind::Shell => step_shell(enemy, solids),
        Kind::GreenPara => step_green_para(enemy, solids),
        Kind::RedPara => step_red_para(enemy),
        Kind::Piranha => step_piranha(enemy, player_x),
        Kind::Lakitu => step_lakitu(enemy, player_x, spawned),
        Kind::HammerBro => step_hammer_bro(enemy, solids, player_x, spawned),
        Kind::Blaster => step_blaster(enemy, player_x, spawned),
        Kind::Bill => step_bill(enemy, cam_x),
        Kind::Blooper => step_blooper(enemy, player_x, player_y),
        Kind::CheepGreen => step_cheep(enemy, -WALKER_VX),
        Kind::CheepRed => step_cheep(enemy, -18432),
        Kind::CheepLeap => step_leap(enemy, solids),
        Kind::Podoboo => step_podoboo(enemy),
        Kind::Bowser => step_bowser(enemy, solids, player_x, spawned),
        Kind::Hammer => step_hammer(enemy),
        Kind::SpinyEgg => step_egg(enemy, solids, player_x),
    }
}

fn walk(enemy: &mut EnemyActor, solids: &[Solid], speed: i32, ledges: bool, gravity: i32) {
    if enemy.facing == 0 {
        enemy.facing = -1;
    }
    enemy.facing = enemy.facing.signum();
    if ledges && enemy.on_ground && !ledge_ahead(enemy, solids) {
        enemy.facing = -enemy.facing;
    }
    enemy.vx = enemy.facing.saturating_mul(speed);
    if enemy.on_ground {
        enemy.vy = 0;
    } else {
        enemy.vy = enemy.vy.saturating_add(gravity).min(MAX_FALL);
    }
    integrate(enemy, solids, true);
    if enemy.on_ground && !on_floor(enemy.rect(), solids) {
        enemy.on_ground = false;
    }
}

fn ledge_ahead(enemy: &EnemyActor, solids: &[Solid]) -> bool {
    let x = if enemy.facing > 0 {
        enemy.x + enemy.w + FP
    } else {
        enemy.x - FP
    };
    let probe = Rect {
        x,
        y: enemy.y + enemy.h,
        w: FP,
        h: FP,
    };
    solids
        .iter()
        .any(|solid| !solid.broken && super::super::super::overlaps(probe, solid.rect()))
}

fn integrate(enemy: &mut EnemyActor, solids: &[Solid], reverse_walls: bool) {
    let mut rect = enemy.rect();
    let hit = move_body(&mut rect, enemy.vx, enemy.vy, solids, false);
    enemy.x = rect.x;
    enemy.y = rect.y;
    if reverse_walls && hit.wall && enemy.vx != 0 {
        enemy.facing = -enemy.facing.signum();
        if enemy.facing == 0 {
            enemy.facing = -1;
        }
        enemy.vx = enemy.facing.saturating_mul(enemy.vx.abs());
    }
    match hit.y {
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
}

fn step_shell(enemy: &mut EnemyActor, solids: &[Solid]) {
    if enemy.kicked {
        walk(enemy, solids, SHELL_VX, false, WALKER_GRAVITY);
        return;
    }
    enemy.vx = 0;
    enemy.timer = enemy.timer.saturating_add(1);
    if !enemy.on_ground {
        enemy.vy = enemy.vy.saturating_add(WALKER_GRAVITY).min(MAX_FALL);
    }
    integrate(enemy, solids, false);
    if enemy.timer >= 320 {
        enemy.kind = if enemy.buzzy {
            Kind::Buzzy
        } else {
            Kind::GreenKoopa
        };
        enemy.timer = 0;
        enemy.vx = enemy.facing.signum().saturating_mul(WALKER_VX);
        if enemy.vx == 0 {
            enemy.facing = 1;
            enemy.vx = WALKER_VX;
        }
    }
}

fn step_green_para(enemy: &mut EnemyActor, solids: &[Solid]) {
    if enemy.facing == 0 {
        enemy.facing = -1;
    }
    if enemy.on_ground {
        enemy.vy = -55296;
        enemy.on_ground = false;
    } else {
        enemy.vy = enemy.vy.saturating_add(3456).min(MAX_FALL);
    }
    enemy.vx = enemy.facing.signum().saturating_mul(WALKER_VX);
    integrate(enemy, solids, true);
}

fn step_red_para(enemy: &mut EnemyActor) {
    let range = 216 * FP;
    if enemy.vy == 0 {
        enemy.vy = -18432;
    }
    if enemy.y <= enemy.home_y - range {
        enemy.vy = 18432;
    } else if enemy.y >= enemy.home_y + range {
        enemy.vy = -18432;
    }
    enemy.y = enemy.y.saturating_add(enemy.vy);
    enemy.vx = 0;
}

fn step_piranha(enemy: &mut EnemyActor, player_x: i32) {
    let speed = WALKER_VX;
    match enemy.phase {
        0 => {
            enemy.y = enemy.home_y;
            enemy.vy = 0;
            enemy.timer = enemy.timer.saturating_add(1);
            let dx = (player_x - (enemy.x + enemy.w / 2)).abs();
            if enemy.timer >= 60 && dx >= 108 * FP {
                enemy.phase = 1;
                enemy.timer = 0;
            }
        }
        1 => {
            enemy.vy = -speed;
            enemy.y = enemy.y.saturating_add(enemy.vy);
            enemy.timer = enemy.timer.saturating_add(1);
            if enemy.timer >= 64 {
                enemy.phase = 2;
                enemy.timer = 0;
            }
        }
        2 => {
            enemy.vy = 0;
            enemy.timer = enemy.timer.saturating_add(1);
            if enemy.timer >= 60 {
                enemy.phase = 3;
                enemy.timer = 0;
            }
        }
        _ => {
            enemy.vy = speed;
            enemy.y = enemy.y.saturating_add(enemy.vy);
            enemy.timer = enemy.timer.saturating_add(1);
            if enemy.timer >= 64 {
                enemy.y = enemy.home_y;
                enemy.phase = 0;
                enemy.timer = 0;
            }
        }
    }
}

fn step_lakitu(enemy: &mut EnemyActor, player_x: i32, spawned: &mut Vec<EnemyActor>) {
    let lead = if enemy.x <= player_x {
        4 * TILE_FP
    } else {
        -4 * TILE_FP
    };
    let target = player_x.saturating_add(lead);
    let dx = target.saturating_sub(enemy.x);
    enemy.vx = dx.clamp(-36864, 36864);
    enemy.x = enemy.x.saturating_add(enemy.vx);
    enemy.timer = enemy.timer.saturating_add(1);
    if enemy.timer >= 210 {
        enemy.timer = 0;
        let mut egg = projectile(Kind::SpinyEgg, enemy.x, enemy.y + enemy.h);
        egg.vy = 4096;
        spawned.push(egg);
    }
}

fn step_hammer_bro(
    enemy: &mut EnemyActor,
    solids: &[Solid],
    player_x: i32,
    spawned: &mut Vec<EnemyActor>,
) {
    enemy.timer_b = enemy.timer_b.saturating_add(1);
    if enemy.timer_b >= 1200 {
        enemy.facing = if player_x < enemy.x { -1 } else { 1 };
        enemy.vx = enemy.facing * 18432;
    } else {
        if (enemy.x - enemy.home_x).abs() >= TILE_FP {
            enemy.facing = -enemy.facing.signum();
            if enemy.facing == 0 {
                enemy.facing = -1;
            }
        }
        enemy.vx = enemy.facing.signum().saturating_mul(WALKER_VX);
    }
    if enemy.on_ground && enemy.timer >= 90 {
        enemy.vy = if enemy.phase % 2 == 0 { -73728 } else { -46080 };
        enemy.phase = enemy.phase.saturating_add(1);
        enemy.timer = 0;
        enemy.on_ground = false;
    } else if enemy.on_ground {
        enemy.vy = 0;
        enemy.timer = enemy.timer.saturating_add(1);
    } else {
        enemy.vy = enemy.vy.saturating_add(WALKER_GRAVITY).min(MAX_FALL);
        enemy.timer = enemy.timer.saturating_add(1);
    }
    integrate(enemy, solids, true);
    enemy.hits = enemy.hits.saturating_add(1);
    if enemy.hits >= 150 {
        enemy.hits = 0;
        let dir = if player_x < enemy.x { -1 } else { 1 };
        let mut hammer = projectile(Kind::Hammer, enemy.x, enemy.y);
        hammer.vx = dir * 27648;
        hammer.vy = -64512;
        hammer.home_y = enemy.y;
        spawned.push(hammer);
    }
}

fn step_blaster(enemy: &mut EnemyActor, player_x: i32, spawned: &mut Vec<EnemyActor>) {
    enemy.vx = 0;
    enemy.vy = 0;
    enemy.timer = enemy.timer.saturating_add(1);
    let dx = (player_x - enemy.x).abs();
    if enemy.timer >= 90 && dx > 72 * FP {
        enemy.timer = 0;
        let dir = if player_x >= enemy.x { 1 } else { -1 };
        let mut bill = projectile(Kind::Bill, enemy.x, enemy.y);
        bill.vx = dir * 36864;
        bill.facing = dir;
        spawned.push(bill);
    }
}

fn step_bill(enemy: &mut EnemyActor, cam_x: i32) {
    enemy.vy = 0;
    enemy.x = enemy.x.saturating_add(enemy.vx);
    if (enemy.x - cam_x).abs() > super::super::super::SCREEN_W_FP.saturating_mul(2) {
        enemy.alive = false;
    }
}

fn step_blooper(enemy: &mut EnemyActor, player_x: i32, player_y: i32) {
    if enemy.phase == 0 {
        enemy.vx = 0;
        enemy.vy = WALKER_VX;
        enemy.y = enemy.y.saturating_add(enemy.vy);
        if enemy.y > player_y {
            enemy.phase = 1;
            enemy.timer = 24;
            enemy.vx = if player_x >= enemy.x { 18432 } else { -18432 };
            enemy.vy = -36864;
        }
    } else {
        enemy.x = enemy.x.saturating_add(enemy.vx);
        enemy.y = enemy.y.saturating_add(enemy.vy);
        enemy.timer -= 1;
        if enemy.timer <= 0 {
            enemy.phase = 0;
        }
    }
}

fn step_cheep(enemy: &mut EnemyActor, vx: i32) {
    enemy.vx = vx;
    enemy.phase = enemy.phase.wrapping_add(3_000);
    let bob = i64::from(sin_mdeg(enemy.phase)) * i64::from(24 * FP) / 4096;
    enemy.y = enemy.home_y.saturating_add(bob as i32);
    enemy.x = enemy.x.saturating_add(enemy.vx);
    enemy.vy = 0;
}

fn step_leap(enemy: &mut EnemyActor, solids: &[Solid]) {
    if enemy.on_ground {
        enemy.vy = -101376;
        enemy.on_ground = false;
    } else {
        enemy.vy = enemy.vy.saturating_add(4608).min(MAX_FALL);
    }
    enemy.vx = -WALKER_VX;
    integrate(enemy, solids, false);
}

fn step_podoboo(enemy: &mut EnemyActor) {
    if enemy.phase == 0 {
        enemy.y = enemy.home_y;
        enemy.vy = 0;
        enemy.timer = enemy.timer.saturating_add(1);
        if enemy.timer > 50 {
            enemy.vy = -101376;
            enemy.phase = 1;
            enemy.timer = 0;
        }
    } else {
        enemy.vy = enemy.vy.saturating_add(3456);
        enemy.y = enemy.y.saturating_add(enemy.vy);
        if enemy.vy > 0 && enemy.y >= enemy.home_y {
            enemy.y = enemy.home_y;
            enemy.vy = 0;
            enemy.phase = 0;
        }
    }
}

fn step_bowser(
    enemy: &mut EnemyActor,
    solids: &[Solid],
    player_x: i32,
    spawned: &mut Vec<EnemyActor>,
) {
    if enemy.facing == 0 {
        enemy.facing = -1;
    }
    if (enemy.x - enemy.home_x).abs() >= TILE_FP * 3 {
        enemy.facing = -enemy.facing.signum();
    }
    enemy.vx = enemy.facing.signum().saturating_mul(WALKER_VX);
    if enemy.on_ground {
        enemy.vy = 0;
    } else {
        enemy.vy = enemy.vy.saturating_add(WALKER_GRAVITY).min(MAX_FALL);
    }
    integrate(enemy, solids, true);
    enemy.timer = enemy.timer.saturating_add(1);
    if enemy.timer >= 100 {
        enemy.timer = 0;
        enemy.phase = enemy.phase.saturating_add(1);
        if enemy.phase % 2 == 0 && enemy.on_ground {
            enemy.vy = -73728;
            enemy.on_ground = false;
        } else {
            let dir = if player_x < enemy.x { -1 } else { 1 };
            let mut hammer = projectile(Kind::Hammer, enemy.x, enemy.y);
            hammer.vx = dir * 27648;
            hammer.vy = -64512;
            hammer.home_y = enemy.y;
            spawned.push(hammer);
        }
    }
}

fn step_hammer(enemy: &mut EnemyActor) {
    enemy.vy = enemy.vy.saturating_add(WALKER_GRAVITY).min(MAX_FALL);
    enemy.x = enemy.x.saturating_add(enemy.vx);
    enemy.y = enemy.y.saturating_add(enemy.vy);
    if enemy.y > enemy.home_y.saturating_add(TILE_FP * 20) {
        enemy.alive = false;
    }
}

fn step_egg(enemy: &mut EnemyActor, solids: &[Solid], player_x: i32) {
    enemy.vy = enemy.vy.saturating_add(WALKER_GRAVITY).min(MAX_FALL);
    integrate(enemy, solids, false);
    if enemy.on_ground {
        enemy.kind = Kind::Spiny;
        enemy.facing = if player_x < enemy.x { -1 } else { 1 };
        enemy.vx = enemy.facing * WALKER_VX;
        enemy.vy = 0;
        enemy.timer = 0;
    }
}

fn projectile(kind: Kind, x: i32, y: i32) -> EnemyActor {
    EnemyActor {
        kind,
        x,
        y,
        vx: 0,
        vy: 0,
        w: HIT_W,
        h: HIT_H,
        facing: -1,
        alive: true,
        awake: true,
        on_ground: false,
        timer: 0,
        timer_b: 0,
        phase: 0,
        home_x: x,
        home_y: y,
        hits: 0,
        kicked: false,
        buzzy: false,
        source: false,
        asset_id: String::new(),
        cell_x: -1,
        cell_y: -1,
    }
}

fn bump_and_shells(enemies: &mut [EnemyActor]) {
    let len = enemies.len();
    for i in 0..len {
        if !enemies[i].alive || !enemies[i].awake {
            continue;
        }
        if enemies[i].kind == Kind::Shell && enemies[i].kicked {
            for j in 0..len {
                if i == j || !enemies[j].alive {
                    continue;
                }
                if super::super::super::overlaps(enemies[i].rect(), enemies[j].rect()) {
                    enemies[j].alive = false;
                }
            }
        }
    }
    for i in 0..len {
        if !enemies[i].alive || enemies[i].kind != Kind::Goomba {
            continue;
        }
        for j in 0..len {
            if i == j || !enemies[j].alive {
                continue;
            }
            if enemies[j].kind == Kind::Shell && enemies[j].kicked {
                continue;
            }
            if !super::super::super::overlaps(enemies[i].rect(), enemies[j].rect()) {
                continue;
            }
            let toward = (enemies[i].vx > 0 && enemies[j].x >= enemies[i].x)
                || (enemies[i].vx < 0 && enemies[j].x <= enemies[i].x);
            if toward {
                enemies[i].facing = -enemies[i].facing.signum();
                if enemies[i].facing == 0 {
                    enemies[i].facing = -1;
                }
                enemies[i].vx = enemies[i].facing * WALKER_VX;
            }
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::collision::BlockKind;
    use super::*;

    #[test]
    fn goomba_reverses_when_a_solid_is_in_front() {
        let mut goomba = EnemyActor::at_cell(Kind::Goomba, 0, 0, "goomba".into());
        goomba.awake = true;
        goomba.on_ground = true;
        goomba.facing = 1;
        goomba.vx = WALKER_VX;
        goomba.x = 2 * TILE_FP - goomba.w - 100;
        let wall = Solid::at_cell(2, 0, BlockKind::Solid);
        let floor = Solid::at_cell(0, 1, BlockKind::Solid);
        let mut enemies = vec![goomba];
        tick(&mut enemies, &[wall, floor], 0, 0, 0, 0);
        assert_eq!(enemies[0].vx, -WALKER_VX);
        assert_eq!(enemies[0].facing, -1);
    }
}
