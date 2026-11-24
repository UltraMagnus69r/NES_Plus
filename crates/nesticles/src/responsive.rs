//! R3.1 responsive layout policy.
//!
//! Decisions use logical egui points (after DPI), not raw monitor pixels.
//! Game-world tile coordinates are never scaled by these modes.

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

    pub fn content_after_panels(self, reserved_w: f32, reserved_h: f32) -> (f32, f32) {
        (
            (self.width - reserved_w).max(1.0),
            (self.height - reserved_h).max(1.0),
        )
    }

    pub fn unmaker_nav_width(self) -> f32 {
        match self.mode {
            LayoutMode::Compact => 72.0,
            LayoutMode::Medium => 168.0,
            LayoutMode::Standard | LayoutMode::Expanded => 236.0,
        }
    }

    pub fn show_unmaker_nav_labels(self) -> bool {
        !matches!(self.mode, LayoutMode::Compact)
    }

    pub fn technical_panel_width(self) -> f32 {
        match self.mode {
            LayoutMode::Compact => 200.0,
            LayoutMode::Medium => 240.0,
            LayoutMode::Standard | LayoutMode::Expanded => 280.0,
        }
    }

    pub fn show_technical_panel(self) -> bool {
        // Compact: keep Technical details as a drawer only when the user opens it,
        // but do not auto-reserve width that would crush Investigate.
        !matches!(self.mode, LayoutMode::Compact) || self.width >= 720.0
    }

    pub fn stage_hero_height(self) -> f32 {
        match self.mode {
            LayoutMode::Compact => 72.0,
            LayoutMode::Medium => 96.0,
            LayoutMode::Standard | LayoutMode::Expanded => 122.0,
        }
    }

    pub fn discover_inspector_width(self) -> f32 {
        match self.mode {
            LayoutMode::Compact => 0.0,
            LayoutMode::Medium => 280.0,
            LayoutMode::Standard => 320.0,
            LayoutMode::Expanded => 360.0,
        }
    }

    pub fn show_discover_inspector(self, content_width: f32) -> bool {
        match self.mode {
            LayoutMode::Compact => false,
            LayoutMode::Medium => content_width >= 980.0,
            LayoutMode::Standard | LayoutMode::Expanded => content_width >= 900.0,
        }
    }

    pub fn gallery_columns(self, gallery_width: f32) -> usize {
        let card = 200.0;
        let gap = 12.0;
        if gallery_width < card {
            return 1;
        }
        let span = card + gap;
        let raw = ((gallery_width + gap) / span).floor() as usize;
        let cap = match self.mode {
            LayoutMode::Compact => 2,
            LayoutMode::Medium => 3,
            LayoutMode::Standard => 4,
            LayoutMode::Expanded => 5,
        };
        raw.clamp(1, cap)
    }

    pub fn gallery_card_width(self, gallery_width: f32) -> f32 {
        let cols = self.gallery_columns(gallery_width) as f32;
        let gap = 12.0;
        let gaps = gap * (cols - 1.0).max(0.0);
        let min_card = match self.mode {
            LayoutMode::Compact => 160.0,
            _ => 200.0,
        };
        ((gallery_width - gaps) / cols).clamp(min_card, 260.0)
    }

    pub fn maker_tools_width(self) -> f32 {
        match self.mode {
            LayoutMode::Compact => 148.0,
            LayoutMode::Medium => 180.0,
            LayoutMode::Standard | LayoutMode::Expanded => 212.0,
        }
    }

    pub fn maker_parts_width(self) -> f32 {
        match self.mode {
            LayoutMode::Compact => 160.0,
            LayoutMode::Medium => 190.0,
            LayoutMode::Standard | LayoutMode::Expanded => 220.0,
        }
    }

    pub fn page_margin(self) -> f32 {
        match self.mode {
            LayoutMode::Compact => 8.0,
            LayoutMode::Medium => 12.0,
            LayoutMode::Standard | LayoutMode::Expanded => 16.0,
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
    }

    #[test]
    fn gallery_wraps_with_width() {
        let compact = ResponsiveLayout::from_points(700.0, 500.0, 1.0);
        assert_eq!(
            compact.gallery_columns(700.0),
            2.max(compact.gallery_columns(400.0))
        );
        assert_eq!(compact.gallery_columns(180.0), 1);
        let expanded = ResponsiveLayout::from_points(1920.0, 1080.0, 1.0);
        assert!(expanded.gallery_columns(1600.0) >= 4);
        assert!(expanded.show_discover_inspector(1000.0));
        assert!(!compact.show_discover_inspector(700.0));
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
    fn dpi_does_not_change_logical_mode() {
        let a = ResponsiveLayout::from_points(1200.0, 700.0, 1.0);
        let b = ResponsiveLayout::from_points(1200.0, 700.0, 1.5);
        assert_eq!(a.mode, b.mode);
        assert_eq!(a.gallery_columns(900.0), b.gallery_columns(900.0));
    }

    #[test]
    fn maker_tools_fit_laptop() {
        let layout = ResponsiveLayout::from_points(1366.0, 694.0, 1.0);
        let tools = layout.maker_tools_width();
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
        assert!(layout.stage_hero_height() < 130.0);
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
}
