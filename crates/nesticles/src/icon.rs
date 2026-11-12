//! Program icon. The same PNG is the native window icon, the top-bar mark, and the exported favicon.

use eframe::egui::IconData;

pub const PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Straight (unmultiplied) RGBA. The source has no alpha, and [`IconData`] wants separate alpha.
pub fn icon_data() -> IconData {
    eframe::icon_data::from_png_bytes(PNG).expect("crates/nesticles/assets/icon.png is a PNG")
}

/// Width and height from the PNG IHDR, without decoding the pixels.
pub fn png_size() -> (u32, u32) {
    assert!(PNG.len() >= 24, "icon.png is truncated");
    assert_eq!(&PNG[..8], b"\x89PNG\r\n\x1a\n", "icon.png is not a PNG");
    let width = u32::from_be_bytes(PNG[16..20].try_into().expect("png width"));
    let height = u32::from_be_bytes(PNG[20..24].try_into().expect("png height"));
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_is_square_straight_rgba() {
        let (width, height) = png_size();
        assert!(width >= 32 && height >= 32);
        assert_eq!(width, height);
        let icon = icon_data();
        assert_eq!((icon.width, icon.height), (width, height));
        assert_eq!(icon.rgba.len(), width as usize * height as usize * 4);
        assert!(icon.rgba.chunks_exact(4).any(|px| px[3] == 255 && px[..3] != [0, 0, 0]));
    }
}
