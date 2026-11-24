//! Shared visual tokens and workspace decisions for the two Nesticles sides.
//! Maker Engine and UnMaker Lab both read these values.

use eframe::egui::{Color32, Visuals};

/// The desktop build opens as a decorated window. Fullscreen is never automatic.
pub const STARTS_FULLSCREEN: bool = false;

/// Inner size for a 1366×768 desktop. The title bar and panel stay outside this.
pub const INITIAL_INNER: [f32; 2] = [1200.0, 640.0];

/// Smallest inner size. Compact mode starts below 800 points; keep a usable floor.
pub const MIN_INNER: [f32; 2] = [800.0, 500.0];

/// Window geometry file. This is not the alpha library directory.
pub const WINDOW_APP_ID: &str = "nesticles-window";

/// R2.3 visual system. One token set, two workspace surfaces.
pub const FUTURE_UI: &str =
    "Design Language 2.1: glossy layered surfaces, playful scenery, premium rounded controls";

pub const BAR_H: f32 = 84.0;
pub const STRIP_H: f32 = 112.0;
pub const RAIL_W: f32 = 80.0;
pub const BOTTOM_H: f32 = 56.0;
pub const THUMB: f32 = 68.0;
pub const TOOL: f32 = 46.0;
pub const INSPECTOR_W: f32 = 360.0;

/// Discover card width used to choose a column count. Cards may grow slightly
/// to fill a row. They do not shrink below this on Standard+ layouts.
pub const GALLERY_CARD_W: f32 = 200.0;
pub const GALLERY_GAP: f32 = 12.0;

/// Content width at which Discover keeps a side inspector (Standard mode floor).
/// Prefer [`crate::responsive::ResponsiveLayout::show_discover_inspector`].
pub const INSPECTOR_BREAK: f32 = 1200.0;

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

pub fn nesticles_red() -> Color32 {
    Color32::from_rgb(229, 57, 53)
}

pub fn unmaker_red() -> Color32 {
    nesticles_red()
}

pub fn apple_blue() -> Color32 {
    Color32::from_rgb(10, 132, 255)
}

pub fn game_green() -> Color32 {
    Color32::from_rgb(52, 199, 89)
}

pub fn accent_yellow() -> Color32 {
    Color32::from_rgb(255, 204, 0)
}

pub fn accent_purple() -> Color32 {
    Color32::from_rgb(175, 82, 222)
}

pub fn unmaker_ink() -> Color32 {
    Color32::from_rgb(28, 28, 30)
}

pub fn caption() -> Color32 {
    Color32::from_rgb(70, 78, 92)
}

pub fn unmaker_line() -> Color32 {
    Color32::from_rgb(214, 227, 240)
}

pub fn unmaker_paper() -> Color32 {
    Color32::from_rgb(255, 255, 255)
}

pub fn unmaker_mist() -> Color32 {
    Color32::from_rgb(238, 245, 255)
}

pub fn panel() -> Color32 {
    Color32::from_rgb(225, 236, 252)
}

pub fn maker_paper() -> Color32 {
    Color32::from_rgb(255, 249, 239)
}

pub fn maker_mist() -> Color32 {
    Color32::from_rgb(251, 233, 215)
}

fn round_controls(visuals: &mut Visuals) {
    let radius = eframe::egui::CornerRadius::same(18);
    visuals.widgets.noninteractive.corner_radius = radius;
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.active.corner_radius = radius;
    visuals.widgets.open.corner_radius = radius;
}

/// Shared control colors. UnMaker uses the cool surface. Maker uses the warm one.
pub fn unmaker_visuals() -> Visuals {
    let mut visuals = Visuals::light();
    let ink = unmaker_ink();
    visuals.panel_fill = unmaker_mist();
    visuals.window_fill = unmaker_paper();
    visuals.extreme_bg_color = panel();
    visuals.faint_bg_color = panel();
    visuals.widgets.noninteractive.bg_fill = unmaker_paper();
    visuals.widgets.noninteractive.fg_stroke.color = ink;
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(249, 251, 255);
    visuals.widgets.inactive.fg_stroke.color = ink;
    visuals.widgets.inactive.bg_stroke.color = unmaker_line();
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(224, 240, 255);
    visuals.widgets.hovered.fg_stroke.color = ink;
    visuals.widgets.hovered.bg_stroke.color = apple_blue();
    visuals.widgets.active.bg_fill = Color32::from_rgb(33, 138, 255);
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    visuals.widgets.active.bg_stroke.color = apple_blue();
    visuals.selection.bg_fill = Color32::from_rgb(232, 242, 255);
    visuals.selection.stroke.color = apple_blue();
    visuals.hyperlink_color = apple_blue();
    visuals.override_text_color = Some(ink);
    visuals.window_stroke.color = unmaker_line();
    visuals.warn_fg_color = Color32::from_rgb(140, 96, 0);
    visuals.error_fg_color = nesticles_red();
    visuals.window_shadow = eframe::egui::epaint::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(42),
    };
    visuals.popup_shadow = eframe::egui::epaint::Shadow {
        offset: [0, 5],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(36),
    };
    round_controls(&mut visuals);
    visuals
}

/// Maker Engine keeps the same actions and uses a warmer panel.
pub fn visuals() -> Visuals {
    let mut visuals = unmaker_visuals();
    visuals.panel_fill = maker_paper();
    visuals.window_fill = maker_paper();
    visuals.widgets.noninteractive.bg_fill = maker_paper();
    visuals.widgets.inactive.bg_fill = maker_paper();
    visuals.faint_bg_color = maker_mist();
    visuals
}

/// Course canvas behind an empty area. Placed cells still use the course sky.
pub fn canvas() -> Color32 {
    Color32::from_rgb(186, 222, 236)
}

/// How many Discover cards fit. Uses the R3.1 responsive column caps.
pub fn gallery_columns(width: f32) -> usize {
    crate::responsive::ResponsiveLayout::from_points(width, 700.0, 1.0).gallery_columns(width)
}

pub fn gallery_card_width(width: f32) -> f32 {
    crate::responsive::ResponsiveLayout::from_points(width, 700.0, 1.0).gallery_card_width(width)
}

pub fn discover_shows_inspector(content_width: f32) -> bool {
    crate::responsive::ResponsiveLayout::from_points(content_width, 700.0, 1.0)
        .show_discover_inspector(content_width)
}

/// Clamp a restored inner size so a 4K-saved geometry remains usable on a laptop.
pub fn clamped_inner_size(desired: [f32; 2], monitor: [f32; 2]) -> [f32; 2] {
    let mw = monitor[0].max(1.0);
    let mh = monitor[1].max(1.0);
    // Leave room for window decorations and a KDE panel (~74px historically).
    let max_w = (mw - 16.0).max(MIN_INNER[0]);
    let max_h = (mh - 80.0).max(MIN_INNER[1]);
    [
        desired[0].clamp(MIN_INNER[0], max_w),
        desired[1].clamp(MIN_INNER[1], max_h),
    ]
}

/// Graphical presentation of a course. Only Classic is implemented.
/// Choosing another value must not rewrite assets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GameStyle {
    #[default]
    Classic8Bit,
    SixteenBit,
    ModernPixel,
    RetroPalette,
}

impl GameStyle {
    pub fn all() -> [Self; 4] {
        [
            Self::Classic8Bit,
            Self::SixteenBit,
            Self::ModernPixel,
            Self::RetroPalette,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Classic8Bit => "Classic 8-bit",
            Self::SixteenBit => "16-bit",
            Self::ModernPixel => "Modern Pixel",
            Self::RetroPalette => "Retro Palette",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::Classic8Bit => "The style this build actually draws.",
            Self::SixteenBit => "Planned. This build cannot draw a 16-bit style.",
            Self::ModernPixel => "Planned. This build cannot draw a modern pixel style.",
            Self::RetroPalette => "Planned. This build cannot recolor the course.",
        }
    }

    pub fn selectable(self) -> bool {
        matches!(self, Self::Classic8Bit)
    }
}

/// Returns the style to keep. An unavailable request leaves the current style
/// alone and explains why.
pub fn choose_game_style(
    current: GameStyle,
    requested: GameStyle,
) -> Result<GameStyle, &'static str> {
    if requested.selectable() {
        Ok(requested)
    } else {
        let _ = current;
        Err("That style is planned. The course was not changed.")
    }
}

pub const PART_GROUPS: [&str; 6] = ["All", "Terrain", "Players", "Enemies", "Items", "Other"];

pub fn show_terrain_brush(group: &str) -> bool {
    group == "All" || group == "Terrain"
}

pub fn show_asset_kind(group: &str, kind_label: &str) -> bool {
    match group {
        "Terrain" => false,
        "Players" => kind_label == "Player",
        "Enemies" => kind_label == "Enemy",
        "Items" => kind_label == "Item",
        "Other" => !matches!(kind_label, "Player" | "Enemy" | "Item"),
        _ => true,
    }
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

/// Move a restored window back if fewer than 48 points remain on the monitor.
/// A `None` result means the saved place is still usable. Maximized windows are
/// left to the window manager.
pub fn restored_origin(
    outer_min: [f32; 2],
    outer_size: [f32; 2],
    monitor: [f32; 2],
) -> Option<[f32; 2]> {
    let (x, y) = (outer_min[0], outer_min[1]);
    let (w, h) = (outer_size[0], outer_size[1]);
    let (mw, mh) = (monitor[0], monitor[1]);
    if mw <= 0.0 || mh <= 0.0 || w <= 0.0 || h <= 0.0 {
        return None;
    }
    let visible_w = (x + w).min(mw) - x.max(0.0);
    let visible_h = (y + h).min(mh) - y.max(0.0);
    if visible_w >= 48.0 && visible_h >= 48.0 {
        return None;
    }
    let nx = ((mw - w) * 0.5).clamp(8.0, (mw - 64.0).max(8.0));
    let ny = ((mh - h) * 0.5).clamp(8.0, (mh - 64.0).max(8.0));
    Some([nx, ny])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maker_uses_the_same_actions_and_a_warmer_surface() {
        assert_eq!(
            visuals().widgets.active.bg_fill,
            unmaker_visuals().widgets.active.bg_fill
        );
        assert_ne!(visuals().panel_fill, unmaker_visuals().panel_fill);
        // R6.1 uses the glossy blue active fill. The Nesticles red token remains
        // for primary actions and is no longer the widget active color.
        assert_eq!(
            unmaker_visuals().widgets.active.bg_fill,
            eframe::egui::Color32::from_rgb(33, 138, 255)
        );
        assert_ne!(unmaker_visuals().widgets.active.bg_fill, nesticles_red());
    }

    #[test]
    fn discover_wraps_instead_of_growing_one_row() {
        // R3.1: Medium caps at 3; Standard at 4; Expanded at 5.
        assert_eq!(gallery_columns(1600.0), 4);
        assert_eq!(gallery_columns(1800.0), 5);
        assert_eq!(gallery_columns(846.0), 3);
        assert!(gallery_columns(760.0) <= 2);
        assert_eq!(gallery_columns(400.0), 1);
        assert!(discover_shows_inspector(1300.0));
        assert!(!discover_shows_inspector(780.0));
        assert!(!discover_shows_inspector(760.0));
    }

    #[test]
    fn restored_inner_size_clamps_to_laptop() {
        let sized = clamped_inner_size([2400.0, 1400.0], [1366.0, 768.0]);
        assert!(sized[0] <= 1366.0);
        assert!(sized[1] <= 768.0);
        assert!(sized[0] >= MIN_INNER[0]);
        assert!(sized[1] >= MIN_INNER[1]);
    }

    #[test]
    fn only_classic_style_can_be_selected() {
        assert!(GameStyle::Classic8Bit.selectable());
        assert!(!GameStyle::SixteenBit.selectable());
        assert_eq!(
            choose_game_style(GameStyle::Classic8Bit, GameStyle::Classic8Bit),
            Ok(GameStyle::Classic8Bit)
        );
        assert!(choose_game_style(GameStyle::Classic8Bit, GameStyle::ModernPixel).is_err());
        assert!(choose_game_style(GameStyle::Classic8Bit, GameStyle::SixteenBit).is_err());
        assert!(choose_game_style(GameStyle::Classic8Bit, GameStyle::RetroPalette).is_err());
    }

    #[test]
    fn the_part_tray_filters_without_inventing_parts() {
        assert!(show_terrain_brush("All"));
        assert!(show_terrain_brush("Terrain"));
        assert!(!show_terrain_brush("Enemies"));
        assert!(show_asset_kind("Enemies", "Enemy"));
        assert!(!show_asset_kind("Enemies", "Player"));
        assert!(!show_asset_kind("Terrain", "Item"));
        assert!(show_asset_kind("Other", "Hazard"));
    }

    #[test]
    fn the_desktop_build_starts_in_a_window() {
        assert!(!STARTS_FULLSCREEN);
        assert!(INITIAL_INNER[0] < 1366.0);
        assert!(INITIAL_INNER[1] < 700.0);
        assert!(MIN_INNER[0] <= INITIAL_INNER[0]);
        assert!(MIN_INNER[1] <= INITIAL_INNER[1]);
        assert_eq!(WINDOW_APP_ID, "nesticles-window");
        assert_ne!(WINDOW_APP_ID, "nesticles");
    }

    #[test]
    fn an_on_screen_window_is_not_moved() {
        assert_eq!(
            restored_origin([40.0, 40.0], [800.0, 600.0], [1366.0, 768.0]),
            None
        );
    }

    #[test]
    fn an_off_screen_window_is_brought_back() {
        let origin = restored_origin([-4000.0, -4000.0], [800.0, 600.0], [1366.0, 768.0]);
        let origin = origin.expect("off-screen geometry should move");
        assert!(origin[0] >= 0.0 && origin[0] < 1366.0);
        assert!(origin[1] >= 0.0 && origin[1] < 768.0);
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
    fn the_course_stays_the_large_surface() {
        let client_w = 1366.0;
        let client_h = 694.0;
        let course_h = client_h - BAR_H - BOTTOM_H;
        assert!(course_h / client_h > 0.75);
        assert!(client_w > INSPECTOR_W * 3.0);
        assert!(THUMB >= 56.0);
        assert!(TOOL >= 40.0);
        assert!(RAIL_TOOLS.len() >= 7);
    }
}
