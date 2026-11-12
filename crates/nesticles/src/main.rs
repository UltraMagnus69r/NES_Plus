mod audio_out;
mod export_game;
mod icon;
mod keys;
mod net;
mod studio;

include!(concat!(env!("OUT_DIR"), "/player_embed.rs"));

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1920.0, 1080.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_fullscreen(studio::STARTS_FULLSCREEN)
            .with_title("Nesticles")
            .with_icon(icon::icon_data()),
        // A restored window rectangle would undo the fullscreen startup.
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Nesticles",
        options,
        Box::new(|cc| Ok(Box::new(app::NestApp::new(cc)))),
    )
}

mod app;
