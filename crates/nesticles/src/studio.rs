//! Shared visual tokens and workspace decisions for the two Nesticles sides.
//! Maker Engine and UnMaker Lab both read these values.

use eframe::egui::{Color32, Visuals};

/// The desktop build opens fullscreen. A windowed fallback is not the default.
pub const STARTS_FULLSCREEN: bool = true;

/// Next visual pass: white field, light grey, dark text, a little red, large controls.
/// Phase 9.1 does not restyle the screens that already shipped.
pub const FUTURE_UI: &str = "clinical white, light grey, dark text, restrained red";

pub const BAR_H: f32 = 56.0;
pub const STRIP_H: f32 = 112.0;
pub const RAIL_W: f32 = 80.0;
pub const BOTTOM_H: f32 = 72.0;
pub const THUMB: f32 = 72.0;
pub const TOOL: f32 = 56.0;
pub const INSPECTOR_W: f32 = 320.0;

pub fn navy() -> Color32 {
    Color32::from_rgb(27, 40, 54)
}
pub fn parchment() -> Color32 {
    Color32::from_rgb(243, 230, 208)
}
pub fn sky() -> Color32 {
    Color32::from_rgb(126, 182, 201)
}
pub fn grass() -> Color32 {
    Color32::from_rgb(62, 143, 74)
}
pub fn amber() -> Color32 {
    Color32::from_rgb(224, 160, 69)
}
pub fn clay() -> Color32 {
    Color32::from_rgb(196, 92, 74)
}
pub fn ink() -> Color32 {
    Color32::from_rgb(26, 26, 26)
}
pub fn cream() -> Color32 {
    Color32::from_rgb(244, 239, 230)
}

pub fn visuals() -> Visuals {
    let mut visuals = Visuals::light();
    visuals.panel_fill = parchment();
    visuals.window_fill = parchment();
    visuals.extreme_bg_color = Color32::from_rgb(255, 248, 236);
    visuals.faint_bg_color = Color32::from_rgb(232, 214, 186);
    visuals.widgets.noninteractive.bg_fill = parchment();
    visuals.widgets.noninteractive.fg_stroke.color = ink();
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(255, 248, 236);
    visuals.widgets.inactive.fg_stroke.color = ink();
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(255, 214, 140);
    visuals.widgets.hovered.fg_stroke.color = ink();
    visuals.widgets.active.bg_fill = amber();
    visuals.widgets.active.fg_stroke.color = ink();
    visuals.selection.bg_fill = amber();
    visuals.selection.stroke.color = ink();
    visuals.hyperlink_color = Color32::from_rgb(36, 96, 120);
    visuals.override_text_color = Some(ink());
    visuals.window_stroke.color = Color32::from_rgb(180, 156, 120);
    visuals.widgets.inactive.expansion = 1.0;
    visuals
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Workspace {
    #[default]
    Maker,
    Unmaker,
}

impl Workspace {
    pub fn flip(self) -> Self {
        match self {
            Self::Maker => Self::Unmaker,
            Self::Unmaker => Self::Maker,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Maker => "Nesticles — Maker Engine",
            Self::Unmaker => "Nesticles — UnMaker Lab",
        }
    }

    pub fn other_label(self) -> &'static str {
        match self {
            Self::Maker => "UnMaker Lab",
            Self::Unmaker => "Maker Engine",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum UnmakerView {
    #[default]
    Tiles,
    /// Shown every time the user flips into UnMaker Lab.
    Home,
    Review,
    Puzzle,
    Providers,
    Behavior,
    Assemble,
    Animate,
    Audio,
    Library,
    Notes,
    Overworld,
    Export,
    Guide,
}

/// What Exit to Desktop, or the window close button, should do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExitStep {
    /// Nothing unsaved, or the user discarded changes.
    Close,
    /// Ask before throwing work away.
    Ask,
    /// A save was requested and it did not succeed. Stay open.
    Stay,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExitChoice {
    Save,
    Discard,
    Cancel,
}

pub fn exit_step(dirty: bool, choice: Option<ExitChoice>, save_succeeded: bool) -> ExitStep {
    match choice {
        Some(ExitChoice::Cancel) => ExitStep::Stay,
        Some(ExitChoice::Discard) => ExitStep::Close,
        Some(ExitChoice::Save) => {
            if save_succeeded {
                ExitStep::Close
            } else {
                ExitStep::Stay
            }
        }
        None if dirty => ExitStep::Ask,
        None => ExitStep::Close,
    }
}

/// Editor rail tools that are real operations. Undo and redo are actions.
pub const RAIL_TOOLS: [&str; 7] = [
    "Select",
    "Paint",
    "Erase",
    "Line",
    "Vertical",
    "Rectangle",
    "Inspect",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_build_starts_fullscreen() {
        assert!(STARTS_FULLSCREEN);
    }

    #[test]
    fn flipping_is_a_view_change() {
        assert_eq!(Workspace::Maker.flip(), Workspace::Unmaker);
        assert_eq!(Workspace::Unmaker.flip(), Workspace::Maker);
        assert_eq!(Workspace::Maker.flip().flip(), Workspace::Maker);
    }

    #[test]
    fn a_clean_project_exits_without_a_prompt() {
        assert_eq!(exit_step(false, None, false), ExitStep::Close);
    }

    #[test]
    fn unsaved_work_asks_before_exit() {
        assert_eq!(exit_step(true, None, false), ExitStep::Ask);
        assert_eq!(
            exit_step(true, Some(ExitChoice::Cancel), false),
            ExitStep::Stay
        );
        assert_eq!(
            exit_step(true, Some(ExitChoice::Discard), false),
            ExitStep::Close
        );
    }

    #[test]
    fn a_failed_save_keeps_the_studio_open() {
        assert_eq!(
            exit_step(true, Some(ExitChoice::Save), false),
            ExitStep::Stay
        );
        assert_eq!(
            exit_step(false, Some(ExitChoice::Save), true),
            ExitStep::Close
        );
    }

    #[test]
    fn the_1080p_rails_leave_a_large_course() {
        let course_w = 1920.0 - RAIL_W;
        let course_h = 1080.0 - BAR_H - STRIP_H - BOTTOM_H;
        assert!(course_w > 1700.0);
        assert!(course_h > 800.0);
        assert!(THUMB >= 64.0);
        assert!(TOOL >= 48.0);
        assert!(RAIL_TOOLS.len() >= 7);
        let visible_tools = (course_h / TOOL).floor();
        assert!(visible_tools >= RAIL_TOOLS.len() as f32);
    }
}
