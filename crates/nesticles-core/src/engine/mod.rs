//! Screen-resolution gameplay. Each profile owns its own step so a later
//! `res_240p` or `res_720p` module does not share bodies with `res_1080p`.

pub mod res_1080p;

/// Which screen category is simulating. Play asks the profile to tick; it does not
/// reach into another resolution's numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ResolutionProfile {
    #[default]
    Res1080p,
}

impl ResolutionProfile {
    pub fn screen_px(self) -> (i32, i32) {
        match self {
            Self::Res1080p => (res_1080p::SCREEN_W, res_1080p::SCREEN_H),
        }
    }

    pub fn tile_px(self) -> i32 {
        match self {
            Self::Res1080p => res_1080p::TILE_PX,
        }
    }

    pub fn unit(self) -> i32 {
        match self {
            Self::Res1080p => res_1080p::FP,
        }
    }
}

/// One fixed 60 Hz pad sample. Velocities are never scaled by a frame delta.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pad {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump_held: bool,
    pub jump_pressed: bool,
    pub run_held: bool,
}

/// Active maker field. New resolutions are new variants, not edits inside `res_1080p`.
#[derive(Clone, Debug)]
pub enum PlayField {
    Res1080p(res_1080p::World),
}

impl PlayField {
    pub fn profile(&self) -> ResolutionProfile {
        match self {
            Self::Res1080p(_) => ResolutionProfile::Res1080p,
        }
    }

    pub fn tick(&mut self, pad: &Pad) {
        match self {
            Self::Res1080p(world) => {
                let profile = ResolutionProfile::Res1080p;
                let _ = (profile.screen_px(), profile.tile_px(), profile.unit());
                world.tick(pad);
            }
        }
    }

    pub fn world_1080(&self) -> Option<&res_1080p::World> {
        match self {
            Self::Res1080p(world) => Some(world),
        }
    }

    pub fn world_1080_mut(&mut self) -> Option<&mut res_1080p::World> {
        match self {
            Self::Res1080p(world) => Some(world),
        }
    }
}
