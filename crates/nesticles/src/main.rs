mod audio_out;
mod export_game;
mod icon;
mod keys;
mod net;

include!(concat!(env!("OUT_DIR"), "/player_embed.rs"));

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([980.0, 640.0])
            .with_title("Nesticles")
            .with_icon(icon::icon_data()),
        ..Default::default()
    };
    eframe::run_native(
        "Nesticles",
        options,
        Box::new(|cc| Ok(Box::new(app::NestApp::new(cc)))),
    )
}

mod app;
