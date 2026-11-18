mod audio_out;
mod openai;
mod export_game;
mod icon;
mod keys;
mod net;
mod studio;

include!(concat!(env!("OUT_DIR"), "/player_embed.rs"));

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(studio::INITIAL_INNER)
            .with_min_inner_size(studio::MIN_INNER)
            .with_decorations(true)
            .with_resizable(true)
            .with_maximized(false)
            .with_fullscreen(studio::STARTS_FULLSCREEN)
            .with_title("Nesticles")
            .with_app_id(studio::WINDOW_APP_ID)
            .with_icon(icon::icon_data()),
        // Geometry is `app.ron` under the OS data dir for `nesticles-window`.
        // That file is not the alpha library and is not deleted with a project.
        persist_window: true,
        // Centering on X11 runs after restore and would drop the saved position.
        // The first window is placed by the desktop. An off-screen restore is
        // moved back in `place_window_if_needed`.
        centered: false,
        ..Default::default()
    };
    eframe::run_native(
        "Nesticles",
        options,
        Box::new(|cc| Ok(Box::new(app::NestApp::new(cc)))),
    )
}

mod app;
