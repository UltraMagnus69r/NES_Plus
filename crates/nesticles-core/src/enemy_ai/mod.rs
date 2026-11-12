//! Pluggable enemy brains. The SMB1 baseline steps in fixed point at 60 Hz.
//! Later games add an [`EnemyAiStyle`] variant; they do not rewrite the caller.

mod smb1_nes;

pub use smb1_nes::step_enemies;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyArchetype {
    Walker,
    ShellWalker,
    Jumper,
    Flyer,
    Swimmer,
    Shooter,
    Boss,
    Goomba,
    GreenKoopa,
    BuzzyBeetle,
    RedKoopa,
    GreenParatroopa,
    RedParatroopa,
    GreenPiranha,
    Lakitu,
    HammerBro,
    BulletBill,
    Blooper,
    CheepGreen,
    CheepRed,
    CheepLeap,
    Podoboo,
    Bowser,
    Spiny,
    Custom(String),
}

impl Default for EnemyArchetype {
    fn default() -> Self {
        Self::Walker
    }
}

/// Course or project enemy brain. Per-enemy Inherit is [`None`], not a variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EnemyAiStyle {
    #[default]
    Smb1Nes,
    // Future styles are additional variants. Leave them out until a game is implemented.
}

impl EnemyAiStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Smb1Nes => "SMB1",
        }
    }

    pub const ALL: [EnemyAiStyle; 1] = [Self::Smb1Nes];
}

pub fn override_label(style: Option<EnemyAiStyle>) -> &'static str {
    match style {
        None => "Inherit Level Style",
        Some(style) => style.label(),
    }
}

/// Roster entries the course editor can assign. Unknown names still walk as a Goomba.
pub const SMB1_ROSTER: [EnemyArchetype; 17] = [
    EnemyArchetype::Goomba,
    EnemyArchetype::GreenKoopa,
    EnemyArchetype::BuzzyBeetle,
    EnemyArchetype::RedKoopa,
    EnemyArchetype::GreenParatroopa,
    EnemyArchetype::RedParatroopa,
    EnemyArchetype::GreenPiranha,
    EnemyArchetype::Lakitu,
    EnemyArchetype::HammerBro,
    EnemyArchetype::BulletBill,
    EnemyArchetype::Blooper,
    EnemyArchetype::CheepGreen,
    EnemyArchetype::CheepRed,
    EnemyArchetype::CheepLeap,
    EnemyArchetype::Podoboo,
    EnemyArchetype::Bowser,
    EnemyArchetype::Spiny,
];

pub fn archetype_label(kind: &EnemyArchetype) -> String {
    match kind {
        EnemyArchetype::Walker | EnemyArchetype::Goomba => "Goomba".into(),
        EnemyArchetype::GreenKoopa => "Green Koopa".into(),
        EnemyArchetype::BuzzyBeetle => "Buzzy Beetle".into(),
        EnemyArchetype::RedKoopa => "Red Koopa".into(),
        EnemyArchetype::GreenParatroopa => "Green Paratroopa".into(),
        EnemyArchetype::RedParatroopa => "Red Paratroopa".into(),
        EnemyArchetype::GreenPiranha => "Green Piranha".into(),
        EnemyArchetype::Lakitu => "Lakitu".into(),
        EnemyArchetype::HammerBro => "Hammer Bro".into(),
        EnemyArchetype::BulletBill => "Bullet Bill".into(),
        EnemyArchetype::Blooper => "Blooper".into(),
        EnemyArchetype::CheepGreen => "Cheep Cheep".into(),
        EnemyArchetype::CheepRed => "Red Cheep Cheep".into(),
        EnemyArchetype::CheepLeap => "Leaping Cheep Cheep".into(),
        EnemyArchetype::Podoboo => "Podoboo".into(),
        EnemyArchetype::Bowser => "Bowser".into(),
        EnemyArchetype::Spiny => "Spiny".into(),
        EnemyArchetype::ShellWalker => "Shell walker".into(),
        EnemyArchetype::Jumper => "Jumper".into(),
        EnemyArchetype::Flyer => "Flyer".into(),
        EnemyArchetype::Swimmer => "Swimmer".into(),
        EnemyArchetype::Shooter => "Shooter".into(),
        EnemyArchetype::Boss => "Boss".into(),
        EnemyArchetype::Custom(name) => name.clone(),
    }
}

/// None on the enemy uses the course style. A missing course style uses the project style.
pub fn resolve_style(
    enemy_override: Option<EnemyAiStyle>,
    level: EnemyAiStyle,
    global: EnemyAiStyle,
) -> EnemyAiStyle {
    if let Some(style) = enemy_override {
        return style;
    }
    match level {
        EnemyAiStyle::Smb1Nes => match global {
            EnemyAiStyle::Smb1Nes => EnemyAiStyle::Smb1Nes,
        },
    }
}

/// Fixed-point enemy body. Same 4096 scale as the SMB1 character step.
#[derive(Clone, Debug)]
pub struct EnemyRuntime {
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub vy: i32,
    pub w: i32,
    pub h: i32,
    pub timers: [i32; 4],
    pub patrol_dir: i32,
    pub phase: i32,
    pub flags: u32,
    pub awake: bool,
    pub on_ground: bool,
    pub archetype: EnemyArchetype,
    pub style_override: Option<EnemyAiStyle>,
    pub cell_x: i32,
    pub cell_y: i32,
}

impl EnemyRuntime {
    pub fn spawn(
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        archetype: EnemyArchetype,
        style_override: Option<EnemyAiStyle>,
        cell_x: i32,
        cell_y: i32,
    ) -> Self {
        Self {
            x,
            y,
            vx: 0,
            vy: 0,
            w,
            h,
            timers: [0; 4],
            patrol_dir: -1,
            phase: 0,
            flags: 0,
            awake: false,
            on_ground: false,
            archetype,
            style_override,
            cell_x,
            cell_y,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CameraPx {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
