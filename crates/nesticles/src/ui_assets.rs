//! Built-in UI art for Nesticles R4. Assets are decorative interface imagery,
//! not extracted game data or authored course entities.
//! If an override file exists at `$HOME/Desktop/Nesticle/Assets`, it wins.
//! Otherwise these compiled-in files keep portable builds self-contained.

use std::collections::HashMap;
use std::path::PathBuf;
use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

static EMBEDDED: &[(&str, &str, &[u8])] = &[
    ("backgrounds/maker-tools", "backgrounds/maker-tools.png", include_bytes!("../assets/ui/backgrounds/maker-tools.png")),
    ("backgrounds/stage-hero", "backgrounds/stage-hero.png", include_bytes!("../assets/ui/backgrounds/stage-hero.png")),
    ("backgrounds/discover-banner", "backgrounds/discover-banner.png", include_bytes!("../assets/ui/backgrounds/discover-banner.png")),
    ("backgrounds/maker-sidebar", "backgrounds/maker-sidebar.png", include_bytes!("../assets/ui/backgrounds/maker-sidebar.png")),
    ("backgrounds/maker-wallpaper", "backgrounds/maker-wallpaper.png", include_bytes!("../assets/ui/backgrounds/maker-wallpaper.png")),
    ("backgrounds/unmaker-sidebar", "backgrounds/unmaker-sidebar.png", include_bytes!("../assets/ui/backgrounds/unmaker-sidebar.png")),
    ("filters/default/all", "filters/default/all.png", include_bytes!("../assets/ui/filters/default/all.png")),
    ("filters/default/backgrounds", "filters/default/backgrounds.png", include_bytes!("../assets/ui/filters/default/backgrounds.png")),
    ("filters/default/blocks", "filters/default/blocks.png", include_bytes!("../assets/ui/filters/default/blocks.png")),
    ("filters/default/bosses", "filters/default/bosses.png", include_bytes!("../assets/ui/filters/default/bosses.png")),
    ("filters/default/enemies", "filters/default/enemies.png", include_bytes!("../assets/ui/filters/default/enemies.png")),
    ("filters/default/ignored", "filters/default/ignored.png", include_bytes!("../assets/ui/filters/default/ignored.png")),
    ("filters/default/items", "filters/default/items.png", include_bytes!("../assets/ui/filters/default/items.png")),
    ("filters/default/other", "filters/default/other.png", include_bytes!("../assets/ui/filters/default/other.png")),
    ("filters/default/players", "filters/default/players.png", include_bytes!("../assets/ui/filters/default/players.png")),
    ("filters/default/terrain", "filters/default/terrain.png", include_bytes!("../assets/ui/filters/default/terrain.png")),
    ("filters/default/unidentified", "filters/default/unidentified.png", include_bytes!("../assets/ui/filters/default/unidentified.png")),
    ("filters/selected/all", "filters/selected/all.png", include_bytes!("../assets/ui/filters/selected/all.png")),
    ("filters/selected/backgrounds", "filters/selected/backgrounds.png", include_bytes!("../assets/ui/filters/selected/backgrounds.png")),
    ("filters/selected/blocks", "filters/selected/blocks.png", include_bytes!("../assets/ui/filters/selected/blocks.png")),
    ("filters/selected/bosses", "filters/selected/bosses.png", include_bytes!("../assets/ui/filters/selected/bosses.png")),
    ("filters/selected/enemies", "filters/selected/enemies.png", include_bytes!("../assets/ui/filters/selected/enemies.png")),
    ("filters/selected/ignored", "filters/selected/ignored.png", include_bytes!("../assets/ui/filters/selected/ignored.png")),
    ("filters/selected/items", "filters/selected/items.png", include_bytes!("../assets/ui/filters/selected/items.png")),
    ("filters/selected/other", "filters/selected/other.png", include_bytes!("../assets/ui/filters/selected/other.png")),
    ("filters/selected/players", "filters/selected/players.png", include_bytes!("../assets/ui/filters/selected/players.png")),
    ("filters/selected/terrain", "filters/selected/terrain.png", include_bytes!("../assets/ui/filters/selected/terrain.png")),
    ("filters/selected/unidentified", "filters/selected/unidentified.png", include_bytes!("../assets/ui/filters/selected/unidentified.png")),
    ("icons/discover-search", "icons/discover-search.png", include_bytes!("../assets/ui/icons/discover-search.png")),
    ("icons/egg-logo", "icons/egg-logo.png", include_bytes!("../assets/ui/icons/egg-logo.png")),
    ("icons/navigation/default/create", "icons/navigation/default/create.png", include_bytes!("../assets/ui/icons/navigation/default/create.png")),
    ("icons/navigation/default/discover", "icons/navigation/default/discover.png", include_bytes!("../assets/ui/icons/navigation/default/discover.png")),
    ("icons/navigation/default/home", "icons/navigation/default/home.png", include_bytes!("../assets/ui/icons/navigation/default/home.png")),
    ("icons/navigation/default/import", "icons/navigation/default/import.png", include_bytes!("../assets/ui/icons/navigation/default/import.png")),
    ("icons/navigation/default/investigate", "icons/navigation/default/investigate.png", include_bytes!("../assets/ui/icons/navigation/default/investigate.png")),
    ("icons/navigation/default/review", "icons/navigation/default/review.png", include_bytes!("../assets/ui/icons/navigation/default/review.png")),
    ("icons/navigation/default/save", "icons/navigation/default/save.png", include_bytes!("../assets/ui/icons/navigation/default/save.png")),
    ("icons/navigation/selected/create", "icons/navigation/selected/create.png", include_bytes!("../assets/ui/icons/navigation/selected/create.png")),
    ("icons/navigation/selected/discover", "icons/navigation/selected/discover.png", include_bytes!("../assets/ui/icons/navigation/selected/discover.png")),
    ("icons/navigation/selected/home", "icons/navigation/selected/home.png", include_bytes!("../assets/ui/icons/navigation/selected/home.png")),
    ("icons/navigation/selected/import", "icons/navigation/selected/import.png", include_bytes!("../assets/ui/icons/navigation/selected/import.png")),
    ("icons/navigation/selected/investigate", "icons/navigation/selected/investigate.png", include_bytes!("../assets/ui/icons/navigation/selected/investigate.png")),
    ("icons/navigation/selected/review", "icons/navigation/selected/review.png", include_bytes!("../assets/ui/icons/navigation/selected/review.png")),
    ("icons/navigation/selected/save", "icons/navigation/selected/save.png", include_bytes!("../assets/ui/icons/navigation/selected/save.png")),
    ("icons/nesticles-logo", "icons/nesticles-logo.png", include_bytes!("../assets/ui/icons/nesticles-logo.png")),
    ("icons/tools/default/erase", "icons/tools/default/erase.png", include_bytes!("../assets/ui/icons/tools/default/erase.png")),
    ("icons/tools/default/fill", "icons/tools/default/fill.png", include_bytes!("../assets/ui/icons/tools/default/fill.png")),
    ("icons/tools/default/inspect", "icons/tools/default/inspect.png", include_bytes!("../assets/ui/icons/tools/default/inspect.png")),
    ("icons/tools/default/line", "icons/tools/default/line.png", include_bytes!("../assets/ui/icons/tools/default/line.png")),
    ("icons/tools/default/place", "icons/tools/default/place.png", include_bytes!("../assets/ui/icons/tools/default/place.png")),
    ("icons/tools/default/select", "icons/tools/default/select.png", include_bytes!("../assets/ui/icons/tools/default/select.png")),
    ("icons/tools/default/undo", "icons/tools/default/undo.png", include_bytes!("../assets/ui/icons/tools/default/undo.png")),
    ("icons/tools/default/vertical", "icons/tools/default/vertical.png", include_bytes!("../assets/ui/icons/tools/default/vertical.png")),
    ("icons/tools/selected/erase", "icons/tools/selected/erase.png", include_bytes!("../assets/ui/icons/tools/selected/erase.png")),
    ("icons/tools/selected/fill", "icons/tools/selected/fill.png", include_bytes!("../assets/ui/icons/tools/selected/fill.png")),
    ("icons/tools/selected/inspect", "icons/tools/selected/inspect.png", include_bytes!("../assets/ui/icons/tools/selected/inspect.png")),
    ("icons/tools/selected/line", "icons/tools/selected/line.png", include_bytes!("../assets/ui/icons/tools/selected/line.png")),
    ("icons/tools/selected/place", "icons/tools/selected/place.png", include_bytes!("../assets/ui/icons/tools/selected/place.png")),
    ("icons/tools/selected/select", "icons/tools/selected/select.png", include_bytes!("../assets/ui/icons/tools/selected/select.png")),
    ("icons/tools/selected/undo", "icons/tools/selected/undo.png", include_bytes!("../assets/ui/icons/tools/selected/undo.png")),
    ("icons/tools/selected/vertical", "icons/tools/selected/vertical.png", include_bytes!("../assets/ui/icons/tools/selected/vertical.png")),
];

pub struct UiAssets {
    textures: HashMap<&'static str, TextureHandle>,
}

impl UiAssets {
    pub fn new(ctx: &egui::Context) -> Self {
        let assets_dir = std::env::var_os("NESTICLES_ASSETS_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Desktop/Nesticle/Assets")));
        let mut textures = HashMap::new();
        for &(key, rel, embedded) in EMBEDDED {
            let personal = assets_dir.as_ref().and_then(|root| {
                let custom_filename = match key {
                    "backgrounds/maker-tools" => Some("Glossy Super Platformer Landscape(4).png"),
                    "backgrounds/stage-hero" => Some("ChatGPT Image Nov 18, 2026, 08_53_11 PM(3).png"),
                    _ => None,
                };
                custom_filename.and_then(|name| std::fs::read(root.join("backgrounds").join(name)).ok())
                    .or_else(|| std::fs::read(root.join(rel)).ok())
            });
            let bytes = personal.unwrap_or_else(|| embedded.to_vec());
            let decoded = image::load_from_memory(&bytes)
                .or_else(|_| image::load_from_memory(embedded));
            let Ok(decoded) = decoded else { continue };
            let rgba = decoded.to_rgba8();
            let (w, h) = rgba.dimensions();
            let egui_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize], rgba.as_raw());
            let tex = ctx.load_texture(format!("nesticles-ui/{key}"), egui_image,
                if key == "icons/nesticles-logo" { TextureOptions::LINEAR } else { TextureOptions::LINEAR });
            textures.insert(key, tex);
        }
        Self { textures }
    }

    pub fn get(&self, key: &str) -> Option<&TextureHandle> {
        self.textures.get(key)
    }

    pub fn nav(&self, key: &str, selected: bool) -> Option<&TextureHandle> {
        let k = format!("icons/navigation/{}/{}", if selected { "selected" } else { "default" }, key);
        self.get(&k)
    }

    pub fn tool(&self, key: &str, selected: bool) -> Option<&TextureHandle> {
        let k = format!("icons/tools/{}/{}", if selected { "selected" } else { "default" }, key);
        self.get(&k)
    }

    pub fn filter(&self, key: &str, selected: bool) -> Option<&TextureHandle> {
        let k = format!("filters/{}/{}", if selected { "selected" } else { "default" }, key);
        self.get(&k)
    }

    pub fn paint_cover(&self, painter: &egui::Painter, key: &str, rect: Rect, focus_x: f32, focus_y: f32) -> bool {
        let Some(tex) = self.get(key) else { return false };
        paint_cover_texture(painter, tex, rect, focus_x, focus_y);
        true
    }

    pub fn paint_cover_tinted(&self, painter: &egui::Painter, key: &str,
        rect: Rect, focus_x: f32, focus_y: f32, tint: Color32) -> bool {
        let Some(tex) = self.get(key) else { return false };
        paint_cover_texture_tinted(painter, tex, rect, focus_x, focus_y, tint);
        true
    }

    pub fn paint(&self, painter: &egui::Painter, key: &str, rect: Rect) -> bool {
        let Some(tex) = self.get(key) else { return false };
        painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        true
    }
}

pub fn paint_cover_texture(painter: &egui::Painter, texture: &TextureHandle,
    rect: Rect, focus_x: f32, focus_y: f32) {
    paint_cover_texture_tinted(painter, texture, rect, focus_x, focus_y, Color32::WHITE);
}

pub fn paint_cover_texture_tinted(painter: &egui::Painter, texture: &TextureHandle,
    rect: Rect, focus_x: f32, focus_y: f32, tint: Color32) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 { return; }
    let size = texture.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 { return; }
    let image_ratio = size.x / size.y;
    let target_ratio = rect.width() / rect.height();
    let (uw, vh) = if image_ratio > target_ratio {
        ((target_ratio / image_ratio).clamp(0.0, 1.0), 1.0)
    } else {
        (1.0, (image_ratio / target_ratio).clamp(0.0, 1.0))
    };
    let u = (focus_x.clamp(0.0, 1.0) * (1.0 - uw)).clamp(0.0, 1.0 - uw);
    let v = (focus_y.clamp(0.0, 1.0) * (1.0 - vh)).clamp(0.0, 1.0 - vh);
    let uv = Rect::from_min_max(Pos2::new(u, v), Pos2::new(u + uw, v + vh));
    painter.image(texture.id(), rect, uv, tint);
}

pub fn image_in_rect(painter: &egui::Painter, texture: Option<&TextureHandle>, rect: Rect) {
    if let Some(tex) = texture {
        painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0,1.0)), Color32::WHITE);
    }
}
