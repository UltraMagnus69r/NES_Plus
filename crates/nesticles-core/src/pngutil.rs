//! PNG helpers and the sprite quantizer used by the image maker.

use base64::Engine;

pub fn b64_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

pub fn b64_decode(text: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|e| e.to_string())
}

#[cfg(feature = "png")]
pub fn rgba_to_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    if rgba.len() < (width as usize) * (height as usize) * 4 {
        return Err("RGBA buffer is shorter than the image".into());
    }
    let mut cursor = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
    use image::ImageEncoder;
    encoder
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

#[cfg(feature = "png")]
pub fn png_to_rgba(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let image = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let rgba = image.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}

/// Workshop palette used when a generated picture is snapped to a sprite.
pub fn sprite_palette() -> Vec<[u8; 4]> {
    vec![
        [0, 0, 0, 0],
        [28, 22, 20, 255],
        [243, 234, 215, 255],
        [224, 114, 58, 255],
        [110, 168, 74, 255],
        [36, 72, 48, 255],
        [150, 96, 54, 255],
        [96, 58, 36, 255],
        [230, 186, 84, 255],
        [176, 64, 78, 255],
        [126, 176, 196, 255],
        [64, 84, 120, 255],
        [240, 220, 190, 255],
        [80, 64, 56, 255],
        [200, 140, 90, 255],
        [20, 16, 14, 255],
    ]
}

pub fn downscale_box(src: &[u8], sw: u32, sh: u32, side: u32) -> Vec<u8> {
    let side = side.clamp(1, 256);
    let mut out = vec![0u8; (side * side * 4) as usize];
    if sw == 0 || sh == 0 || src.len() < (sw * sh * 4) as usize {
        return out;
    }
    for y in 0..side {
        for x in 0..side {
            let x0 = x * sw / side;
            let x1 = ((x + 1) * sw / side).max(x0 + 1).min(sw);
            let y0 = y * sh / side;
            let y1 = ((y + 1) * sh / side).max(y0 + 1).min(sh);
            let mut acc = [0u32; 4];
            let mut n = 0u32;
            for py in y0..y1 {
                for px in x0..x1 {
                    let o = ((py * sw + px) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += src[o + c] as u32;
                    }
                    n += 1;
                }
            }
            let o = ((y * side + x) * 4) as usize;
            if n == 0 {
                continue;
            }
            for c in 0..4 {
                out[o + c] = (acc[c] / n) as u8;
            }
        }
    }
    out
}

pub fn quantize_rgba(rgba: &mut [u8]) {
    let palette = sprite_palette();
    for px in rgba.chunks_exact_mut(4) {
        if px[3] < 48 {
            px.copy_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let mut best_i = 1usize;
        let mut best_d = i32::MAX;
        for (i, color) in palette.iter().enumerate().skip(1) {
            let d = (px[0] as i32 - color[0] as i32).pow(2)
                + (px[1] as i32 - color[1] as i32).pow(2)
                + (px[2] as i32 - color[2] as i32).pow(2);
            if d < best_d {
                best_d = d;
                best_i = i;
            }
        }
        px.copy_from_slice(&palette[best_i]);
    }
}

pub fn sprite_from_image(bytes: &[u8], side: u32) -> Result<(u32, u32, Vec<u8>, Vec<u8>), String> {
    #[cfg(feature = "png")]
    {
        let (w, h, rgba) = png_to_rgba(bytes)?;
        let side = match side {
            16 | 32 | 48 => side,
            _ => 32,
        };
        let mut small = downscale_box(&rgba, w, h, side);
        quantize_rgba(&mut small);
        let png = rgba_to_png(&small, side, side)?;
        return Ok((side, side, small, png));
    }
    #[cfg(not(feature = "png"))]
    {
        let _ = (bytes, side);
        Err("PNG support is not compiled into this build".into())
    }
}
