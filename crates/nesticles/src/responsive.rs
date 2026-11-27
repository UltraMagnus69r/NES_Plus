//! R3.1 / R3.1.1 responsive layout policy.
//!
//! Decisions use logical egui points (after DPI), not raw monitor pixels.
//! Game-world tile coordinates are never scaled by these modes.
//!
//! R3.1.1 correction notes (production path):
//! - Mode is still derived from full viewport width, but **density** densifies
//!   one step on short clients (height < 740) so a 1366×694 laptop does not
//!   keep desktop chrome that clips Discover and Maker.
//! - Gallery card widths never clamp above the width that actually fits.
//! - Inspector visibility and shell chrome helpers use density, not raw mode.

use eframe::egui::Context;

/// Layout density derived from the usable client width in points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    /// Below 800 points.
    Compact,
    /// 800–1199 points.
    Medium,
    /// 1200–1699 points.
    Standard,
    /// 1700 points and above.
    Expanded,
}

/// Snapshot of the current viewport for layout helpers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponsiveLayout {
    pub width: f32,
    pub height: f32,
    pub pixels_per_point: f32,
    pub mode: LayoutMode,
}

impl ResponsiveLayout {
    pub const COMPACT_MAX: f32 = 799.0;
    pub const MEDIUM_MAX: f32 = 1199.0;
    pub const STANDARD_MAX: f32 = 1699.0;
    /// Below this height, density steps down one level (laptop maximized clients).
    pub const SHORT_CLIENT_H: f32 = 740.0;

    pub fn mode_for_width(width: f32) -> LayoutMode {
        if width < 800.0 {
            LayoutMode::Compact
        } else if width < 1200.0 {
            LayoutMode::Medium
        } else if width < 1700.0 {
            LayoutMode::Standard
        } else {
            LayoutMode::Expanded
        }
    }

    pub fn from_points(width: f32, height: f32, pixels_per_point: f32) -> Self {
        let width = width.max(1.0);
        let height = height.max(1.0);
        let pixels_per_point = pixels_per_point.max(0.25);
        Self {
            width,
            height,
            pixels_per_point,
            mode: Self::mode_for_width(width),
        }
    }

    /// Prefer the inner viewport size when the platform reports it.
    pub fn from_context(ctx: &Context) -> Self {
        let (width, height, ppp) = ctx.input(|input| {
            let ppp = input.pixels_per_point;
            if let Some(inner) = input.viewport().inner_rect {
                (inner.width(), inner.height(), ppp)
            } else {
                let screen = input.screen_rect();
                (screen.width(), screen.height(), ppp)
            }
        });
        Self::from_points(width, height, ppp)
    }

    /// Density used for chrome sizes. Short clients densify so primary laptop
    /// maximized areas (~1366×694) do not keep Standard desktop padding, and
    /// restored ~800×500 windows drop to Compact shell chips instead of
    /// clipping "Maker Engine" in the top bar.
    pub fn density(self) -> LayoutMode {
        if self.height < Self::SHORT_CLIENT_H {
            match self.mode {
                LayoutMode::Expanded => LayoutMode::Standard,
                LayoutMode::Standard => LayoutMode::Medium,
                LayoutMode::Medium => LayoutMode::Compact,
                LayoutMode::Compact => LayoutMode::Compact,
            }
        } else {
            self.mode
        }
    }

    pub fn content_after_panels(self, reserved_w: f32, reserved_h: f32) -> (f32, f32) {
        (
            (self.width - reserved_w).max(1.0),
            (self.height - reserved_h).max(1.0),
        )
    }

    pub fn unmaker_nav_width(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 72.0,
            LayoutMode::Medium => 156.0,
            LayoutMode::Standard | LayoutMode::Expanded => 220.0,
        }
    }

    pub fn show_unmaker_nav_labels(self) -> bool {
        !matches!(self.density(), LayoutMode::Compact)
    }

    pub fn technical_panel_width(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 200.0,
            LayoutMode::Medium => 240.0,
            LayoutMode::Standard | LayoutMode::Expanded => 280.0,
        }
    }

    pub fn show_technical_panel(self) -> bool {
        // Compact: keep Technical details as a drawer only when the user opens it,
        // but do not auto-reserve width that would crush Investigate.
        !matches!(self.density(), LayoutMode::Compact) || self.width >= 720.0
    }

    pub fn stage_hero_height(self) -> f32 {
        let base: f32 = match self.density() {
            LayoutMode::Compact => 64.0,
            LayoutMode::Medium => 84.0,
            LayoutMode::Standard | LayoutMode::Expanded => 110.0,
        };
        if self.height < 600.0 {
            base.min(56.0)
        } else {
            base
        }
    }

    pub fn discover_inspector_width(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 0.0,
            LayoutMode::Medium => 260.0,
            LayoutMode::Standard => 300.0,
            LayoutMode::Expanded => 340.0,
        }
    }

    pub fn show_discover_inspector(self, content_width: f32) -> bool {
        match self.density() {
            LayoutMode::Compact => false,
            // Laptop Medium clients keep a drawer so the gallery keeps width.
            LayoutMode::Medium => content_width >= 1200.0,
            LayoutMode::Standard | LayoutMode::Expanded => content_width >= 1000.0,
        }
    }

    /// Columns that fit without forcing cards past the available width.
    pub fn gallery_columns(self, gallery_width: f32) -> usize {
        self.gallery_fit(gallery_width).0
    }

    pub fn gallery_card_width(self, gallery_width: f32) -> f32 {
        self.gallery_fit(gallery_width).1
    }

    /// Returns `(columns, card_width)` that never overflows `gallery_width`.
    pub fn gallery_fit(self, gallery_width: f32) -> (usize, f32) {
        let gap = 12.0;
        let min_card = match self.density() {
            LayoutMode::Compact => 148.0,
            _ => 168.0,
        };
        let ideal_card = match self.density() {
            LayoutMode::Compact => 180.0,
            LayoutMode::Medium => 200.0,
            LayoutMode::Standard | LayoutMode::Expanded => 210.0,
        };
        // Medium (laptop densified Standard) allows 4 so a ~1100pt gallery
        // is not stuck at three 260pt cards with a large empty right gutter.
        let cap = match self.density() {
            LayoutMode::Compact => 2,
            LayoutMode::Medium => 4,
            LayoutMode::Standard => 4,
            LayoutMode::Expanded => 5,
        };
        if gallery_width < min_card {
            return (1, gallery_width.max(1.0));
        }
        let mut cols =
            (((gallery_width + gap) / (ideal_card + gap)).floor() as usize).clamp(1, cap);
        loop {
            let gaps = gap * (cols.saturating_sub(1) as f32);
            let card = ((gallery_width - gaps) / cols as f32).max(1.0);
            if card + 0.5 >= min_card || cols == 1 {
                let card = card.min(260.0);
                return (cols, card);
            }
            cols -= 1;
        }
    }

    /// Width reserved for gallery when a side inspector is shown.
    /// Guarantees gallery + gap + inspector ≤ content_width.
    pub fn discover_split_widths(self, content_width: f32) -> Option<(f32, f32)> {
        if !self.show_discover_inspector(content_width) {
            return None;
        }
        let inspector = self.discover_inspector_width().min(content_width * 0.38);
        let gap = 12.0;
        let gallery = (content_width - inspector - gap).max(self.gallery_fit(160.0).1);
        if gallery + gap + inspector > content_width + 0.5 {
            return None;
        }
        Some((gallery, inspector))
    }

    pub fn maker_tools_width(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 136.0,
            LayoutMode::Medium => 168.0,
            LayoutMode::Standard | LayoutMode::Expanded => 200.0,
        }
    }

    /// Tool capsule width inside the Maker tools sidebar (accounts for margin).
    pub fn maker_tool_button_width(self) -> f32 {
        (self.maker_tools_width() - 28.0).clamp(100.0, 168.0)
    }

    pub fn maker_parts_width(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 160.0,
            LayoutMode::Medium => 190.0,
            LayoutMode::Standard | LayoutMode::Expanded => 220.0,
        }
    }

    pub fn page_margin(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 6.0,
            LayoutMode::Medium => 10.0,
            LayoutMode::Standard | LayoutMode::Expanded => 14.0,
        }
    }

    pub fn filter_chip_height(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 36.0,
            LayoutMode::Medium => 40.0,
            LayoutMode::Standard | LayoutMode::Expanded => 44.0,
        }
    }

    pub fn shell_brand_width(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 148.0,
            LayoutMode::Medium => 190.0,
            LayoutMode::Standard | LayoutMode::Expanded => 254.0,
        }
    }

    pub fn shell_workspace_chip_width(self, active: bool) -> f32 {
        match self.density() {
            LayoutMode::Compact => {
                if active {
                    148.0
                } else {
                    136.0
                }
            }
            LayoutMode::Medium => {
                if active {
                    178.0
                } else {
                    164.0
                }
            }
            LayoutMode::Standard | LayoutMode::Expanded => {
                if active {
                    214.0
                } else {
                    199.0
                }
            }
        }
    }

    pub fn shell_shows_project_chip(self) -> bool {
        self.width >= 1180.0 && !matches!(self.density(), LayoutMode::Compact)
    }

    pub fn shell_action_button_width(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 78.0,
            LayoutMode::Medium => 88.0,
            LayoutMode::Standard | LayoutMode::Expanded => 94.0,
        }
    }

    /// Vertical chrome above the Discover gallery (hero + filters + search + gaps).
    pub fn discover_chrome_budget(self) -> f32 {
        let hero = self.stage_hero_height();
        let filters = self.filter_chip_height() + 8.0;
        let search = 46.0;
        let tools = 52.0;
        let gaps = 28.0;
        hero + filters + search + tools + gaps
    }

    pub fn gallery_card_height(self) -> f32 {
        match self.density() {
            LayoutMode::Compact => 178.0,
            LayoutMode::Medium => 190.0,
            LayoutMode::Standard | LayoutMode::Expanded => 200.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoint_boundaries() {
        assert_eq!(ResponsiveLayout::mode_for_width(799.0), LayoutMode::Compact);
        assert_eq!(ResponsiveLayout::mode_for_width(800.0), LayoutMode::Medium);
        assert_eq!(ResponsiveLayout::mode_for_width(1199.0), LayoutMode::Medium);
        assert_eq!(
            ResponsiveLayout::mode_for_width(1200.0),
            LayoutMode::Standard
        );
        assert_eq!(
            ResponsiveLayout::mode_for_width(1699.0),
            LayoutMode::Standard
        );
        assert_eq!(
            ResponsiveLayout::mode_for_width(1700.0),
            LayoutMode::Expanded
        );
    }

    #[test]
    fn usable_width_stays_positive() {
        let layout = ResponsiveLayout::from_points(1366.0, 694.0, 1.0);
        let (w, h) = layout.content_after_panels(layout.unmaker_nav_width(), 84.0);
        assert!(w > 200.0);
        assert!(h > 200.0);
        assert_eq!(layout.mode, LayoutMode::Standard);
        assert_eq!(layout.density(), LayoutMode::Medium);
    }

    #[test]
    fn laptop_densifies_to_medium() {
        let layout = ResponsiveLayout::from_points(1366.0, 694.0, 1.0);
        assert_eq!(layout.density(), LayoutMode::Medium);
        assert!(layout.unmaker_nav_width() < 180.0);
        assert!(layout.stage_hero_height() <= 90.0);
        assert!(!layout.show_discover_inspector(1100.0));
    }

    #[test]
    fn short_medium_densifies_to_compact_shell() {
        let layout = ResponsiveLayout::from_points(800.0, 500.0, 1.0);
        assert_eq!(layout.mode, LayoutMode::Medium);
        assert_eq!(layout.density(), LayoutMode::Compact);
        assert!(layout.unmaker_nav_width() <= 80.0);
        assert!(!layout.show_unmaker_nav_labels());
        let brand = layout.shell_brand_width();
        let chips =
            layout.shell_workspace_chip_width(true) + layout.shell_workspace_chip_width(false);
        let actions = layout.shell_action_button_width() * 3.0;
        assert!(
            brand + chips + actions + 24.0 < layout.width,
            "compact shell still overflows 800pt"
        );
    }

    #[test]
    fn gallery_wraps_with_width() {
        let compact = ResponsiveLayout::from_points(700.0, 500.0, 1.0);
        assert_eq!(compact.gallery_columns(180.0), 1);
        let expanded = ResponsiveLayout::from_points(1920.0, 1080.0, 1.0);
        assert!(expanded.gallery_columns(1600.0) >= 4);
        assert!(expanded.show_discover_inspector(1000.0));
        assert!(!compact.show_discover_inspector(700.0));
    }

    #[test]
    fn gallery_cards_never_overflow_row() {
        for width in [160.0, 320.0, 500.0, 762.0, 900.0, 1100.0, 1600.0] {
            for &(w, h) in &[
                (700.0, 500.0),
                (1366.0, 694.0),
                (1920.0, 1080.0),
                (980.0, 600.0),
                (800.0, 500.0),
            ] {
                let layout = ResponsiveLayout::from_points(w, h, 1.0);
                let (cols, card) = layout.gallery_fit(width);
                let gaps = 12.0 * (cols.saturating_sub(1) as f32);
                assert!(
                    cols as f32 * card + gaps <= width + 0.75,
                    "overflow: viewport {w}x{h} gallery {width} cols {cols} card {card}"
                );
            }
        }
    }

    #[test]
    fn inspector_collapses_on_compact() {
        let compact = ResponsiveLayout::from_points(780.0, 600.0, 1.0);
        assert!(!compact.show_discover_inspector(780.0));
        assert_eq!(compact.discover_inspector_width(), 0.0);
        assert!(compact.unmaker_nav_width() < 100.0);
        assert!(!compact.show_unmaker_nav_labels());
    }

    #[test]
    fn discover_split_stays_inside_content() {
        let layout = ResponsiveLayout::from_points(1920.0, 1080.0, 1.0);
        let content = 1400.0;
        let Some((gallery, inspector)) = layout.discover_split_widths(content) else {
            panic!("expected split on wide desktop");
        };
        assert!(gallery + 12.0 + inspector <= content + 0.5);
        assert!(gallery >= 300.0);
        let laptop = ResponsiveLayout::from_points(1366.0, 694.0, 1.0);
        assert!(laptop.discover_split_widths(1100.0).is_none());
    }

    #[test]
    fn dpi_does_not_change_logical_mode() {
        let a = ResponsiveLayout::from_points(1200.0, 700.0, 1.0);
        let b = ResponsiveLayout::from_points(1200.0, 700.0, 1.5);
        assert_eq!(a.mode, b.mode);
        assert_eq!(a.density(), b.density());
        assert_eq!(a.gallery_columns(900.0), b.gallery_columns(900.0));
    }

    #[test]
    fn maker_tools_fit_laptop() {
        let layout = ResponsiveLayout::from_points(1366.0, 694.0, 1.0);
        let tools = layout.maker_tools_width();
        let button = layout.maker_tool_button_width();
        assert!(button + 24.0 <= tools);
        let parts = layout.maker_parts_width();
        let (content, _) = layout.content_after_panels(tools + parts, 0.0);
        assert!(content > 600.0);
        assert!(tools + parts < layout.width);
    }

    #[test]
    fn vertical_reachability_reserves_issue_space() {
        // Laptop client ~1366×694: after chrome, Investigate must still expose
        // Accept/Skip/Reject via scroll rather than clipping them away.
        let layout = ResponsiveLayout::from_points(1366.0, 694.0, 1.0);
        let (_, content_h) = layout.content_after_panels(
            layout.unmaker_nav_width(),
            84.0 + layout.stage_hero_height(),
        );
        assert!(content_h >= 400.0);
        assert!(layout.stage_hero_height() < 100.0);
        assert!(layout.discover_chrome_budget() < 320.0);
    }

    #[test]
    fn nav_allocation_compact_width() {
        let compact = ResponsiveLayout::from_points(780.0, 600.0, 1.0);
        assert!(compact.unmaker_nav_width() <= 80.0);
        let (content, _) = compact.content_after_panels(compact.unmaker_nav_width(), 84.0);
        assert!(content >= 680.0);
    }

    #[test]
    fn shell_chrome_compacts_on_laptop() {
        let layout = ResponsiveLayout::from_points(1366.0, 694.0, 1.0);
        let brand = layout.shell_brand_width();
        let chips =
            layout.shell_workspace_chip_width(true) + layout.shell_workspace_chip_width(false);
        let actions = layout.shell_action_button_width() * 3.0 + 140.0;
        let used = brand + chips + actions + 40.0;
        assert!(
            used < layout.width,
            "shell chrome {used} exceeds viewport {}",
            layout.width
        );
        assert!(!layout.shell_shows_project_chip() || layout.width >= 1180.0);
    }

    #[test]
    fn canvas_cell_mapping_ignores_window_size() {
        // Game-world tile picking uses EDITOR_CELL_PX, not the desktop viewport.
        // Resizing the window must not change which cell a pointer hits.
        use nesticles_core::terrain::pointer_cell;
        let a = pointer_cell(41.0, 21.0);
        let _compact = ResponsiveLayout::from_points(800.0, 600.0, 1.0);
        let _expanded = ResponsiveLayout::from_points(3840.0, 2160.0, 1.5);
        assert_eq!(a, pointer_cell(41.0, 21.0));
        assert_eq!(a, (2, 1));
    }

    #[test]
    fn mode_switch_preserves_policy_identity() {
        // Switching compact↔expanded changes density only; helpers stay deterministic.
        let compact = ResponsiveLayout::from_points(780.0, 600.0, 1.0);
        let expanded = ResponsiveLayout::from_points(1800.0, 1000.0, 1.0);
        assert_ne!(compact.mode, expanded.mode);
        assert!(compact.unmaker_nav_width() < expanded.unmaker_nav_width());
        assert!(compact.maker_tools_width() <= expanded.maker_tools_width());
        assert_eq!(
            ResponsiveLayout::from_points(780.0, 600.0, 1.0).mode,
            LayoutMode::Compact
        );
    }

    #[test]
    fn viewport_transition_recomputes_each_call() {
        let a = ResponsiveLayout::from_points(800.0, 500.0, 1.0);
        let b = ResponsiveLayout::from_points(1920.0, 1080.0, 1.0);
        assert_ne!(a.density(), b.density());
        assert!(a.gallery_columns(700.0) <= b.gallery_columns(1500.0));
        assert!(a.stage_hero_height() < b.stage_hero_height());
    }
}
