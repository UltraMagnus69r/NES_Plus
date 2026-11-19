//! Shared Nesticles widgets. Tokens live in `studio`. These painters do not
//! copy mockup artwork and they do not own sprite or animation data.

use crate::studio;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Stroke, Vec2};

pub fn soft_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: [0, 8],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(32),
    }
}

pub fn floating_frame(fill: Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::same(22))
        .inner_margin(18.0)
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
    capsule_button(ui, label, None, (label.chars().count() as f32 * 9.0 + 46.0).max(188.0),
        48.0, CapsuleTone::Red)
}

pub fn primary_widget(label: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(label).size(17.0).strong().color(Color32::WHITE))
        .fill(studio::nesticles_red())
        .stroke(Stroke::new(1.0, Color32::from_rgb(192, 35, 44)))
        .corner_radius(CornerRadius::same(17))
        .min_size(Vec2::new(188.0, 48.0))
}

pub fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    capsule_button(ui, label, None, (label.chars().count() as f32 * 8.7 + 36.0).max(108.0),
        42.0, CapsuleTone::White)
}

pub fn status_chip(ui: &mut egui::Ui, text: &str) {
    let (fill, fg) = chip_colors(text);
    egui::Frame::NONE
        .fill(fill)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).strong().color(fg));
        });
}

pub fn chip_colors(text: &str) -> (Color32, Color32) {
    let lower = text.to_ascii_lowercase();
    if lower.contains("fail") || lower.contains("error") {
        (Color32::from_rgb(255, 228, 226), studio::nesticles_red())
    } else if lower.contains("not reconstructed")
        || lower.contains("needs")
        || lower.contains("unresolved")
        || lower.contains("candidate")
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

/// Gradient-tinted beveled surface for selected navigation, primary actions,
/// and tactile tool controls. All shapes are native egui paint operations.
/// No textures, remote images, or externally licensed UI assets are needed.
pub fn glossy_surface(painter: &egui::Painter, rect: Rect, color: Color32, selected: bool) {
    let radius = CornerRadius::same(18);
    let dark = color.gamma_multiply(if selected { 0.85 } else { 0.93 });
    painter.rect_filled(rect, radius, dark);
    let top = Rect::from_min_max(
        rect.min + Vec2::new(1.5, 1.5),
        Pos2::new(rect.right() - 1.5, rect.center().y + 2.0),
    );
    painter.rect_filled(top, CornerRadius::same(17), color);
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(
            1.0,
            if selected {
                color.gamma_multiply(0.65)
            } else {
                studio::unmaker_line()
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            rect.left_top() + Vec2::new(14.0, 2.0),
            rect.right_top() + Vec2::new(-14.0, 2.0),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 190)),
    );
    painter.line_segment(
        [
            rect.left_bottom() + Vec2::new(16.0, -2.0),
            rect.right_bottom() + Vec2::new(-16.0, -2.0),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(25, 60, 96, 26)),
    );
}

/// A decorative, intentionally non-gameplay landscape. It is never used as a
/// reconstructed ROM sprite, a Maker placement cell, or an exported asset.
pub fn paint_scenery(painter: &egui::Painter, rect: Rect, subtle: bool) {
    if rect.width() < 16.0 || rect.height() < 16.0 {
        return;
    }
    let painter = painter.with_clip_rect(rect);
    let sky_top = if subtle {
        Color32::from_rgb(239, 246, 255)
    } else {
        Color32::from_rgb(121, 204, 252)
    };
    let sky_bottom = if subtle {
        Color32::from_rgb(218, 236, 255)
    } else {
        Color32::from_rgb(179, 235, 255)
    };
    let steps = 28;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let lerp = |a: u8, b: u8, t: f32| -> u8 {
            (a as f32 + (b as f32 - a as f32) * t).round() as u8
        };
        let color = Color32::from_rgb(
            lerp(sky_top.r(), sky_bottom.r(), t0),
            lerp(sky_top.g(), sky_bottom.g(), t0),
            lerp(sky_top.b(), sky_bottom.b(), t0),
        );
        let y = rect.top() + rect.height() * t0;
        let h = rect.height() * (t1 - t0) + 1.0;
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), h)),
            0.0,
            color,
        );
    }

    let cloud = if subtle {
        Color32::from_rgba_unmultiplied(255, 255, 255, 170)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 228)
    };
    for (x, y, s) in [(0.18_f32, 0.28_f32, 0.08_f32), (0.55, 0.22, 0.06), (0.82, 0.34, 0.09)] {
        let c = Pos2::new(rect.left() + rect.width() * x, rect.top() + rect.height() * y);
        let r = rect.height().min(rect.width()) * s;
        painter.circle_filled(c + Vec2::new(-r * 0.95, r * 0.18), r * 0.72, cloud);
        painter.circle_filled(c + Vec2::new(0.0, -r * 0.12), r, cloud);
        painter.circle_filled(c + Vec2::new(r * 0.95, r * 0.18), r * 0.74, cloud);
    }

    let hill_back = if subtle {
        Color32::from_rgb(181, 229, 219)
    } else {
        Color32::from_rgb(106, 216, 179)
    };
    let hill_mid = if subtle {
        Color32::from_rgb(152, 221, 191)
    } else {
        Color32::from_rgb(144, 224, 122)
    };
    let hill_front = if subtle {
        Color32::from_rgb(196, 233, 145)
    } else {
        Color32::from_rgb(191, 230, 83)
    };
    for (x, y, r, color) in [
        (-0.02_f32, 1.18_f32, 0.54_f32, hill_back),
        (0.42, 1.12, 0.46, hill_back),
        (0.98, 1.18, 0.56, hill_back),
        (0.14, 1.32, 0.40, hill_mid),
        (0.74, 1.30, 0.46, hill_mid),
        (0.10, 1.44, 0.30, hill_front),
        (0.78, 1.43, 0.35, hill_front),
    ] {
        let center = Pos2::new(rect.left() + rect.width() * x, rect.top() + rect.height() * y);
        painter.circle_filled(center, rect.height() * r, color);
    }

    if !subtle {
        let ground_h = rect.height() * 0.12;
        let grass_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - ground_h * 0.9),
            Pos2::new(rect.right(), rect.bottom()),
        );
        painter.rect_filled(grass_rect, 0.0, Color32::from_rgb(99, 215, 84));
        let dirt_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - ground_h * 0.55),
            Pos2::new(rect.right(), rect.bottom()),
        );
        painter.rect_filled(dirt_rect, 0.0, Color32::from_rgb(234, 161, 30));
        let diamond = Color32::from_rgb(201, 123, 12);
        let cell = (rect.width() / 26.0).max(24.0);
        let start_y = rect.bottom() - ground_h * 0.48;
        let mut x = rect.left() - cell;
        while x < rect.right() + cell {
            let p1 = Pos2::new(x + cell * 0.5, start_y + cell * 0.15);
            let p2 = Pos2::new(x + cell, start_y + cell * 0.45);
            let p3 = Pos2::new(x + cell * 0.5, start_y + cell * 0.75);
            let p4 = Pos2::new(x, start_y + cell * 0.45);
            painter.add(egui::Shape::convex_polygon(
                vec![p1, p2, p3, p4],
                Color32::from_rgb(247, 185, 45),
                Stroke::new(1.0, diamond),
            ));
            x += cell;
        }
    }
}

/// Illustrated original yellow speckled egg brand mark.
pub fn paint_egg(painter: &egui::Painter, rect: Rect) {
    let center = rect.center();
    let scale = rect.width().min(rect.height()) / 32.0;
    let egg_points: Vec<Pos2> = (0..40)
        .map(|i| {
            let theta = std::f32::consts::TAU * i as f32 / 40.0;
            let sy = theta.sin();
            let rx = if sy < 0.0 { 10.2 } else { 12.9 };
            Pos2::new(
                center.x + theta.cos() * rx * scale,
                center.y + sy * 14.6 * scale,
            )
        })
        .collect();
    painter.add(egui::Shape::convex_polygon(
        egg_points,
        Color32::from_rgb(255, 235, 122),
        Stroke::new(2.1 * scale, Color32::from_rgb(116, 63, 21)),
    ));
    painter.circle_filled(
        center + Vec2::new(-4.1 * scale, -6.0 * scale),
        3.1 * scale,
        Color32::from_rgb(255, 251, 218),
    );
    for (x, y, r) in [(-3.8, 3.4, 2.7), (5.2, -1.4, 3.4), (2.3, 8.0, 2.1), (-1.0, -10.3, 1.3)] {
        painter.circle_filled(
            center + Vec2::new(x * scale, y * scale),
            r * scale,
            Color32::from_rgb(243, 166, 26),
        );
    }
}

/// Original castle mark with towers, banners and an arched entry.
pub fn paint_castle(painter: &egui::Painter, rect: Rect) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) / 32.0;
    let base = Rect::from_center_size(c + Vec2::new(0.0, 6.0 * s), Vec2::new(26.0 * s, 15.0 * s));
    painter.rect_filled(base, CornerRadius::same(2), Color32::from_rgb(222, 130, 47));
    for x in [-10.0_f32, 8.0] {
        let tower = Rect::from_min_size(c + Vec2::new(x * s, -8.0 * s), Vec2::new(7.0 * s, 20.0 * s));
        painter.rect_filled(tower, CornerRadius::same(1), Color32::from_rgb(240, 171, 77));
        for n in 0..3 {
            let p = tower.left_top() + Vec2::new(n as f32 * 2.7 * s, -3.0 * s);
            painter.rect_filled(
                Rect::from_min_size(p, Vec2::new(2.8 * s, 4.0 * s)),
                0.0,
                Color32::from_rgb(176, 83, 42),
            );
        }
    }
    painter.rect_filled(
        Rect::from_center_size(c + Vec2::new(0.0, 8.9 * s), Vec2::new(7.0 * s, 9.0 * s)),
        CornerRadius::same(4),
        Color32::from_rgb(87, 61, 96),
    );
    painter.line_segment(
        [c + Vec2::new(-7.0 * s, -13.0 * s), c + Vec2::new(-7.0 * s, -21.0 * s)],
        Stroke::new(1.3 * s, Color32::from_rgb(70, 47, 55)),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            c + Vec2::new(-6.5 * s, -20.5 * s),
            c + Vec2::new(1.0 * s, -18.0 * s),
            c + Vec2::new(-6.5 * s, -15.5 * s),
        ],
        Color32::from_rgb(226, 52, 64),
        Stroke::NONE,
    ));
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


/// Visual states matching the Nesticles button reference: high-radius bevel,
/// soft drop shadow, bright top gloss, strong centered readable typography.
#[derive(Clone, Copy, Debug)]
pub enum CapsuleTone {
    White,
    Blue,
    Red,
    Green,
    Yellow,
}

/// Drop-in style for secondary controls throughout the legacy editor. This
/// uses egui's Button widget so keyboard focus, disabled state and screen-reader
/// semantics continue to work exactly as before.
pub fn soft_button(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>) -> egui::Response {
    let fill = if ui.is_enabled() { Color32::from_rgb(250, 252, 255) }
               else { Color32::from_rgb(240, 243, 248) };
    ui.add(
        egui::Button::new(label)
            .fill(fill)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(201, 218, 244)))
            .corner_radius(CornerRadius::same(16))
            .min_size(Vec2::new(68.0, 36.0))
    )
}

fn rgb_lerp(a: Color32, b: Color32, f: f32) -> Color32 {
    let f = f.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| ((x as f32) + (y as f32 - x as f32) * f).round() as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

/// A native egui custom button (no HTML): polished blue / white / red / green
/// / yellow capsule surfaces, smooth gradient, specular edge, restrained soft
/// shadow, centered text and any supplied asset icon. The entire icon+label
/// group is centered mathematically. All controls keep real click handling.
pub fn capsule_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<&egui::TextureHandle>,
    width: f32,
    height: f32,
    tone: CapsuleTone,
) -> egui::Response {
    let size = Vec2::new(width.max(44.0), height.max(30.0));
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let pressed = response.is_pointer_button_down_on();
    let hovered = response.hovered();
    let (mut upper, mut lower, rim, fg) = match tone {
        CapsuleTone::White => (
            Color32::from_rgb(255, 255, 255), Color32::from_rgb(229, 238, 255),
            Color32::from_rgb(195, 215, 242), studio::unmaker_ink(),
        ),
        CapsuleTone::Blue => (
            Color32::from_rgb(73, 199, 255), Color32::from_rgb(0, 104, 235),
            Color32::from_rgb(22, 118, 235), Color32::WHITE,
        ),
        CapsuleTone::Red => (
            Color32::from_rgb(255, 145, 136), Color32::from_rgb(225, 24, 42),
            Color32::from_rgb(218, 37, 45), Color32::WHITE,
        ),
        CapsuleTone::Green => (
            Color32::from_rgb(104, 237, 124), Color32::from_rgb(7, 157, 52),
            Color32::from_rgb(25, 153, 57), Color32::WHITE,
        ),
        CapsuleTone::Yellow => (
            Color32::from_rgb(255, 254, 230), Color32::from_rgb(255, 218, 145),
            Color32::from_rgb(246, 71, 70), studio::unmaker_ink(),
        ),
    };
    if hovered && !pressed {
        upper = rgb_lerp(upper, Color32::WHITE, 0.10);
        lower = rgb_lerp(lower, Color32::WHITE, 0.065);
    }
    if pressed {
        upper = upper.gamma_multiply(0.87);
        lower = lower.gamma_multiply(0.90);
    }
    let r = (height * 0.46).min(height * 0.5 - 0.25);
    let rounding = CornerRadius::same(r as u8);
    let painter = ui.painter();
    // Four outside highlights create the soft lower-right ambient shadow.
    let shadow_rect = rect.translate(Vec2::new(0.0, if pressed { 1.0 } else { 3.0 }));
    painter.rect_filled(shadow_rect.expand(2.0), rounding,
        Color32::from_rgba_unmultiplied(34, 81, 153, 22));
    painter.rect_filled(shadow_rect.expand(0.7), rounding,
        Color32::from_rgba_unmultiplied(41, 90, 162, 19));
    // Each horizontal band is clipped to the mathematical pill silhouette;
    // the gradient has no visible flat half-tone seam.
    let top = rect.top() + 1.0;
    let bottom = rect.bottom() - 1.0;
    let bands = ((bottom - top).ceil() as usize).max(24);
    for i in 0..bands {
        let y0 = top + (bottom - top) * i as f32 / bands as f32;
        let y1 = top + (bottom - top) * (i + 1) as f32 / bands as f32 + 0.35;
        let mid = ((y0 + y1) * 0.5 - top).clamp(0.0, bottom - top);
        let inset = if mid < r {
            r - (r*r - (r-mid)*(r-mid)).max(0.0).sqrt()
        } else if mid > (bottom - top) - r {
            let d = (bottom - top) - mid;
            r - (r*r - (r-d)*(r-d)).max(0.0).sqrt()
        } else { 0.0 };
        let factor = i as f32 / (bands.saturating_sub(1).max(1)) as f32;
        let mut fill = rgb_lerp(upper, lower, factor.powf(0.86));
        if factor < 0.35 { fill = rgb_lerp(fill, Color32::WHITE, (0.35-factor)*0.20); }
        let x0 = rect.left() + 1.0 + inset;
        let x1 = rect.right() - 1.0 - inset;
        if x1 > x0 {
            painter.rect_filled(Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)), 0.0, fill);
        }
    }
    painter.rect_stroke(rect, rounding, Stroke::new(1.3_f32, rim), egui::StrokeKind::Inside);
    painter.line_segment(
        [rect.left_top() + Vec2::new(15.0, 2.3), rect.right_top() + Vec2::new(-15.0, 2.3)],
        Stroke::new(1.25_f32, Color32::from_rgba_unmultiplied(255,255,255,175)),
    );

    let font_size = if height < 42.0 { 14.0 } else if width < 108.0 { 15.0 } else { 16.5 };
    let font = egui::FontId::proportional(font_size);
    let label_width = painter.layout_no_wrap(label.to_string(), font.clone(), fg).size().x;
    let drawn_icon = icon.is_none() && matches!(label, "Menu" | "Exit");
    let mut icon_w = if icon.is_some() || drawn_icon {
        (height * 0.59).clamp(20.0, 35.0)
    } else { 0.0 };
    let mut gap = if icon.is_some() || drawn_icon { 7.0 } else { 0.0 };
    // Avoid drawing the text outside the pill on small screens.
    if label_width + icon_w + gap > width - 12.0 {
        icon_w = 0.0;
        gap = 0.0;
    }
    let group_w = label_width + icon_w + gap;
    let left = rect.center().x - group_w * 0.5;
    if icon_w > 0.0 {
        let square = Rect::from_center_size(
            Pos2::new(left + icon_w * 0.5, rect.center().y), Vec2::splat(icon_w));
        if let Some(tex) = icon {
            painter.image(tex.id(), square,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
        } else if drawn_icon {
            let c = square.center();
            let line = Stroke::new(2.8_f32, fg);
            if label == "Menu" {
                for dy in [-6.0_f32, 0.0, 6.0] {
                    painter.line_segment(
                        [c + Vec2::new(-9.0, dy), c + Vec2::new(9.0, dy)], line);
                }
            } else {
                painter.line_segment(
                    [c + Vec2::new(-7.0, -7.0), c + Vec2::new(7.0, 7.0)], line);
                painter.line_segment(
                    [c + Vec2::new(7.0, -7.0), c + Vec2::new(-7.0, 7.0)], line);
            }
        }
    }
    let cx = left + icon_w + gap + label_width * 0.5;
    let cy = rect.center().y - 0.2;
    // Subtle subpixel overprint supplies a little more weight to the control
    // typography without bundling external font files.
    if font_size >= 15.0 {
        painter.text(Pos2::new(cx + 0.24, cy), egui::Align2::CENTER_CENTER,
            label, font.clone(), fg.gamma_multiply(0.87));
    }
    painter.text(Pos2::new(cx, cy), egui::Align2::CENTER_CENTER, label, font, fg);
    response
}
