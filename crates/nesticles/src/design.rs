//! Shared Nesticles widgets. Tokens live in `studio`. These painters do not
//! copy mockup artwork and they do not own sprite or animation data.

use crate::studio;
use eframe::egui::{self, Color32, CornerRadius, Rect, RichText, Stroke, Vec2};

pub fn soft_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 3],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(32),
    }
}

pub fn floating_frame(fill: Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::same(16))
        .inner_margin(12.0)
        .stroke(Stroke::new(1.0_f32, studio::unmaker_line()))
        .shadow(soft_shadow())
}

pub fn screen_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .size(28.0)
            .strong()
            .color(studio::unmaker_ink()),
    );
    if !subtitle.is_empty() {
        ui.label(
            RichText::new(subtitle)
                .size(14.0)
                .color(studio::caption()),
        );
    }
    ui.add_space(8.0);
}

pub fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(primary_widget(label))
}

pub fn primary_widget(label: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(label).size(16.0).color(Color32::WHITE))
        .fill(studio::nesticles_red())
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(12))
        .min_size(Vec2::new(180.0, 40.0))
}

pub fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(16.0).color(studio::unmaker_ink()))
            .fill(studio::unmaker_paper())
            .stroke(Stroke::new(1.0_f32, studio::unmaker_line()))
            .corner_radius(CornerRadius::same(12))
            .min_size(Vec2::new(120.0, 40.0)),
    )
}

pub fn status_chip(ui: &mut egui::Ui, text: &str) {
    let (fill, fg) = chip_colors(text);
    egui::Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).color(fg));
        });
}

pub fn chip_colors(text: &str) -> (Color32, Color32) {
    let lower = text.to_ascii_lowercase();
    if lower.contains("fail") || lower.contains("error") {
        (Color32::from_rgb(255, 228, 226), studio::nesticles_red())
    } else if lower.contains("not reconstructed")
        || lower.contains("needs")
        || lower.contains("unresolved")
    {
        (Color32::from_rgb(226, 238, 255), studio::apple_blue())
    } else if lower.contains("provisional")
        || lower.contains("not verified")
        || lower.contains("progress")
    {
        (
            Color32::from_rgb(255, 244, 214),
            Color32::from_rgb(140, 96, 0),
        )
    } else if lower.contains("reconstructed") || lower.contains("identified") {
        (Color32::from_rgb(220, 245, 226), studio::game_green())
    } else {
        (studio::panel(), studio::caption())
    }
}

/// Original geometric mark. Not a game sprite.
pub fn paint_egg(painter: &egui::Painter, rect: Rect) {
    let center = rect.center();
    painter.circle_filled(center, rect.width() * 0.42, studio::accent_yellow());
    painter.circle_stroke(
        center,
        rect.width() * 0.42,
        Stroke::new(1.0_f32, Color32::from_rgb(214, 160, 0)),
    );
    for offset in [
        Vec2::new(-3.0, -1.0),
        Vec2::new(3.0, 2.0),
        Vec2::new(-1.0, 4.0),
    ] {
        painter.circle_filled(center + offset, 1.3, Color32::from_rgb(176, 112, 32));
    }
}

/// Original geometric mark. Not a game sprite.
pub fn paint_castle(painter: &egui::Painter, rect: Rect) {
    let color = studio::nesticles_red();
    let base = Rect::from_min_max(
        rect.left_bottom() + Vec2::new(rect.width() * 0.18, -rect.height() * 0.42),
        rect.right_bottom() + Vec2::new(-rect.width() * 0.18, -2.0),
    );
    painter.rect_filled(base, CornerRadius::same(2), color);
    for tower in [0.12_f32, 0.40, 0.68] {
        let tower = Rect::from_min_max(
            rect.min + Vec2::new(rect.width() * tower, rect.height() * 0.18),
            rect.min + Vec2::new(rect.width() * (tower + 0.2), rect.height() * 0.72),
        );
        painter.rect_filled(tower, CornerRadius::same(2), studio::accent_purple());
    }
}

pub fn paint_mark(painter: &egui::Painter, rect: Rect, maker: bool) {
    if maker {
        paint_castle(painter, rect);
    } else {
        paint_egg(painter, rect);
    }
}

#[cfg(test)]
mod tests {
    use super::chip_colors;
    use crate::studio;

    #[test]
    fn not_reconstructed_is_not_a_success_color() {
        let (fill, _) = chip_colors("Artwork not reconstructed · Needs a person");
        assert_ne!(fill, chip_colors("Reconstructed").0);
        let (_, fg) = chip_colors("Reconstructed");
        assert_eq!(fg, studio::game_green());
    }
}
