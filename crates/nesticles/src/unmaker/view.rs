//! UnMaker Lab screens. Maker Engine does not use this module.
//!
//! Source tiles drawn here are evidence from CHR. They are not a reconstructed
//! character, and the preview does not play them as an animation.

use super::model::{
    approve_feedback, approve_label, character_sheet, continue_available, discover_goomba,
    gallery_card, gallery_items, ignore_feedback, issue_queue, label_for_state, preview_policy,
    reconstructed_frame_count, restore_feedback, review_tile_rgba, save_status_line, skip_feedback,
    Group, IssueCard, ReviewPalette, Stage, StudioState, StudioTool, NOT_RECONSTRUCTED,
};
use crate::app::NestApp;
use crate::design;
use crate::responsive::ResponsiveLayout;
use crate::studio::{self, Workspace};
use eframe::egui::{
    self, Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions,
    Vec2,
};
use nesticles_core::codec::{decode_tile, Codec};
use nesticles_core::pipeline::{
    approve_candidate, fail_candidate, ignore_candidate, inspect_nes, job_cancel, puzzle_from,
    reinspect, restore_candidate, FailReason, SMB1_PAYLOAD_SHA256,
};
use std::collections::HashMap;

pub struct StudioUi {
    pub state: StudioState,
    pub tiles: HashMap<u32, TextureHandle>,
    painted: ReviewPalette,
    tool_env_applied: bool,
    r2_env_applied: bool,
    preview_accum: f32,
    assembled: HashMap<String, TextureHandle>,
}

impl Default for StudioUi {
    fn default() -> Self {
        Self {
            state: StudioState::default(),
            tiles: HashMap::new(),
            painted: ReviewPalette::default(),
            tool_env_applied: false,
            r2_env_applied: false,
            preview_accum: 0.0,
            assembled: HashMap::new(),
        }
    }
}

impl StudioUi {
    pub fn note_stage(&mut self, name: &str) {
        if self.state.stage_log.last().map(String::as_str) != Some(name) {
            self.state.stage_log.push(name.to_string());
        }
    }
}

impl NestApp {
    pub(crate) fn unmaker_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("shell")
            .exact_height(studio::BAR_H)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(224, 236, 255))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .stroke(Stroke::new(1.0_f32, studio::unmaker_line())),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.brand_mark(ui);
                    ui.add_space(10.0);
                    self.workspace_chip(ui, Workspace::Unmaker);
                    ui.add_space(5.0);
                    self.workspace_chip(ui, Workspace::Maker);
                    ui.add_space(6.0);
                    // A real Open Project action, rather than a fake dropdown.
                    // Reserve enough space for the action buttons on 1080p.
                    if ui.available_width() > 570.0 {
                        let caption = self.project_caption();
                        let short: String = caption.chars().take(25).collect();
                        if design::capsule_button(
                            ui, &format!("Project: {short} ▾"),
                            self.ui_assets.nav("import", false),
                            300.0, 46.0, design::CapsuleTone::White,
                        ).on_hover_text("Open a saved project").clicked() {
                            self.open_project_dialog();
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if design::capsule_button(ui, "Exit", None, 94.0, 46.0,
                            design::CapsuleTone::Red).clicked() {
                            self.request_exit(ctx);
                        }
                        if design::capsule_button(ui, "Menu", None, 94.0, 46.0,
                            design::CapsuleTone::White).clicked() {
                            self.app_menu = !self.app_menu;
                        }
                        let save = if self.dirty { "Save ●" } else { "Save" };
                        if design::capsule_button(ui, save, self.ui_assets.nav("save", false),
                            100.0, 46.0, design::CapsuleTone::White).clicked() {
                            self.save_project_quick();
                            if self.workspace == Workspace::Unmaker {
                                self.studio.state.notice = if self.dirty {
                                    "The project save did not finish. Nothing was published.".into()
                                } else {
                                    "Project file saved. The extraction session is still only in memory.".into()
                                };
                            }
                        }
                        if self.workspace == Workspace::Maker && self.tab != crate::app::Tab::Play {
                            if design::capsule_button(ui, "▶ Test Play", None, 140.0, 46.0,
                                design::CapsuleTone::Green).clicked() {
                                self.toggle_play();
                            }
                        }
                    });
                });
            });
    }

    fn brand_mark(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(254.0, 62.0), Sense::hover());
        if !self.ui_assets.paint(ui.painter(), "icons/nesticles-logo", rect) {
            design::paint_egg(ui.painter(), Rect::from_min_size(rect.min, Vec2::splat(46.0)));
            ui.painter().text(rect.center() + Vec2::new(30.0, 0.0), egui::Align2::CENTER_CENTER,
                "Nesticles", egui::FontId::proportional(29.0), studio::unmaker_ink());
        }
        ui.painter().line_segment(
            [rect.right_top() + Vec2::new(5.0, 3.0), rect.right_bottom() + Vec2::new(5.0, -3.0)],
            Stroke::new(1.0_f32, Color32::from_rgb(143, 170, 215)),
        );
    }

    fn workspace_chip(&mut self, ui: &mut egui::Ui, which: Workspace) {
        let on = self.workspace == which;
        let (title, subtitle) = match which {
            Workspace::Unmaker => ("UnMaker Lab", "Investigate & Reconstruct"),
            Workspace::Maker => ("Maker Engine", "Build & Play Creations"),
        };
        let width = if on { 214.0 } else { 199.0 };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 56.0), Sense::click());
        let fill = if on { Color32::from_rgb(22, 130, 253) }
            else if response.hovered() { Color32::from_rgb(234, 245, 255) }
            else { Color32::from_rgb(252, 254, 255) };
        design::glossy_surface(ui.painter(), rect, fill, on);
        let icon = Rect::from_center_size(
            rect.left_center() + Vec2::new(28.0, 0.0), Vec2::splat(38.0)
        );
        if which == Workspace::Unmaker {
            if !self.ui_assets.paint(ui.painter(), "icons/egg-logo", icon) {
                design::paint_egg(ui.painter(), icon);
            }
        } else {
            design::paint_castle(ui.painter(), icon);
        }
        let text = if on { Color32::WHITE } else { studio::unmaker_ink() };
        ui.painter().text(rect.left_top() + Vec2::new(55.0, 20.0),
            egui::Align2::LEFT_CENTER, title,
            egui::FontId::proportional(17.5), text);
        ui.painter().text(rect.left_top() + Vec2::new(55.0, 39.0),
            egui::Align2::LEFT_CENTER, subtitle,
            egui::FontId::proportional(11.5),
            if on { Color32::from_rgb(243, 249, 255) } else { studio::caption() });
        if response.clicked() && !on { self.flip_workspace(); }
    }

    fn project_caption(&self) -> String {
        let name = if self.project.name.is_empty() {
            "Untitled"
        } else {
            self.project.name.as_str()
        };
        if self.workspace == Workspace::Unmaker {
            if self.rom.is_some() {
                let file = std::path::Path::new(&self.rom_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("cartridge");
                format!("{name} · {file}")
            } else {
                format!("{name} · no cartridge")
            }
        } else {
            let course = self
                .project
                .levels
                .get(self.level_sel)
                .map(|level| level.name.as_str())
                .unwrap_or("no course");
            format!("{name} · {course}")
        }
    }

    fn play_button(&self, label: &str) -> egui::Button<'_> {
        egui::Button::new(RichText::new(format!("▶ {label}")).size(16.0).strong().color(Color32::WHITE))
            .fill(studio::game_green())
            .stroke(Stroke::new(1.0, Color32::from_rgb(32, 158, 68)))
            .corner_radius(CornerRadius::same(18))
            .min_size(Vec2::new(148.0, 46.0))
    }

    fn bar_button(&self, label: &str, danger: bool) -> egui::Button<'_> {
        let color = if danger { Color32::WHITE } else { studio::unmaker_ink() };
        let fill = if danger { studio::unmaker_red() } else { Color32::from_rgb(255, 255, 255) };
        let icon = if label.starts_with("Save") {
            "💾 "
        } else if label == "Menu" {
            "☰ "
        } else if label == "Exit" {
            "✕ "
        } else {
            ""
        };
        egui::Button::new(RichText::new(format!("{icon}{label}")).size(16.0).strong().color(color))
            .fill(fill)
            .stroke(if danger { Stroke::new(1.0, Color32::from_rgb(192, 35, 44)) } else { Stroke::new(1.0_f32, studio::unmaker_line()) })
            .corner_radius(CornerRadius::same(17))
            .min_size(Vec2::new(if danger { 110.0 } else { 104.0 }, 46.0))
    }

    pub(crate) fn unmaker_studio(&mut self, ctx: &egui::Context) {
        self.apply_studio_tool_env();
        let layout = ResponsiveLayout::from_context(ctx);
        egui::SidePanel::left("unmaker-nav")
            .exact_width(layout.unmaker_nav_width())
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(studio::unmaker_paper())
                    .inner_margin(if layout.show_unmaker_nav_labels() { 8.0 } else { 4.0 })
                    .stroke(Stroke::new(1.0_f32, studio::unmaker_line())),
            )
            .show(ctx, |ui| {
                let r = ui.max_rect();
                if !self.ui_assets.paint_cover(ui.painter(),
                    "backgrounds/unmaker-sidebar", r, 0.5, 0.75) {
                    design::paint_scenery(ui.painter(), r, true);
                }
                if self.stage_button(ui, Stage::Home, layout).clicked() {
                    self.studio.state.stage = Stage::Home;
                    self.studio.state.show_manual = false;
                    self.studio.state.tool = None;
                }
                ui.add_space(8.0);
                for stage in Stage::journey() {
                    if self.stage_button(ui, stage, layout).clicked() {
                        self.studio.state.stage = stage;
                        self.studio.state.show_manual = false;
                        self.studio.state.tool = None;
                    }
                }
                ui.add_space(12.0);
                let details = self.studio.state.show_technical;
                let tech_label = if layout.show_unmaker_nav_labels() {
                    "Technical details"
                } else {
                    "Tech"
                };
                if ui
                    .selectable_label(details, tech_label)
                    .on_hover_text("Show hashes, indexes, and other diagnostics")
                    .clicked()
                {
                    self.studio.state.show_technical = !details;
                }
                if layout.show_unmaker_nav_labels() {
                    ui.add_space(8.0);
                    egui::CollapsingHeader::new("About this build")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.label("Goomba approval writes that sprite into the project. Other approvals do not.");
                            ui.label("Mario and Luigi have no tile mapping.");
                            ui.label("Review colors are not a verified palette.");
                        });
                }
            });
        let show_tech = self.studio.state.show_technical && layout.show_technical_panel();
        if show_tech {
            egui::SidePanel::right("unmaker-tech")
                .exact_width(layout.technical_panel_width())
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(studio::unmaker_mist())
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| self.technical_panel(ui));
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(studio::unmaker_mist())
                    .inner_margin(layout.page_margin()),
            )
            .show(ctx, |ui| {
                // Routine cartridge/payload summaries already appear inside
                // the active screen. Keep actionable errors visible.
                let notice = self.studio.state.notice.as_str();
                let routine_notice = notice.starts_with("Payload ")
                    || notice.starts_with("Investigation started.")
                    || notice.starts_with("Cartridge opened.")
                    || notice.starts_with("Mapper ")
                    || notice.starts_with("Current stage:");
                if !notice.is_empty() && !routine_notice {
                    egui::Frame::NONE
                        .fill(Color32::from_rgb(255, 244, 214))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(10.0)
                        .stroke(Stroke::new(1.0, studio::accent_yellow()))
                        .show(ui, |ui| {
                            ui.label(RichText::new(&self.studio.state.notice).size(15.0));
                        });
                    ui.add_space(8.0);
                }
                let gallery = self.studio.state.tool.is_none()
                    && self.studio.state.stage == Stage::Discover;
                if gallery {
                    self.stage_hero(ui, Stage::Discover, layout);
                    self.discover_stage(ui, ctx, layout);
                    return;
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if let Some(tool) = self.studio.state.tool {
                            self.embedded_tool(ui, ctx, tool);
                        } else {
                            self.stage_hero(ui, self.studio.state.stage, layout);
                            match self.studio.state.stage {
                                Stage::Home => self.home_stage(ui),
                                Stage::Import => self.import_stage(ui),
                                Stage::Investigate => self.investigate_stage(ui, layout),
                                Stage::Discover => self.discover_stage(ui, ctx, layout),
                                Stage::Review => self.review_stage(ui, ctx),
                                Stage::Create => self.create_stage(ui, ctx),
                                Stage::Save => self.save_stage(ui),
                            }
                        }
                    });
            });
    }

    /// A full-width section identity banner shared by every UnMaker page.
    /// The scenery is cropped (never stretched), with a soft bottom blend into
    /// the page and a pale left-side veil to preserve title legibility.
    fn stage_hero(&self, ui: &mut egui::Ui, stage: Stage, layout: ResponsiveLayout) {
        let (title, subtitle) = match stage {
            Stage::Home => ("Home", "Choose where your creation begins."),
            Stage::Import => ("Import", "Open a cartridge or continue a project."),
            Stage::Investigate => ("Investigation", "Inspect the evidence, locally."),
            Stage::Discover => ("Discover", "Browse what UnMaker found in your cartridge."),
            Stage::Review => ("Review", "Verify sprites and unresolved discoveries."),
            Stage::Create => ("Create", "Build original assets from the ground up."),
            Stage::Save => ("Save", "Preserve your work as a native project."),
        };
        let hero_h = layout.stage_hero_height();
        let size = Vec2::new(ui.available_width().max(100.0), hero_h);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(22), Color32::from_rgb(218, 238, 255));
        let image_rect = rect.shrink(2.0);
        if !self.ui_assets.paint_cover(&painter, "backgrounds/stage-hero", image_rect, 0.5, 0.56) {
            design::paint_scenery(&painter, image_rect, true);
        }
        // Fade the illustrated scene into the page so the lower border is not
        // a sharp rectangle. The left light panel keeps all headings readable.
        for n in 0..28 {
            let t = n as f32 / 28.0;
            let x = rect.left() + rect.width() * 0.58 * t;
            let strip = Rect::from_min_size(Pos2::new(x, rect.top()),
                Vec2::new((rect.width() * 0.58 / 28.0) + 1.0, rect.height()));
            let alpha = (228.0 * (1.0 - t * t)).round() as u8;
            painter.rect_filled(strip, 0.0, Color32::from_white_alpha(alpha));
        }
        for n in 0..16 {
            let t = n as f32 / 16.0;
            let y = rect.bottom() - 22.0 + t * 22.0;
            painter.rect_filled(Rect::from_min_size(Pos2::new(rect.left(), y),
                Vec2::new(rect.width(), 22.0 / 16.0 + 1.0)), 0.0,
                studio::unmaker_mist().gamma_multiply(t * 0.60));
        }
        let key = match stage {
            Stage::Home => "home", Stage::Import => "import",
            Stage::Investigate => "investigate", Stage::Discover => "discover",
            Stage::Review => "review", Stage::Create => "create", Stage::Save => "save",
        };
        let mark_size = if matches!(layout.mode, crate::responsive::LayoutMode::Compact) {
            36.0
        } else {
            48.0
        };
        let title_size = if matches!(layout.mode, crate::responsive::LayoutMode::Compact) {
            22.0
        } else {
            30.0
        };
        let mark_y = ((hero_h - mark_size) * 0.5).max(8.0);
        let mark = Rect::from_min_size(rect.min + Vec2::new(16.0, mark_y), Vec2::splat(mark_size));
        let iconic = if stage == Stage::Discover {
            self.ui_assets.get("icons/discover-search")
        } else {
            self.ui_assets.nav(key, false)
        };
        crate::app::ui_assets::image_in_rect(&painter, iconic, mark);
        let text_x = 16.0 + mark_size + 14.0;
        painter.text(rect.min + Vec2::new(text_x, mark_y), egui::Align2::LEFT_TOP,
            title, egui::FontId::proportional(title_size), studio::unmaker_ink());
        if hero_h >= 90.0 {
            painter.text(rect.min + Vec2::new(text_x + 1.0, mark_y + title_size + 6.0),
                egui::Align2::LEFT_TOP,
                subtitle, egui::FontId::proportional(15.0), studio::unmaker_ink());
        }
        ui.add_space(8.0);
    }

    fn stage_button(
        &self,
        ui: &mut egui::Ui,
        stage: Stage,
        layout: ResponsiveLayout,
    ) -> egui::Response {
        let on = self.studio.state.stage == stage;
        let key = match stage {
            Stage::Home => "home",
            Stage::Import => "import",
            Stage::Investigate => "investigate",
            Stage::Discover => "discover",
            Stage::Review => "review",
            Stage::Create => "create",
            Stage::Save => "save",
        };
        if layout.show_unmaker_nav_labels() {
            let available = ui.available_width().max(140.0);
            design::capsule_button(ui, stage.label(), self.ui_assets.nav(key, on),
                available, 54.0,
                if on { design::CapsuleTone::Blue } else { design::CapsuleTone::White },
            ).on_hover_text(stage_tip(stage))
        } else {
            let available = ui.available_width().max(56.0);
            design::capsule_button(ui, "", self.ui_assets.nav(key, on),
                available, 48.0,
                if on { design::CapsuleTone::Blue } else { design::CapsuleTone::White },
            ).on_hover_text(format!("{} — {}", stage.label(), stage_tip(stage)))
        }
    }

    fn home_stage(&mut self, ui: &mut egui::Ui) {
        if self
            .home_card(
                ui,
                "Disassemble a ROM",
                "Open a cartridge and start an investigation.",
            )
            .clicked()
        {
            self.studio.state.stage = Stage::Import;
        }
        if self
            .home_card(
                ui,
                "Open a Saved Creation",
                "Open a project you saved. Older alpha libraries are not imported.",
            )
            .clicked()
        {
            self.open_project_dialog();
        }
        if self
            .home_card(
                ui,
                "Start a New Creation",
                "Make an original sprite without a ROM.",
            )
            .clicked()
        {
            self.studio.state.stage = Stage::Create;
            self.studio.state.create_animate = false;
        }
        let show = continue_available(
            self.rom.is_some(),
            self.extract.session.is_some(),
            self.extract.job.is_some(),
            !self.project.sprites.is_empty(),
        );
        if show
            && self
                .home_card(
                    ui,
                    "Continue Current Work",
                    "Return to the cartridge or sprite already open in this session.",
                )
                .clicked()
        {
            self.studio.state.stage = if self.extract.job.is_some() {
                Stage::Investigate
            } else if self.extract.session.is_some() {
                Stage::Discover
            } else if !self.project.sprites.is_empty() {
                Stage::Create
            } else {
                Stage::Import
            };
        }
    }

    fn home_card(&self, ui: &mut egui::Ui, title: &str, body: &str) -> egui::Response {
        let width = ui.available_width().min(520.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 72.0), Sense::click());
        let fill = if response.hovered() {
            Color32::from_rgb(232, 242, 255)
        } else {
            studio::unmaker_paper()
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(16),
            fill,
            Stroke::new(1.0_f32, studio::unmaker_line()),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.min + Vec2::new(16.0, 24.0),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(20.0),
            studio::unmaker_ink(),
        );
        ui.painter().text(
            rect.min + Vec2::new(16.0, 48.0),
            egui::Align2::LEFT_CENTER,
            body,
            egui::FontId::proportional(14.0),
            Color32::from_rgb(70, 70, 70),
        );
        ui.add_space(8.0);
        response
    }

    fn import_stage(&mut self, ui: &mut egui::Ui) {
        if design::capsule_button(ui, "Open ROM",
            self.ui_assets.nav("import", true), 206.0, 49.0,
            design::CapsuleTone::Red)
            .on_hover_text("Choose a cartridge file. The name is not its identity.")
            .clicked()
        {
            self.choose_studio_rom();
        }
        ui.add_space(8.0);
        self.identity_card(ui);
        ui.add_space(12.0);
        ui.label("Optional reference URL");
        ui.add(
            egui::TextEdit::singleline(&mut self.extract.ref_url)
                .hint_text("https://")
                .desired_width(420.0),
        );
        ui.horizontal(|ui| {
            if design::capsule_button(ui, "Fetch reference", None,
                165.0, 43.0, design::CapsuleTone::White)
                .on_hover_text("Download that HTTPS page. This does not send the ROM anywhere.")
                .clicked()
            {
                self.start_reference_fetch();
                self.studio.state.notice =
                    "Reference fetch started. It is not part of ROM identity.".into();
            }
            if design::capsule_button(ui, "Start Investigation", None,
                214.0, 43.0, design::CapsuleTone::Green)
                .on_hover_text("Run the local profile. This does not fetch the reference URL.")
                .clicked()
            {
                self.start_investigation();
            }
        });
        if let Some(err) = self.extract.ref_error.clone() {
            ui.colored_label(studio::unmaker_red(), &err);
            self.reference_recovery_row(ui);
        } else if let Some(line) = self.extract.ref_log.last() {
            ui.label(line);
        }
        ui.label(format!(
            "{} reference image(s) listed.",
            self.extract.ref_items.len()
        ));
        if !self.extract.ref_items.is_empty() {
            ui.separator();
            ui.label("Choose a listed image. It is not matched to the cartridge.");
            self.reference_panel(ui);
        }
    }

    fn investigate_stage(&mut self, ui: &mut egui::Ui, layout: ResponsiveLayout) {
        let panel_width = ui.available_width().min(if matches!(
            layout.mode,
            crate::responsive::LayoutMode::Compact | crate::responsive::LayoutMode::Medium
        ) {
            ui.available_width()
        } else {
            940.0
        });
        ui.vertical(|ui| {
                ui.set_max_width(panel_width);
                design::floating_frame(studio::unmaker_paper()).show(ui, |ui| {
                    let running = self.extract.job.as_ref()
                        .map(|job| (job.stage_name.clone(), job.error.clone()));
                    let mut cancel = false;
                    if let Some((name, error)) = &running {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Current stage").size(17.0).strong());
                            design::status_chip(ui, name);
                        });
                        if !error.is_empty() {
                            ui.colored_label(studio::unmaker_red(), error);
                        }
                        if design::capsule_button(ui, "Cancel investigation", None,
                            214.0, 44.0, design::CapsuleTone::Red)
                            .on_hover_text("Stop this pass. Saved files are unchanged.").clicked() {
                            cancel = true;
                        }
                    } else if self.extract.session.is_some() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Current stage").size(17.0).strong());
                            design::status_chip(ui, "Ready for review");
                        });
                    } else {
                        ui.label("No investigation is running.");
                        if design::capsule_button(ui, "Go to Import", None,
                            161.0, 44.0, design::CapsuleTone::Blue).clicked() {
                            self.studio.state.stage = Stage::Import;
                        }
                    }
                    if cancel {
                        if let Some(job) = self.extract.job.as_mut() {
                            job_cancel(job);
                            self.studio.state.notice = job.stage_name.clone();
                        }
                    }
                    // Compact/short viewports collapse meta so R3 decisions stay reachable.
                    let collapse_meta = matches!(
                        layout.mode,
                        crate::responsive::LayoutMode::Compact
                    ) || layout.height < 720.0;
                    ui.add_space(11.0);
                    egui::CollapsingHeader::new("Stages reached")
                        .default_open(!collapse_meta)
                        .show(ui, |ui| {
                            if self.studio.state.stage_log.is_empty() {
                                ui.label("Nothing has been recorded yet.");
                            } else {
                                let columns = if ui.available_width() > 660.0 {
                                    3_usize
                                } else if ui.available_width() > 420.0 {
                                    2_usize
                                } else {
                                    1_usize
                                };
                                let max_width =
                                    (ui.available_width() / columns as f32 - 9.0).max(120.0);
                                egui::Grid::new("investigation-checklist")
                                    .num_columns(columns)
                                    .spacing(Vec2::new(9.0, 9.0))
                                    .show(ui, |ui| {
                                        for (index, name) in
                                            self.studio.state.stage_log.iter().enumerate()
                                        {
                                            let (r, _) = ui.allocate_exact_size(
                                                Vec2::new(max_width, 41.0),
                                                Sense::hover(),
                                            );
                                            design::glossy_surface(
                                                ui.painter(),
                                                r,
                                                Color32::from_rgb(233, 244, 255),
                                                false,
                                            );
                                            let shortened = if name.chars().count() > 24 {
                                                format!(
                                                    "{}…",
                                                    name.chars().take(23).collect::<String>()
                                                )
                                            } else {
                                                name.clone()
                                            };
                                            ui.painter().text(
                                                r.center(),
                                                egui::Align2::CENTER_CENTER,
                                                format!("✓ {}. {}", index + 1, shortened),
                                                egui::FontId::proportional(13.0),
                                                studio::unmaker_ink(),
                                            );
                                            if (index + 1) % columns == 0 {
                                                ui.end_row();
                                            }
                                        }
                                    });
                            }
                        });
                    ui.add_space(12.0);
                    if let Some(session) = self.extract.session.clone() {
                        let unresolved = session.coverage.iter()
                            .filter(|row| row.assembly == "unresolved").count();
                        let ready = session.candidates.iter().filter(|candidate| {
                            self.rom.as_ref().is_some_and(|rom| {
                                nesticles_core::reconstruct::assemble(
                                    &rom.bytes, &session.report.payload_sha256,
                                    candidate, &self.studio.state.review_palette.colors(),
                                ).is_ok()
                            })
                        }).count();
                        if !collapse_meta {
                            ui.columns(3, |cols| {
                                for (column, count, label) in [
                                    (0_usize, session.candidates.len(), "Candidates"),
                                    (1_usize, ready, "Reconstructed"),
                                    (2_usize, unresolved, "Unresolved"),
                                ] {
                                    egui::Frame::NONE
                                        .fill(Color32::from_rgb(244, 249, 255))
                                        .corner_radius(CornerRadius::same(16))
                                        .inner_margin(egui::Margin::symmetric(12, 12))
                                        .stroke(Stroke::new(1.0_f32, studio::unmaker_line()))
                                        .show(&mut cols[column], |ui| {
                                            ui.label(RichText::new(count.to_string()).size(27.0)
                                                .strong().color(studio::unmaker_ink()));
                                            ui.label(RichText::new(label).size(13.0)
                                                .color(studio::caption()));
                                        });
                                }
                            });
                            ui.add_space(9.0);
                            ui.label(RichText::new("Only verified assemblies are marked reconstructed; names for the remaining candidates are proposals.")
                                .size(13.0).color(studio::caption()));
                            ui.add_space(10.0);
                            if design::capsule_button(ui, "Open the gallery", None,
                                217.0, 48.0, design::CapsuleTone::Blue).clicked() {
                                self.studio.state.stage = Stage::Discover;
                            }
                            ui.add_space(12.0);
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                design::status_chip(
                                    ui,
                                    &format!("{} candidates", session.candidates.len()),
                                );
                                design::status_chip(ui, &format!("{ready} reconstructed"));
                                design::status_chip(ui, &format!("{unresolved} unresolved"));
                            });
                            ui.add_space(8.0);
                        }
                        let mut reconstructed = Vec::new();
                        for candidate in &session.candidates {
                            let assembled = self.rom.as_ref().is_some_and(|rom| {
                                nesticles_core::reconstruct::assemble(
                                    &rom.bytes,
                                    &session.report.payload_sha256,
                                    candidate,
                                    &self.studio.state.review_palette.colors(),
                                )
                                .is_ok()
                            });
                            if assembled {
                                reconstructed.push(candidate.name.clone());
                            }
                        }
                        self.investigation_issue_panel(ui, &session, &reconstructed);
                    } else if self.extract.job.is_some() {
                        ui.label("Counts appear when this pass finishes.");
                    }
                });
        });
        ui.add_space(13.0);
        ui.label(RichText::new("Investigation tools").size(16.0).strong().color(studio::unmaker_ink()));
        self.tool_row(ui, &[
            (StudioTool::Notes, "Project notes. They are not uploaded."),
            (StudioTool::Providers, "Provider settings. Nothing is sent until you ask."),
            (StudioTool::Guide, "Explanations. They do not change the cartridge."),
        ]);
    }

    fn investigation_issue_panel(
        &mut self,
        ui: &mut egui::Ui,
        session: &nesticles_core::pipeline::ExtractionSession,
        reconstructed: &[String],
    ) {
        use nesticles_core::issues::{decide, latest_kind, DecisionKind};
        ui.label(RichText::new("Open issue").size(18.0).strong().color(studio::unmaker_ink()));
        ui.add_space(6.0);
        let shown = super::model::open_investigation_issue(
            &session.candidates,
            &session.coverage,
            reconstructed,
            &self.project.issue_decisions,
        );
        let Some(shown) = shown else {
            ui.label("No open issue. An unchanged rejected grouping stays hidden. Nothing was invented.");
            return;
        };
        let suggestion = &shown.issue.suggestion;
        ui.label(RichText::new("Issue").size(13.0).color(studio::caption()));
        ui.label(RichText::new(&shown.issue.question).size(16.0).strong().color(studio::unmaker_ink()));
        ui.label(RichText::new(format!("Category: {}", shown.category)).size(13.0).color(studio::caption()));
        ui.add_space(8.0);
        ui.label(RichText::new("Suggested action").size(13.0).color(studio::caption()));
        ui.label(RichText::new(&suggestion.action).size(15.0).strong().color(studio::unmaker_ink()));
        ui.add_space(6.0);
        ui.label(RichText::new("Why").size(13.0).color(studio::caption()));
        ui.label(RichText::new(&suggestion.why).size(14.0).color(studio::unmaker_ink()));
        ui.add_space(6.0);
        ui.label(RichText::new("Uncertainty").size(13.0).color(studio::caption()));
        ui.label(RichText::new(&suggestion.uncertainty).size(14.0).color(studio::caption()));
        ui.add_space(4.0);
        ui.label(RichText::new(&suggestion.accept_effect).size(13.0).color(studio::caption()));
        egui::CollapsingHeader::new("Technical evidence")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(format!("Evidence refs: {}", shown.issue.evidence_ids.join(", ")));
                ui.label(format!(
                    "Fingerprint: {}",
                    nesticles_core::issues::candidate_fingerprint(
                        &shown.issue.asset_id,
                        &suggestion.grouping,
                    )
                ));
                ui.label(&suggestion.summary);
            });
        let state = match latest_kind(&self.project.issue_decisions, &shown.issue) {
            Some(DecisionKind::Accept) => "Accepted — not a verified reconstruction",
            Some(DecisionKind::Skip) => "Skipped — still unresolved",
            Some(DecisionKind::Reject) => "Rejected",
            None => "No decision yet",
        };
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            design::status_chip(ui, state);
            design::status_chip(ui, &format!("Confidence: {}", shown.confidence));
            design::status_chip(ui, "Not a verified finding");
        });
        ui.add_space(8.0);
        ui.label(RichText::new("Decision").size(13.0).color(studio::caption()));
        let issue = shown.issue.clone();
        let mut chosen = None;
        ui.horizontal_wrapped(|ui| {
            if design::capsule_button(ui, "Accept", None, 120.0, 44.0, design::CapsuleTone::Green)
                .on_hover_text(&issue.suggestion.accept_effect)
                .clicked()
            {
                chosen = Some(DecisionKind::Accept);
            }
            if design::capsule_button(ui, "Skip", None, 110.0, 44.0, design::CapsuleTone::White)
                .on_hover_text("Leave the evidence unresolved. The same grouping stays eligible.")
                .clicked()
            {
                chosen = Some(DecisionKind::Skip);
            }
            if design::capsule_button(ui, "Reject", None, 120.0, 44.0, design::CapsuleTone::Red)
                .on_hover_text("Hide this exact grouping. A different grouping can still appear.")
                .clicked()
            {
                chosen = Some(DecisionKind::Reject);
            }
        });
        if let Some(kind) = chosen {
            if latest_kind(&self.project.issue_decisions, &issue) != Some(kind) {
                self.project.issue_decisions.push(decide(&issue, kind));
                self.dirty = true;
            }
            self.studio.state.notice = match kind {
                DecisionKind::Accept => format!("Accepted. {}", issue.suggestion.accept_effect),
                DecisionKind::Skip => {
                    "Skipped. The evidence is still unresolved and stays eligible.".into()
                }
                DecisionKind::Reject => {
                    "Rejected. That exact grouping will not be offered again.".into()
                }
            };
        }
    }

    fn discover_stage(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        layout: ResponsiveLayout,
    ) {
        self.tool_row(
            ui,
            &[
                (StudioTool::Tiles, "Inspect the raw tiles from the cartridge."),
                (StudioTool::Audio, "Inspect sounds kept in the project."),
            ],
        );
        ui.add_space(8.0);
        let Some(session) = self.extract.session.clone() else {
            ui.label("There is no investigation to show. Start one from Import.");
            if design::secondary_button(ui, "Go to Import").clicked() {
                self.studio.state.stage = Stage::Import;
            }
            return;
        };
        egui::ScrollArea::horizontal()
            .id_salt("discover-filters")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if design::capsule_button(ui, "All",
                        self.ui_assets.filter("all", self.studio.state.filter.is_none()),
                        92.0, 46.0,
                        if self.studio.state.filter.is_none() { design::CapsuleTone::Blue }
                        else { design::CapsuleTone::White }).clicked() {
                        self.studio.state.filter = None;
                    }
                    for group in Group::all() {
                        let on = self.studio.state.filter == Some(group);
                        let label = group.label();
                        let key = label.to_lowercase();
                        let width = (label.chars().count() as f32 * 8.5 + 66.0).clamp(112.0, 170.0);
                        if design::capsule_button(ui, label,
                            self.ui_assets.filter(&key, on), width, 46.0,
                            if on { design::CapsuleTone::Blue } else { design::CapsuleTone::White }
                        ).clicked() { self.studio.state.filter = Some(group); }
                    }
                });
            });
        ui.add_space(6.0);
        let search_w = ui.available_width().min(300.0).max(160.0);
        ui.add(egui::TextEdit::singleline(&mut self.studio.state.gallery_query)
            .hint_text("Search assets...")
            .desired_width(search_w)
            .min_size(Vec2::new(search_w.min(220.0), 40.0)));
        ui.add_space(10.0);
        let width = ui.available_width();
        let split = layout.show_discover_inspector(width);
        if split {
            let inspector_w = layout.discover_inspector_width().max(240.0);
            let gallery_w = (width - inspector_w - 16.0)
                .max(layout.gallery_card_width(width.min(400.0)));
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(gallery_w, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.discover_gallery(ui, ctx, &session, layout),
                );
                ui.add_space(16.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(inspector_w, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("discover-inspector-scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.discover_inspector(ui, ctx, &session));
                    },
                );
            });
        } else {
            self.discover_gallery(ui, ctx, &session, layout);
            if self.studio.state.selected.is_some() {
                ui.add_space(10.0);
                egui::CollapsingHeader::new("Selected asset")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("discover-inspector-drawer")
                            .max_height(ui.available_height().min(280.0).max(160.0))
                            .show(ui, |ui| self.discover_inspector(ui, ctx, &session));
                    });
            }
        }
    }

    fn discover_shown(
        &self,
        session: &nesticles_core::pipeline::ExtractionSession,
    ) -> (
        Vec<super::model::GalleryItem>,
        Option<Result<super::model::GoombaDiscovery, String>>,
    ) {
        let items = gallery_items(&session.candidates, &session.coverage);
        let discovery = self.goomba_discovery(session);
        let filter = self.studio.state.filter;
        let query = self.studio.state.gallery_query.to_lowercase();
        let mut shown: Vec<_> = items
            .into_iter()
            .filter(|item| filter.is_none_or(|group| item.group == group))
            .filter(|item| query.is_empty() || item.name.to_lowercase().contains(&query))
            .collect();
        if let Some(Ok(found)) = &discovery {
            if let Some(index) = shown.iter().position(|item| item.id == found.candidate_id) {
                let item = shown.remove(index);
                shown.insert(0, item);
            }
        }
        (shown, discovery)
    }

    fn discover_gallery(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &nesticles_core::pipeline::ExtractionSession,
        layout: ResponsiveLayout,
    ) {
        let (shown, discovery) = self.discover_shown(session);
        let goomba_art = self.assemble_goomba(session);
        if shown.is_empty() {
            ui.label("No assets match this filter.");
            return;
        }
        let scroll_h = ui.available_height().max(240.0);
        egui::ScrollArea::vertical()
            .id_salt("discover-gallery")
            .auto_shrink([false, false])
            .max_height(scroll_h)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                let width = ui.available_width();
                let cols = layout.gallery_columns(width);
                let card_w = layout.gallery_card_width(width);
                for row in shown.chunks(cols) {
                    ui.horizontal_top(|ui| {
                        // Native egui adds automatic inter-widget spacing.
                        // Eliminate it here so cards follow an exact even grid.
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for (index, item) in row.iter().enumerate() {
                            let is_goomba = session.candidates.iter().any(|candidate| {
                                candidate.id == item.id && candidate.character_id == "goomba"
                            });
                            let card = gallery_card(item, is_goomba, discovery.as_ref());
                            self.discover_card(
                                ui,
                                ctx,
                                session,
                                item,
                                &card,
                                goomba_art.as_ref(),
                                card_w,
                            );
                            if index + 1 < row.len() {
                                ui.add_space(studio::GALLERY_GAP);
                            }
                        }
                    });
                    ui.add_space(studio::GALLERY_GAP);
                }
            });
    }

    fn discover_card(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &nesticles_core::pipeline::ExtractionSession,
        item: &super::model::GalleryItem,
        card: &super::model::GalleryCard,
        goomba_art: Option<&nesticles_core::reconstruct::Assembly>,
        card_w: f32,
    ) {
        let selected = self.studio.state.selected.as_deref() == Some(item.id.as_str());
        let reconstructed = card.status.starts_with("Reconstructed");
        let stroke = if selected { Stroke::new(2.0_f32, studio::accent_yellow()) }
            else { Stroke::new(1.0_f32, studio::unmaker_line()) };
        let fill = if selected { Color32::from_rgb(255, 250, 218) }
            else { studio::unmaker_paper() };
        let frame = egui::Frame::NONE
            .fill(fill).stroke(stroke).corner_radius(CornerRadius::same(20))
            .inner_margin(12.0).shadow(design::soft_shadow());
        // The whole card occupies one *fixed* footprint. The inner minimum
        // excludes the frame margins; this removes the old diagonal step layout.
        let card_height = 203.0;
        ui.allocate_ui_with_layout(Vec2::new(card_w, card_height),
            egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(card_w);
                let inner = frame.show(ui, |ui| {
                    ui.set_min_size(Vec2::new(card_w - 24.0, card_height - 24.0));
                    ui.set_max_width(card_w - 24.0);
                    ui.vertical_centered(|ui| {
                        if reconstructed {
                            if let Some(assembly) = goomba_art {
                                self.assembly_thumb(ui, ctx, assembly);
                            } else { self.thumb(ui, ctx, None); }
                        } else if item.tile_ids.is_empty() {
                            self.thumb(ui, ctx, None);
                        } else {
                            self.thumb(ui, ctx, item.tile_ids.first().copied());
                        }
                        ui.add_space(3.0);
                        ui.label(RichText::new(&card.title).size(17.0).strong()
                            .color(studio::unmaker_ink()));
                        design::status_chip(ui, &card.status);
                        ui.label(RichText::new(&card.summary).size(12.0)
                            .color(studio::caption()));
                    });
                });
                // One click selects the inspector. Double-click opens Review.
                let card_id = ui.id().with(("discover-card", item.id.as_str()));
                let response = ui.interact(inner.response.rect, card_id, Sense::click());
                if response.clicked() {
                    self.studio.state.selected = Some(item.id.clone());
                }
                if response.double_clicked() {
                    self.open_candidate_in_review(session, &item.id);
                }
                response.on_hover_text(&card.open_label);
            });
    }

    fn discover_inspector(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &nesticles_core::pipeline::ExtractionSession,
    ) {
        let frame = design::floating_frame(studio::unmaker_paper());
        frame.show(ui, |ui| {
            ui.set_min_width(studio::INSPECTOR_W - 36.0);
            egui::ScrollArea::vertical()
                .id_salt("discover-inspector")
                .max_height(ui.available_height().max(200.0))
                .show(ui, |ui| self.discover_inspector_body(ui, ctx, session));
        });
    }

    /// Compact Discover preview. The full evidence, pose notes and timing
    /// controls remain available in Review; Discover is for quick selection.
    fn discover_inspector_body(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &nesticles_core::pipeline::ExtractionSession,
    ) {
        let Some(id) = self.studio.state.selected.clone() else {
            ui.label(RichText::new("Select an asset").size(20.0).strong());
            ui.label("Choose a Discover card to see its preview here.");
            return;
        };
        let (shown, discovery) = self.discover_shown(session);
        let Some(item) = shown.into_iter().find(|item| item.id == id) else {
            ui.label("The selection is hidden by the current filter.");
            return;
        };
        let is_goomba = session.candidates.iter().any(|candidate| {
            candidate.id == item.id && candidate.character_id == "goomba"
        });
        let card = gallery_card(&item, is_goomba, discovery.as_ref());

        ui.horizontal(|ui| {
            ui.label(RichText::new(&card.title).size(27.0).strong()
                .color(studio::unmaker_ink()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::design::capsule_button(ui, "✕", None, 38.0, 34.0,
                    crate::design::CapsuleTone::White).clicked() {
                    self.studio.state.selected = None;
                }
            });
        });
        design::status_chip(ui, &card.status);
        ui.add_space(6.0);

        let reconstructed = if let Some(Ok(found)) = discovery.as_ref() {
            if found.candidate_id == item.id { self.assemble_goomba(session) }
            else { None }
        } else { None };

        if let Some(assembly) = reconstructed.as_ref() {
            self.compact_discover_preview(ui, ctx, assembly);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Form").strong().size(16.0));
                ui.add_space(14.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(249, 251, 255))
                    .corner_radius(CornerRadius::same(17))
                    .stroke(Stroke::new(1.0_f32, studio::unmaker_line()))
                    .inner_margin(egui::Margin::symmetric(16, 7))
                    .show(ui, |ui| { ui.label("Default"); });
            });
            ui.add_space(6.0);
            ui.label(RichText::new("Animation State").size(16.0).strong());
            let mut poses: Vec<String> = Vec::new();
            for frame in &assembly.frames {
                if !poses.iter().any(|name| name == &frame.state) {
                    poses.push(frame.state.clone());
                }
            }
            ui.horizontal(|ui| {
                let width = ((ui.available_width() - 9.0) / 2.0).max(110.0);
                for pose in &poses {
                    let active = self.studio.state.preview_pose == *pose;
                    if design::capsule_button(ui, &label_for_state(pose), None,
                        width, 43.0,
                        if active { design::CapsuleTone::Red }
                        else { design::CapsuleTone::White }
                    ).clicked() {
                        self.studio.state.preview_pose = pose.clone();
                        self.studio.state.preview_elapsed = 0;
                        self.studio.preview_accum = 0.0;
                        self.studio.state.preview_playing = true;
                    }
                }
            });
            let frames = assembly.frames.iter()
                .filter(|frame| frame.state == self.studio.state.preview_pose)
                .count();
            let loops = assembly.frames.iter().any(|frame| {
                frame.state == self.studio.state.preview_pose
                    && frame.playback == nesticles_core::anim::Playback::Loop
            });
            ui.label(RichText::new(format!("{} frames · {}", frames,
                if loops { "Loops" } else { "Plays once" })).size(14.0)
                .color(studio::unmaker_ink()));
            ui.add_space(5.0);
            design::status_chip(ui, "Review colors only — palette unverified");
        } else {
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width().max(180.0), 182.0), Sense::hover());
            checker(ui.painter(), rect);
            if !item.tile_ids.is_empty() {
                ui.label("Source tile only — not reconstructed");
                self.thumb(ui, ctx, item.tile_ids.first().copied());
            } else {
                ui.label("Artwork has not been reconstructed.");
            }
        }
        ui.add_space(12.0);
        let width = ui.available_width().max(174.0);
        if design::capsule_button(ui, "Open in Review",
            self.ui_assets.nav("review", true), width, 52.0,
            design::CapsuleTone::Red).clicked() {
            self.open_candidate_in_review(session, &item.id);
        }
    }

    fn compact_discover_preview(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        assembly: &nesticles_core::reconstruct::Assembly,
    ) {
        self.advance_goomba_preview(ctx);
        let pose = self.studio.state.preview_pose.clone();
        let elapsed = self.studio.state.preview_elapsed;
        let frame_and_idx = nesticles_core::reconstruct::pose_frame(assembly, &pose, elapsed)
            .or_else(|| assembly.frames.first().map(|frame| (0_usize, frame)));
        let Some((index, frame)) = frame_and_idx else { return };
        let size = Vec2::new(ui.available_width().max(180.0), 180.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        ui.painter().rect_filled(rect, 20.0, Color32::from_rgb(249, 251, 255));
        let check = rect.shrink(5.0);
        checker(ui.painter(), check);
        ui.painter().rect_stroke(rect, 20.0,
            Stroke::new(1.8_f32, studio::unmaker_line()), egui::StrokeKind::Inside);
        let max_h = (size.y - 24.0).max(1.0);
        let max_w = (size.x - 34.0).max(1.0);
        let zoom = (max_w / frame.w as f32).min(max_h / frame.h as f32).min(8.0);
        let draw_size = Vec2::new(frame.w as f32 * zoom, frame.h as f32 * zoom);
        let dst = Rect::from_center_size(rect.center(), draw_size);
        let key = format!("discover-compact-{:?}-{}-{}", self.studio.state.review_palette,
            frame.state, index);
        let texture = if let Some(texture) = self.studio.assembled.get(&key) {
            texture.clone()
        } else {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.w as usize, frame.h as usize], &frame.rgba);
            let texture = ctx.load_texture(&key, image, TextureOptions::NEAREST);
            self.studio.assembled.insert(key, texture.clone());
            texture
        };
        ui.painter().image(texture.id(), dst,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }

    fn review_stage(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(session) = self.extract.session.clone() else {
            ui.label("There is nothing to review until an investigation finishes.");
            return;
        };
        let issues = issue_queue(&session.candidates, &session.coverage);
        if issues.is_empty() {
            ui.label("This pass did not produce an issue.");
            return;
        }
        let index = self.studio.state.issue_index.min(issues.len() - 1);
        self.studio.state.issue_index = index;
        let issue = issues[index].clone();
        if self.character_review(ui, ctx, &session, &issue) {
            return;
        }
        ui.label(RichText::new("Review one issue").size(28.0));
        ui.label(format!("Issue {} of {}", index + 1, issues.len()));
        ui.label(RichText::new(&issue.name).size(22.0));
        ui.label(&issue.uncertainty);
        ui.label(&issue.evidence);
        ui.label(&issue.attempted);
        ui.label(&issue.next_action);
        if let Some(suggestion) = &issue.suggestion {
            ui.label(RichText::new(format!("Suggested action: {suggestion}")).size(14.0));
            ui.label(
                "Accept / Skip / Reject for this proposal are recorded on Investigate. Review here does not invent a second decision store.",
            );
        } else {
            ui.label("Suggested fix: none. No automatic correction engine is running.");
        }
        ui.label(
            "Ignored candidates stay in the session. The Ignored filter on Discover lists them.",
        );
        let item = gallery_items(&session.candidates, &session.coverage)
            .into_iter()
            .find(|item| item.name == issue.name);
        self.preview_column(ui, ctx, item.as_ref(), &session.candidates);
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let _ = ui
                .add_enabled(false, egui::Button::new("Accept Suggested Fix"))
                .on_hover_text("There is no suggested fix to accept.")
                .on_disabled_hover_text("There is no suggested fix to accept.");
            let _ = ui
                .add_enabled(false, egui::Button::new("Try Another Solution"))
                .on_disabled_hover_text("No alternative solution has been prepared.");
            let failed = session.candidates.iter().any(|candidate| {
                issue.candidate_id.as_ref() == Some(&candidate.id)
                    && matches!(
                        candidate.status,
                        nesticles_core::pipeline::ReviewStatus::Failed
                            | nesticles_core::pipeline::ReviewStatus::Reinspecting
                    )
            });
            if ui
                .add_enabled(failed, egui::Button::new("Investigate Further"))
                .on_disabled_hover_text(
                    "The same profile was already applied. Further investigation starts from a failed candidate.",
                )
                .clicked()
            {
                self.reinspect_same(&issue);
            }
            if ui
                .add_enabled(issue.candidate_id.is_some(), egui::Button::new("Fix Manually"))
                .on_hover_text("Place ROM tiles yourself. This does not invent a match.")
                .clicked()
            {
                self.open_manual(&issue);
            }
            if ui
                .add_enabled(issue.candidate_id.is_some(), egui::Button::new("Needs reinspection"))
                .on_hover_text("Record that the grouping is not accepted. This does not publish anything.")
                .clicked()
            {
                self.fail_issue(&issue);
            }
            let ignored = session.candidates.iter().any(|candidate| {
                issue.candidate_id.as_ref() == Some(&candidate.id)
                    && candidate.status == nesticles_core::pipeline::ReviewStatus::Ignored
            });
            if ui
                .add_enabled(ignored, egui::Button::new("Restore"))
                .on_disabled_hover_text("Only an ignored candidate can be restored.")
                .clicked()
            {
                self.restore_issue(&issue);
            }
            if crate::design::soft_button(ui, "Skip").on_hover_text("Leave this issue unchanged.").clicked() {
                self.studio.state.notice = skip_feedback(&issue.name).message;
                self.studio.state.issue_index = (index + 1) % issues.len();
            }
            if ui
                .add_enabled(issue.candidate_id.is_some(), egui::Button::new("Ignore"))
                .on_hover_text("Hide this candidate. It is not deleted.")
                .clicked()
            {
                self.ignore_issue(&issue);
            }
        });
        ui.add_space(8.0);
        if ui
            .add(design::primary_widget("Mark candidate reviewed"))
            .on_hover_text("Changes the review flag only. It does not publish a sprite.")
            .clicked()
        {
            self.mark_reviewed(&issue);
        }
        if crate::design::soft_button(ui, "Return this candidate to the gallery").clicked() {
            self.studio.state.selected = issue.candidate_id.clone();
            self.studio.state.stage = Stage::Discover;
            self.studio.state.show_manual = false;
        }
        if self.studio.state.show_manual {
            ui.separator();
            ui.label("Manual tile placement uses the existing puzzle. It is not an automatic fix.");
            self.puzzle_view(ui);
        }
    }

    fn create_stage(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label("Original sprites live in the project. They are separate from ROM candidates.");
        ui.label("Assemble and Animate edit sprites already in the project. A saved Goomba plays here without the cartridge.");
        self.tool_row(
            ui,
            &[(
                StudioTool::Behavior,
                "Existing behavior graph. It is not a second physics model.",
            )],
        );
        ui.horizontal(|ui| {
            if ui
                .selectable_label(!self.studio.state.create_animate, "Assemble")
                .clicked()
            {
                self.studio.state.create_animate = false;
            }
            if ui
                .selectable_label(self.studio.state.create_animate, "Animate")
                .clicked()
            {
                self.studio.state.create_animate = true;
            }
        });
        if self.studio.state.create_animate {
            self.animate_view(ui, ctx);
            ui.separator();
            ui.label("Timeline and inspector from the existing animation editor.");
            self.anim_timeline(ui);
            ui.separator();
            self.anim_inspector(ui);
        } else {
            self.assemble_view(ui, ctx);
            ui.separator();
            ui.label("Sprites already in this project.");
            self.sprite_browser(ui);
        }
    }

    fn save_stage(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Save writes the project file. It does not store the extraction session, the ROM, or a published Goomba animation.",
        );
        ui.label(
            "Package export still paints only the first frame and is not offered here as a finished asset.",
        );
        if crate::design::soft_button(ui, "Save project").clicked() {
            self.save_project_quick();
            self.studio.state.notice = if self.dirty {
                "The project save did not finish.".into()
            } else {
                "Project file saved. No extracted character was published.".into()
            };
        }
        ui.add_space(8.0);
        ui.label("Still true after this save:");
        ui.label("• Approving the reconstructed Goomba writes its sprite. Other approvals only change a review flag.");
        ui.label("• A new package keeps every frame in pictures. An older reader still sees the first frame.");
        ui.label("• Mario and Luigi have no reconstruction.");
        ui.label("• Reference pages are not matched to ROM tiles.");
        ui.label("• Palettes and animation timings are not verified.");
        ui.label("• Live OpenAI has not been accepted. This screen does not call it.");
        ui.add_space(8.0);
        ui.label("Library, package export, and the overworld list are the existing tools.");
        self.workshop_files(ui);
        self.tool_row(
            ui,
            &[
                (
                    StudioTool::Library,
                    "Local packages and recipes. Nothing here publishes a reconstructed sprite.",
                ),
                (
                    StudioTool::Export,
                    "Package export still paints the first frame only.",
                ),
                (
                    StudioTool::Overworld,
                    "Existing overworld list. It does not build a map by itself.",
                ),
            ],
        );
    }

    fn tool_row(&mut self, ui: &mut egui::Ui, tools: &[(StudioTool, &str)]) {
        ui.horizontal_wrapped(|ui| {
            for (tool, tip) in tools {
                if crate::design::soft_button(ui, tool.label()).on_hover_text(*tip).clicked() {
                    self.studio.state.tool = Some(*tool);
                    self.studio.state.notice = format!("{} — {}", tool.label(), tool.note());
                }
            }
        });
    }

    fn embedded_tool(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, tool: StudioTool) {
        if design::capsule_button(ui, "Back", None,
            120.0, 43.0, design::CapsuleTone::White)
            .on_hover_text("Return to this stage.")
            .clicked()
        {
            self.studio.state.tool = None;
            return;
        }
        ui.label(RichText::new(tool.label()).size(28.0));
        ui.label(tool.note());
        ui.separator();
        match tool {
            StudioTool::Tiles => {
                self.reference_banner(ui);
                self.cartridge(ui);
                ui.separator();
                self.tiles(ui, ctx);
            }
            StudioTool::Audio => self.audio(ui),
            StudioTool::Library => self.library(ui),
            StudioTool::Export => self.export_tab(ui),
            StudioTool::Notes => self.notes(ui),
            StudioTool::Overworld => self.overworld(ui),
            StudioTool::Providers => self.providers_view(ui),
            StudioTool::Behavior => self.behavior_view(ui),
            StudioTool::Guide => self.guide(ui),
        }
    }

    fn apply_studio_tool_env(&mut self) {
        if self.studio.tool_env_applied {
            return;
        }
        self.studio.tool_env_applied = true;
        let Ok(name) = std::env::var("NESTICLES_STUDIO_TOOL") else {
            return;
        };
        let name = name.to_ascii_lowercase();
        if name == "animate" {
            self.studio.state.stage = Stage::Create;
            self.studio.state.create_animate = true;
            self.studio.state.tool = None;
            return;
        }
        if name == "assemble" {
            self.studio.state.stage = Stage::Create;
            self.studio.state.create_animate = false;
            self.studio.state.tool = None;
            return;
        }
        if let Some(tool) = StudioTool::parse(&name) {
            self.studio.state.tool = Some(tool);
        }
    }

    fn preview_column(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        item: Option<&super::model::GalleryItem>,
        candidates: &[nesticles_core::pipeline::Candidate],
    ) {
        ui.label(RichText::new("Preview").size(20.0));
        self.review_palette_bar(ui);
        let Some(item) = item else {
            ui.label("Choose a candidate to see its evidence.");
            return;
        };
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.id == item.id)
            .cloned();
        let palette = self.studio.state.review_palette.colors();
        let payload = self
            .extract
            .session
            .as_ref()
            .map(|session| session.report.payload_sha256.clone());
        let bytes = self.rom.as_ref().map(|rom| rom.bytes.clone());
        let assembly = match (candidate.as_ref(), bytes.as_deref(), payload.as_deref()) {
            (Some(candidate), Some(bytes), Some(payload)) => Some((
                candidate.character_id.clone(),
                nesticles_core::reconstruct::assemble(bytes, payload, candidate, &palette),
            )),
            _ => None,
        };
        match assembly {
            Some((_, Ok(assembly))) => {
                self.draw_assembly(ui, ctx, &assembly);
                return;
            }
            Some((id, Err(err))) if id == "goomba" => {
                ui.colored_label(studio::unmaker_red(), &err);
                ui.label("Source tiles were not shown in place of this failure.");
                return;
            }
            _ => {}
        }
        let frames = candidate
            .as_ref()
            .map(reconstructed_frame_count)
            .unwrap_or(0);
        let policy = preview_policy(frames, false);
        if !policy.message.is_empty() {
            ui.label(RichText::new(&policy.message).size(16.0));
        }
        ui.label(&policy.palette_label);
        ui.label(&policy.timing_label);
        if item.tile_ids.is_empty() {
            ui.add_space(8.0);
            ui.label(NOT_RECONSTRUCTED);
            return;
        }
        ui.label("Source tiles from the cartridge. Not a reconstructed pose.");
        let index = self
            .studio
            .state
            .evidence_index
            .min(item.tile_ids.len().saturating_sub(1));
        self.studio.state.evidence_index = index;
        let tile = item.tile_ids[index];
        self.draw_evidence(ui, ctx, tile);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(index > 0, egui::Button::new("Previous evidence"))
                .clicked()
            {
                self.studio.state.evidence_index = index.saturating_sub(1);
            }
            ui.label(format!(
                "Evidence {} of {} · tile 0x{tile:02X}",
                index + 1,
                item.tile_ids.len()
            ));
            if ui
                .add_enabled(
                    index + 1 < item.tile_ids.len(),
                    egui::Button::new("Next evidence"),
                )
                .clicked()
            {
                self.studio.state.evidence_index = index + 1;
            }
            if crate::design::soft_button(ui, "Zoom in").clicked() {
                self.studio.state.zoom = (self.studio.state.zoom + 1.0).min(16.0);
            }
            if crate::design::soft_button(ui, "Zoom out").clicked() {
                self.studio.state.zoom = (self.studio.state.zoom - 1.0).max(2.0);
            }
            let _ = ui
                .add_enabled(policy.can_play, egui::Button::new("Play"))
                .on_disabled_hover_text("There are no reconstructed frames to play.");
        });
    }

    fn draw_assembly(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        assembly: &nesticles_core::reconstruct::Assembly,
    ) {
        self.advance_goomba_preview(ctx);
        ui.label(RichText::new("Animation preview").size(19.0).strong());
        ui.label(RichText::new("Assembled from your cartridge. Colors remain provisional.").size(13.0).color(studio::caption()));
        let mut poses = Vec::new();
        for frame in &assembly.frames {
            if poses.iter().all(|id: &String| id != &frame.state) {
                poses.push(frame.state.clone());
            }
        }
        ui.horizontal_wrapped(|ui| {
            for pose in &poses {
                let selected = self.studio.state.preview_pose == *pose;
                if design::capsule_button(ui, &label_for_state(pose), None,
                    138.0, 44.0,
                    if selected { design::CapsuleTone::Red } else { design::CapsuleTone::White }
                ).on_hover_text(label_for_state(pose)).clicked() && !selected
                {
                    self.studio.state.preview_pose = pose.clone();
                    self.studio.state.preview_elapsed = 0;
                    self.studio.preview_accum = 0.0;
                    self.studio.state.preview_playing = false;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            let playing = self.studio.state.preview_playing;
            if !playing && design::capsule_button(ui, "▶ Play", None,
                108.0, 44.0, design::CapsuleTone::Green)
                .on_hover_text("Play the selected state").clicked()
            {
                self.studio.state.preview_playing = true;
            }
            if design::secondary_button(ui, "Pause")
                .on_hover_text("Pause on the current frame")
                .clicked()
            {
                self.studio.state.preview_playing = false;
            }
            if design::secondary_button(ui, "Zoom in").clicked() {
                self.studio.state.zoom = (self.studio.state.zoom + 1.0).min(16.0);
            }
            if design::secondary_button(ui, "Zoom out").clicked() {
                self.studio.state.zoom = (self.studio.state.zoom - 1.0).max(2.0);
            }
        });
        let pose = self.studio.state.preview_pose.clone();
        let elapsed = self.studio.state.preview_elapsed;
        let Some((index, frame)) =
            nesticles_core::reconstruct::pose_frame(assembly, &pose, elapsed)
        else {
            ui.label("That animation state is not in this reconstruction.");
            return;
        };
        let in_pose = assembly
            .frames
            .iter()
            .filter(|item| item.state.eq_ignore_ascii_case(&pose))
            .count();
        let looping = frame.playback == nesticles_core::anim::Playback::Loop;
        ui.label(format!(
            "{} · frame {} of {} · {} · {} ticks",
            label_for_state(&frame.state),
            index + 1,
            in_pose,
            if looping { "looping" } else { "plays once" },
            frame.ticks
        ));
        ui.label(&frame.note);
        let zoom = self.studio.state.zoom;
        let sprite = Vec2::new(frame.w as f32 * zoom, frame.h as f32 * zoom);
        let well = Vec2::new(
            ui.available_width().clamp(180.0, 480.0),
            sprite.y.max(160.0).min(240.0),
        );
        let (rect, _) = ui.allocate_exact_size(well, Sense::hover());
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(16),
            Color32::from_rgb(236, 240, 246),
        );
        let image_rect = Rect::from_center_size(rect.center(), sprite.min(well * 0.92));
        checker(ui.painter(), image_rect);
        let key = format!(
            "{:?}-{}-{}",
            self.studio.state.review_palette, frame.state, index
        );
        let texture = if let Some(texture) = self.studio.assembled.get(&key) {
            texture.clone()
        } else {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.w as usize, frame.h as usize],
                &frame.rgba,
            );
            let texture = ctx.load_texture(&key, image, TextureOptions::NEAREST);
            self.studio.assembled.insert(key, texture.clone());
            texture
        };
        ui.painter().image(
            texture.id(),
            image_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        design::status_chip(
            ui,
            "Provisional palette — original NES colors are not verified",
        );
        ui.label(&assembly.palette_note);
        ui.label(&assembly.timing_note);
    }

    fn advance_goomba_preview(&mut self, ctx: &egui::Context) {
        if !self.studio.state.preview_playing {
            return;
        }
        let dt = ctx.input(|input| input.stable_dt);
        self.studio.preview_accum += dt;
        let step = 1.0 / 60.0;
        let mut ticks = 0;
        while self.studio.preview_accum >= step && ticks < 8 {
            self.studio.preview_accum -= step;
            self.studio.state.preview_elapsed = self.studio.state.preview_elapsed.saturating_add(1);
            ticks += 1;
        }
        ctx.request_repaint();
    }

    fn character_review(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        session: &nesticles_core::pipeline::ExtractionSession,
        issue: &IssueCard,
    ) -> bool {
        let Some(candidate) = session.candidates.iter().find(|candidate| {
            issue.candidate_id.as_deref() == Some(candidate.id.as_str())
                && candidate.character_id == "goomba"
        }) else {
            return false;
        };
        design::screen_title(ui, "Review character", "Inspect animation states, then approve the completed asset.");
        match self.goomba_discovery(session) {
            Some(Ok(found)) if found.candidate_id == candidate.id => {
                let saved =
                    candidate.status == nesticles_core::pipeline::ReviewStatus::Approved;
                let Some(sheet) = character_sheet(candidate, Some(&found), saved) else {
                    return false;
                };
                ui.label(RichText::new(&sheet.name).size(26.0));
                ui.label(&sheet.status);
                ui.label(save_status_line(sheet.saved));
                if let Some(form) = sheet.forms.first() {
                    ui.label(format!("Form: {}", form.label));
                    ui.label(
                        "Walk and Defeat / Stomped are animation states of this one character.",
                    );
                }
                ui.label(
                    "The 8-frame interval is the graphics-handler test, not a filmed measurement.",
                );
                if ui
                    .add(design::primary_widget(&approve_label(&sheet.name)))
                    .on_hover_text(
                        "Writes the assembled walk and defeat into this project. The cartridge is not stored.",
                    )
                    .clicked()
                {
                    self.publish_selected_goomba();
                }
                if let Some(assembly) = self.assemble_goomba(session) {
                    self.draw_assembly(ui, ctx, &assembly);
                }
                self.source_evidence(ui, session, candidate);
                if crate::design::soft_button(ui, "Return to Discover").clicked() {
                    self.studio.state.stage = Stage::Discover;
                    self.studio.state.show_manual = false;
                }
                true
            }
            Some(Err(err)) => {
                ui.label(RichText::new(&candidate.name).size(26.0));
                ui.colored_label(studio::unmaker_red(), err);
                ui.label("Source tiles were not shown in place of this failure.");
                if crate::design::soft_button(ui, "Return to Discover").clicked() {
                    self.studio.state.stage = Stage::Discover;
                }
                true
            }
            _ => false,
        }
    }

    fn source_evidence(
        &mut self,
        ui: &mut egui::Ui,
        session: &nesticles_core::pipeline::ExtractionSession,
        candidate: &nesticles_core::pipeline::Candidate,
    ) {
        egui::CollapsingHeader::new("Source evidence")
            .default_open(false)
            .show(ui, |ui| {
                let report = &session.report;
                ui.label(format!(
                    "Payload {} · {} · CHR {} bytes",
                    &report.payload_sha256[..12.min(report.payload_sha256.len())],
                    report.mirroring,
                    report.chr_bytes
                ));
                ui.label(format!("Profile: {}", report.profile_label));
                let tiles = candidate
                    .tile_ids()
                    .iter()
                    .map(|tile| format!("0x{tile:02X}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.label(format!("CHR indexes: {tiles}"));
                ui.label(
                    "Walk is one 16×24 pose from 0x70, 0x71, 0x72, and 0x73. The second frame is that pose mirrored.",
                );
                ui.label(
                    "Defeat / Stomped is tile 0xEF on the left and the same tile flipped on the right. It plays once.",
                );
            });
    }

    fn open_candidate_in_review(
        &mut self,
        session: &nesticles_core::pipeline::ExtractionSession,
        id: &str,
    ) {
        self.studio.state.selected = Some(id.to_string());
        self.studio.state.evidence_index = 0;
        self.studio.state.preview_pose = "walk".into();
        self.studio.state.preview_elapsed = 0;
        self.studio.preview_accum = 0.0;
        self.studio.state.preview_playing = false;
        let issues = issue_queue(&session.candidates, &session.coverage);
        if let Some(index) = issues
            .iter()
            .position(|issue| issue.candidate_id.as_deref() == Some(id))
        {
            self.studio.state.issue_index = index;
        }
        self.studio.state.stage = Stage::Review;
        self.studio.state.show_manual = false;
    }

    fn assemble_goomba(
        &self,
        session: &nesticles_core::pipeline::ExtractionSession,
    ) -> Option<nesticles_core::reconstruct::Assembly> {
        let rom = self.rom.as_ref()?;
        let candidate = session
            .candidates
            .iter()
            .find(|candidate| candidate.character_id == "goomba")?;
        nesticles_core::reconstruct::assemble(
            &rom.bytes,
            &session.report.payload_sha256,
            candidate,
            &self.studio.state.review_palette.colors(),
        )
        .ok()
    }

    fn assembly_thumb(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        assembly: &nesticles_core::reconstruct::Assembly,
    ) {
        let Some((_, frame)) = nesticles_core::reconstruct::pose_frame(assembly, "walk", 0) else {
            self.thumb(ui, ctx, None);
            return;
        };
        let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 80.0), Sense::hover());
        checker(ui.painter(), rect);
        let key = format!("thumb-{:?}-walk-0", self.studio.state.review_palette);
        let texture = if let Some(texture) = self.studio.assembled.get(&key) {
            texture.clone()
        } else {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.w as usize, frame.h as usize],
                &frame.rgba,
            );
            let texture = ctx.load_texture(&key, image, TextureOptions::NEAREST);
            self.studio.assembled.insert(key, texture.clone());
            texture
        };
        let size = Vec2::new(frame.w as f32 * 2.5, frame.h as f32 * 2.5);
        ui.painter().image(
            texture.id(),
            Rect::from_center_size(rect.center(), size),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    fn goomba_discovery(
        &self,
        session: &nesticles_core::pipeline::ExtractionSession,
    ) -> Option<Result<super::model::GoombaDiscovery, String>> {
        let rom = self.rom.as_ref()?;
        discover_goomba(
            &rom.bytes,
            &session.report.payload_sha256,
            &session.candidates,
            &self.studio.state.review_palette.colors(),
        )
    }

    /// Select the reconstructed Goomba when an investigation finishes and the
    /// user has not chosen someone else. Launch hooks are not involved.
    pub(crate) fn select_reconstructed_goomba(&mut self) {
        if self.studio.state.selected.is_some() {
            return;
        }
        let Some(session) = self.extract.session.clone() else {
            return;
        };
        if let Some(Ok(found)) = self.goomba_discovery(&session) {
            self.studio.state.selected = Some(found.candidate_id);
        }
    }

    pub(crate) fn publish_selected_goomba(&mut self) {
        let Some(bytes) = self.rom.as_ref().map(|rom| rom.bytes.clone()) else {
            self.studio.state.notice = "Open the cartridge before saving a Goomba.".into();
            self.status = self.studio.state.notice.clone();
            return;
        };
        let Some(candidate) = self.extract.session.as_ref().and_then(|session| {
            session
                .candidates
                .iter()
                .find(|candidate| candidate.character_id == "goomba")
                .cloned()
        }) else {
            self.studio.state.notice = "There is no Goomba candidate to save.".into();
            self.status = self.studio.state.notice.clone();
            return;
        };
        let palette = self.studio.state.review_palette.colors();
        match nesticles_core::reconstruct::publish_goomba(
            &mut self.project,
            &bytes,
            &candidate,
            &palette,
        ) {
            Ok(published) => {
                self.sprite_id = Some(published.sprite_id.clone());
                self.set_id = Some(published.set_id.clone());
                self.clip_id = Some(published.walk_clip_id.clone());
                self.frame_id = published.frame_ids.first().cloned();
                let path = self.goomba_save_path();
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                self.write_project(&path);
                let wrote = self.project_path.as_ref() == Some(&path) && !self.dirty;
                if wrote {
                    if let Some(session) = self.extract.session.as_mut() {
                        if let Some(item) = session
                            .candidates
                            .iter_mut()
                            .find(|item| item.character_id == "goomba")
                        {
                            item.status = nesticles_core::pipeline::ReviewStatus::Approved;
                        }
                    }
                    self.studio.state.notice =
                        format!("{} Saved to {}.", published.message, path.display());
                } else {
                    self.studio.state.notice = format!(
                        "{} The project file was not written, so reopen will not find it yet.",
                        published.message
                    );
                }
                self.status = self.studio.state.notice.clone();
            }
            Err(err) => {
                self.studio.state.notice = err;
                self.status = self.studio.state.notice.clone();
            }
        }
    }

    fn goomba_save_path(&self) -> std::path::PathBuf {
        if let Some(path) = &self.project_path {
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            {
                return path.clone();
            }
        }
        directories::ProjectDirs::from("dev", "Nesticles", "Nesticles")
            .map(|dirs| dirs.data_dir().join("creations").join("goomba.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("goomba.json"))
    }

    pub(crate) fn present_saved_goomba(&mut self) {
        let Some(sprite) = self
            .project
            .sprites
            .iter()
            .find(|sprite| {
                sprite
                    .tags
                    .iter()
                    .any(|tag| tag == nesticles_core::reconstruct::TAG)
            })
            .map(|sprite| sprite.id.clone())
        else {
            return;
        };
        let Some(set) = self
            .project
            .anim_sets
            .iter()
            .find(|set| set.sprite_id == sprite)
        else {
            return;
        };
        let set_id = set.id.clone();
        let clip_id = set
            .clips
            .iter()
            .find(|clip| clip.name == "walk")
            .or_else(|| set.clips.first())
            .map(|clip| clip.id.clone());
        self.workspace = studio::Workspace::Unmaker;
        self.studio.state.stage = Stage::Create;
        self.studio.state.create_animate = true;
        self.studio.state.tool = None;
        self.sprite_id = Some(sprite);
        self.set_id = Some(set_id);
        self.clip_id = clip_id;
        self.status =
            "Opened the saved Goomba. No cartridge is loaded. Press Play to watch the walk.".into();
    }

    pub(crate) fn apply_r2_hooks(&mut self) {
        if self.studio.r2_env_applied {
            return;
        }
        self.studio.r2_env_applied = true;
        if let Ok(pose) = std::env::var("NESTICLES_R2_POSE") {
            if pose == "walk" || pose == "defeat" {
                self.studio.state.preview_pose = pose;
            }
        }
        if let Ok(frame) = std::env::var("NESTICLES_R2_FRAME") {
            if let Ok(index) = frame.parse::<u32>() {
                self.studio.state.preview_elapsed = index.saturating_mul(8);
                self.preview_clock.elapsed = index.saturating_mul(8);
                self.anim_playing = false;
            }
        }
        if std::env::var("NESTICLES_R2_PLAY").ok().as_deref() == Some("1") {
            self.studio.state.preview_playing = true;
            self.anim_playing = true;
        }
        if std::env::var("NESTICLES_R2_PUBLISH").ok().as_deref() == Some("1") {
            self.publish_selected_goomba();
        }
    }

    fn draw_evidence(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, tile: u32) {
        let zoom = self.studio.state.zoom;
        let side = 8.0 * zoom;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(side.max(128.0)), Sense::hover());
        checker(ui.painter(), rect);
        let Some(texture) = self.evidence_texture(ctx, tile) else {
            ui.label("That tile could not be decoded from the open cartridge.");
            return;
        };
        let image_rect = Rect::from_center_size(rect.center(), Vec2::splat(side));
        ui.painter().image(
            texture.id(),
            image_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    fn thumb(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, tile: Option<u32>) {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
        ui.painter().rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0_f32, studio::unmaker_line()),
            egui::StrokeKind::Inside,
        );
        let Some(tile) = tile else {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No art",
                egui::FontId::proportional(12.0),
                Color32::from_rgb(90, 90, 90),
            );
            return;
        };
        if let Some(texture) = self.evidence_texture(ctx, tile) {
            ui.painter().image(
                texture.id(),
                Rect::from_center_size(rect.center(), Vec2::splat(40.0)),
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    fn review_palette_bar(&mut self, ui: &mut egui::Ui) {
        ui.label(ReviewPalette::NOTE);
        ui.horizontal_wrapped(|ui| {
            for palette in ReviewPalette::ALL {
                let selected = self.studio.state.review_palette == palette;
                if ui.selectable_label(selected, palette.label()).clicked() && !selected {
                    self.studio.state.review_palette = palette;
                    self.studio.tiles.clear();
                    self.studio.assembled.clear();
                    self.studio.painted = palette;
                    self.studio.state.notice =
                        format!("Showing {}. {}", palette.label(), ReviewPalette::NOTE);
                }
            }
        });
    }

    fn evidence_texture(&mut self, ctx: &egui::Context, tile: u32) -> Option<TextureHandle> {
        if let Some(texture) = self.studio.tiles.get(&tile) {
            return Some(texture.clone());
        }
        let rom = self.rom.as_ref()?;
        let offset = rom.info.chr_start + tile as usize * 16;
        let indices = decode_tile(&rom.bytes, offset, Codec::Nes)?;
        if self.studio.painted != self.studio.state.review_palette {
            self.studio.tiles.clear();
            self.studio.painted = self.studio.state.review_palette;
        }
        let (w, h, rgba) = review_tile_rgba(&indices, self.studio.state.review_palette);
        let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let texture = ctx.load_texture(
            format!("evidence-{:?}-{tile}", self.studio.state.review_palette),
            image,
            TextureOptions::NEAREST,
        );
        self.studio.tiles.insert(tile, texture.clone());
        Some(texture)
    }

    fn identity_card(&mut self, ui: &mut egui::Ui) {
        let Some(rom) = self.rom.as_ref() else {
            ui.label("No cartridge is open.");
            return;
        };
        let Ok(report) = inspect_nes(&rom.bytes, &rom.name) else {
            ui.label("This file could not be identified.");
            return;
        };
        ui.label(format!(
            "Mapper {} · {} · CHR {} bytes",
            report.mapper, report.mirroring, report.chr_bytes
        ));
        if report.payload_sha256 == SMB1_PAYLOAD_SHA256 {
            ui.label(
                "Payload matches the known SMB1 profile. Names from that profile are proposals.",
            );
        } else if report.supported {
            ui.label("No SMB1 profile applies to this payload. Names will not be invented.");
        } else {
            ui.colored_label(
                studio::unmaker_red(),
                "This file is not a supported cartridge. Nothing will be reconstructed.",
            );
        }
        if let Some(warning) = report.warnings.first() {
            ui.label(warning);
        }
    }

    fn technical_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Technical details").size(18.0));
        ui.label("These values do not mean a sprite was finished.");
        if let Some(rom) = &self.rom {
            if let Ok(report) = inspect_nes(&rom.bytes, &rom.name) {
                ui.label(format!("File SHA-256: {}", report.sha256));
                ui.label(format!("Payload SHA-256: {}", report.payload_sha256));
                ui.label(format!(
                    "NES 2.0: {} · trainer: {}",
                    report.nes2, report.trainer
                ));
                ui.label(format!("CHR start: {}", rom.info.chr_start));
            }
        }
        if let Some(session) = &self.extract.session {
            ui.label(format!("Profile: {}", session.report.profile_label));
            ui.label(format!("Raw tiles: {}", session.tiles.len()));
            if let Some(id) = &self.studio.state.selected {
                if let Some(candidate) = session
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.id == id)
                {
                    ui.label(format!("Candidate id: {}", candidate.id));
                    ui.label(format!("Tiles: {:?}", candidate.tile_ids()));
                }
            }
        }
        ui.label("Session storage: memory only, until a sprite is written into the project.");
        if ui
            .add(egui::Button::new(RichText::new("Load synthetic fixture").size(16.0)).corner_radius(CornerRadius::same(19)).min_size(Vec2::new(112.0, 43.0)))
            .on_hover_text("A tiny local test cartridge. It is not Super Mario Bros.")
            .clicked()
        {
            let bytes = nesticles_core::pipeline::synthetic_fixture();
            self.load_rom_bytes("synthetic-fixture.nes", bytes);
            self.studio.state.notice = "Synthetic fixture only. It is not Super Mario Bros.".into();
        }
    }

    fn choose_studio_rom(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Cartridge",
                &["nes", "gb", "gbc", "gba", "sfc", "smc", "bin", "rom"],
            )
            .pick_file()
        else {
            self.studio.state.notice = "No cartridge was chosen.".into();
            return;
        };
        match std::fs::read(&path) {
            Ok(bytes) => {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("cartridge")
                    .to_string();
                self.open_rom_for_unmaker(name, bytes);
            }
            Err(err) => {
                self.studio.state.notice = format!("Could not read that cartridge: {err}");
            }
        }
    }

    pub(crate) fn open_rom_for_unmaker(&mut self, name: String, bytes: Vec<u8>) {
        if self.store_rom(name, bytes).is_none() {
            self.studio.state.notice = self.status.clone();
            return;
        }
        self.studio.state.stage = Stage::Import;
        self.studio.state.investigated = false;
        self.studio.state.stage_log.clear();
        self.studio.state.selected = None;
        self.studio.state.notice =
            "Cartridge opened. The file name is not its identity. Investigation has not started."
                .into();
        self.status = self.studio.state.notice.clone();
    }

    fn start_investigation(&mut self) {
        if self.rom.is_none() {
            self.studio.state.notice = "Open a cartridge before starting an investigation.".into();
            return;
        }
        self.studio.state.stage_log.clear();
        self.studio.state.investigated = false;
        self.studio.state.selected = None;
        self.begin_extract_job();
        self.studio.state.stage = Stage::Investigate;
        self.studio.state.notice =
            "Investigation started. The reference URL was not fetched.".into();
    }

    fn mark_reviewed(&mut self, issue: &IssueCard) {
        let Some(id) = &issue.candidate_id else {
            let feedback = approve_feedback(&issue.name, Err("There is nothing to approve."));
            self.studio.state.notice = feedback.message;
            return;
        };
        let Some(session) = self.extract.session.as_mut() else {
            return;
        };
        let Some(candidate) = session
            .candidates
            .iter_mut()
            .find(|candidate| &candidate.id == id)
        else {
            return;
        };
        let name = candidate.name.clone();
        let result = approve_candidate(candidate).map_err(|err| err);
        let message = match &result {
            Ok(()) => approve_feedback(&name, Ok(())).message,
            Err(err) => approve_feedback(&name, Err(err)).message,
        };
        self.studio.state.notice = message;
        self.status = self.studio.state.notice.clone();
    }

    fn fail_issue(&mut self, issue: &IssueCard) {
        let Some(id) = &issue.candidate_id else {
            self.studio.state.notice = format!("{} has nothing to reinspect.", issue.name);
            return;
        };
        if let Some(session) = self.extract.session.as_mut() {
            if let Some(candidate) = session
                .candidates
                .iter_mut()
                .find(|candidate| &candidate.id == id)
            {
                fail_candidate(
                    candidate,
                    FailReason::MissingFrames,
                    "Artwork has not been reconstructed.",
                );
            }
        }
        self.studio.state.notice = format!(
            "Marked {} as needing reinspection because its artwork is not reconstructed. Nothing was published.",
            issue.name
        );
    }

    fn restore_issue(&mut self, issue: &IssueCard) {
        let Some(id) = &issue.candidate_id else {
            return;
        };
        if let Some(session) = self.extract.session.as_mut() {
            if let Some(candidate) = session
                .candidates
                .iter_mut()
                .find(|candidate| &candidate.id == id)
            {
                restore_candidate(candidate);
            }
        }
        self.studio.state.notice = restore_feedback(&issue.name).message;
    }

    fn ignore_issue(&mut self, issue: &IssueCard) {
        let Some(id) = &issue.candidate_id else {
            self.studio.state.notice = format!(
                "{} has no candidate to ignore. Nothing was deleted.",
                issue.name
            );
            return;
        };
        if let Some(session) = self.extract.session.as_mut() {
            if let Some(candidate) = session
                .candidates
                .iter_mut()
                .find(|candidate| &candidate.id == id)
            {
                ignore_candidate(candidate);
            }
        }
        self.studio.state.notice = ignore_feedback(&issue.name).message;
    }

    fn reinspect_same(&mut self, issue: &IssueCard) {
        let Some(id) = &issue.candidate_id else {
            return;
        };
        let Some(session) = self.extract.session.as_mut() else {
            return;
        };
        let Some(candidate) = session
            .candidates
            .iter_mut()
            .find(|candidate| &candidate.id == id)
        else {
            return;
        };
        let tiles = candidate.tile_ids();
        self.studio.state.notice = match reinspect(candidate, &tiles) {
            Ok(message) => message,
            Err(err) => err,
        };
    }

    fn open_manual(&mut self, issue: &IssueCard) {
        let Some(id) = &issue.candidate_id else {
            self.studio.state.notice = "There is no candidate to edit by hand.".into();
            return;
        };
        let Some(session) = &self.extract.session else {
            return;
        };
        let Some(candidate) = session
            .candidates
            .iter()
            .find(|candidate| &candidate.id == id)
        else {
            return;
        };
        let count = session.tiles.len() as u32;
        self.extract.puzzle = Some(puzzle_from(candidate, count));
        self.studio.state.show_manual = true;
        self.studio.state.notice =
            "Manual placement is open. No automatic match was applied.".into();
    }

    pub(crate) fn apply_r1_tour(&mut self, ctx: &egui::Context) {
        let _ = ctx;
        let Ok(tour) = std::env::var("NESTICLES_R1_TOUR") else {
            return;
        };
        if tour == "maker" {
            self.workspace = Workspace::Maker;
            return;
        }
        self.workspace = Workspace::Unmaker;
        self.studio.state.stage = Stage::Home;
        if let Ok(name) = std::env::var("NESTICLES_REVIEW_PALETTE") {
            if let Some(palette) = ReviewPalette::parse(&name) {
                self.studio.state.review_palette = palette;
                self.studio.tiles.clear();
            }
        }
        if tour == "reference" {
            if let Ok(url) = std::env::var("NESTICLES_REFERENCE_URL") {
                self.extract.ref_url = url;
            }
            if let Some(path) = std::env::var("NESTICLES_ROM").ok() {
                if let Ok(bytes) = std::fs::read(&path) {
                    let name = std::path::Path::new(&path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("cartridge")
                        .to_string();
                    self.open_rom_for_unmaker(name, bytes);
                }
            }
            if !self.extract.ref_url.trim().is_empty() {
                self.start_reference_fetch();
            }
            self.studio.state.stage = Stage::Import;
            return;
        }
        if tour == "home" {
            return;
        }
        let Some(path) = std::env::var("NESTICLES_ROM").ok() else {
            self.studio.state.notice =
                "NESTICLES_ROM was not set. No cartridge was invented.".into();
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            self.studio.state.notice = format!("Could not read {path}. No cartridge was invented.");
            return;
        };
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("cartridge")
            .to_string();
        self.open_rom_for_unmaker(name, bytes);
        if tour == "import" {
            return;
        }
        if tour == "investigate" {
            self.start_investigation();
            self.pump_extract();
            if let Some(job) = self.extract.job.as_mut() {
                job.paused = true;
            }
            self.studio.state.stage = Stage::Investigate;
            return;
        }
        self.finish_investigation();
        if tour == "gallery" {
            self.studio.state.stage = Stage::Discover;
            return;
        }
        self.select_named("Goomba");
        if tour == "goomba" || tour == "preview" {
            self.studio.state.stage = Stage::Discover;
            self.studio.state.zoom = 12.0;
            return;
        }
        if tour == "review" {
            if let Some(session) = &self.extract.session {
                let issues = issue_queue(&session.candidates, &session.coverage);
                if let Some(index) = issues.iter().position(|issue| issue.name == "Goomba") {
                    self.studio.state.issue_index = index;
                }
            }
            self.studio.state.stage = Stage::Review;
        }
    }

    fn finish_investigation(&mut self) {
        self.start_investigation();
        for _ in 0..8 {
            if self.extract.job.is_none() {
                break;
            }
            self.pump_extract();
        }
    }

    fn select_named(&mut self, name: &str) {
        let Some(session) = &self.extract.session else {
            return;
        };
        if let Some(candidate) = session
            .candidates
            .iter()
            .find(|candidate| candidate.name == name)
        {
            self.studio.state.selected = Some(candidate.id.clone());
            self.studio.state.evidence_index = 0;
        }
    }
}

fn stage_tip(stage: Stage) -> &'static str {
    match stage {
        Stage::Home => "Choose how to start",
        Stage::Import => "Open a cartridge",
        Stage::Investigate => "Watch the local investigation",
        Stage::Discover => "Browse what was identified",
        Stage::Review => "Handle one unresolved issue",
        Stage::Create => "Author an original sprite",
        Stage::Save => "Save the project file",
    }
}

fn paint_stage_icon(painter: &egui::Painter, origin: egui::Pos2, stage: Stage) {
    let stroke = Stroke::new(1.5_f32, studio::unmaker_ink());
    let rect = Rect::from_min_size(origin, Vec2::splat(16.0));
    match stage {
        Stage::Home => {
            painter.line_segment([rect.center_top(), rect.left_bottom()], stroke);
            painter.line_segment([rect.center_top(), rect.right_bottom()], stroke);
        }
        Stage::Import => {
            painter.rect_stroke(rect, 1.0, stroke, egui::StrokeKind::Inside);
        }
        Stage::Investigate => {
            painter.circle_stroke(rect.center(), 7.0, stroke);
        }
        Stage::Discover => {
            painter.rect_stroke(rect, 1.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment([rect.center_top(), rect.center_bottom()], stroke);
            painter.line_segment([rect.left_center(), rect.right_center()], stroke);
        }
        Stage::Review => {
            painter.line_segment(
                [rect.left_center(), rect.center_bottom()],
                Stroke::new(1.5_f32, studio::unmaker_red()),
            );
            painter.line_segment(
                [rect.center_bottom(), rect.right_top()],
                Stroke::new(1.5_f32, studio::unmaker_red()),
            );
        }
        Stage::Create => {
            painter.line_segment([rect.left_bottom(), rect.right_top()], stroke);
        }
        Stage::Save => {
            painter.rect_stroke(rect, 1.0, stroke, egui::StrokeKind::Inside);
            painter.rect_filled(
                Rect::from_min_size(rect.min + Vec2::new(4.0, 2.0), Vec2::new(8.0, 4.0)),
                0.0,
                studio::unmaker_red(),
            );
        }
    }
}

fn checker(painter: &egui::Painter, rect: Rect) {
    let cell = 8.0;
    let mut y = rect.top();
    let mut row = 0;
    while y < rect.bottom() {
        let mut x = rect.left();
        let mut col = 0;
        while x < rect.right() {
            let color = if (row + col) % 2 == 0 {
                Color32::from_rgb(230, 230, 230)
            } else {
                Color32::WHITE
            };
            painter.rect_filled(
                Rect::from_min_size(egui::pos2(x, y), Vec2::splat(cell)),
                0.0,
                color,
            );
            x += cell;
            col += 1;
        }
        y += cell;
        row += 1;
    }
}

impl NestApp {
    fn primary<'a>(&self, label: &'a str) -> egui::Button<'a> {
        design::primary_widget(label)
    }
}
