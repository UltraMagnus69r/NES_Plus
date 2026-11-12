//! Fixed-point scale shared by the SMB1 character preset and enemy steps.
//! One native 240p pixel is 4096 units. Whole pixels are `pos >> 12`.

pub const FP_SHIFT: i32 = 12;
pub const FP_ONE: i32 = 1 << FP_SHIFT;

/// 1080 / 240. A 16px tile is 72px on a 1920x1080 frame.
pub const VIRTUAL_PX_AT_1080: f32 = 1080.0 / 240.0;

pub fn px_to_fp(px: f32) -> i32 {
    (px as f64 * f64::from(FP_ONE)).round() as i32
}

pub fn fp_to_px(v: i32) -> f32 {
    (v as f64 / f64::from(FP_ONE)) as f32
}

/// Scale one virtual 240p pixel onto a 1080p frame.
pub fn scale_virtual_px(virtual_px: f32) -> f32 {
    virtual_px * VIRTUAL_PX_AT_1080
}

/// Play-view scale. Large surfaces use [`scale_virtual_px`] (4.5). Smaller panels fit the course
/// so a 4.5x view is not cropped into a corner of the level.
pub fn play_surface_scale(surface_w: f32, surface_h: f32, course_w: f32, course_h: f32) -> f32 {
    let ideal = scale_virtual_px(1.0);
    let min_w = scale_virtual_px(160.0);
    let min_h = scale_virtual_px(120.0);
    if surface_w >= min_w && surface_h >= min_h {
        return ideal;
    }
    if course_w > 1.0 && course_h > 1.0 {
        (surface_w / course_w).min(surface_h / course_h).max(0.05)
    } else {
        1.0
    }
}
