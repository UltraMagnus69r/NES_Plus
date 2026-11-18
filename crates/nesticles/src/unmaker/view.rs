//! UnMaker Lab screens. Maker Engine does not use this module.
//!
//! Source tiles drawn here are evidence from CHR. They are not a reconstructed
//! character, and the preview does not play them as an animation.

use super::model::{
    approve_feedback, continue_available, gallery_items, ignore_feedback, issue_queue,
    preview_policy, reconstructed_frame_count, restore_feedback, review_tile_rgba, skip_feedback,
    Group, IssueCard, ReviewPalette, Stage, StudioState, NOT_RECONSTRUCTED,
};
use crate::app::NestApp;
use crate::studio::{self, Workspace};
use eframe::egui::{
    self, Color32, Rect, RichText, Sense, Stroke, TextureHandle, TextureOptions, Vec2,
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
}

impl Default for StudioUi {
    fn default() -> Self {
        Self {
            state: StudioState::default(),
            tiles: HashMap::new(),
            painted: ReviewPalette::default(),
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
            .frame(egui::Frame::NONE.fill(studio::unmaker_paper()).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let flip = egui::Button::new(
                        RichText::new("Maker Engine")
                            .size(16.0)
                            .color(studio::unmaker_ink()),
                    )
                    .fill(studio::unmaker_paper())
                    .stroke(Stroke::new(1.0_f32, studio::unmaker_line()))
                    .min_size(Vec2::new(148.0, 36.0));
                    if ui
                        .add(flip)
                        .on_hover_text("Flip back to Maker Engine (F6)")
                        .clicked()
                    {
                        self.flip_workspace();
                    }
                    ui.label(
                        RichText::new("UnMaker Lab")
                            .size(20.0)
                            .color(studio::unmaker_ink()),
                    );
                    ui.label(
                        RichText::new(self.studio.state.stage.label())
                            .size(16.0)
                            .color(studio::unmaker_red()),
                    );
                    if ui
                        .add(self.bar_button(
                            if self.studio.state.legacy_tools {
                                "Studio"
                            } else {
                                "Source tools"
                            },
                            false,
                        ))
                        .on_hover_text(
                            "Source tools are the earlier tile page, audio, library, and export. They are not the reconstruction studio.",
                        )
                        .clicked()
                    {
                        self.studio.state.legacy_tools = !self.studio.state.legacy_tools;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(self.bar_button("Exit", true))
                            .on_hover_text("Exit to Desktop")
                            .clicked()
                        {
                            self.request_exit(ctx);
                        }
                        if ui
                            .add(self.bar_button("Menu", false))
                            .on_hover_text("Open the application menu")
                            .clicked()
                        {
                            self.app_menu = !self.app_menu;
                        }
                        let save = if self.dirty { "Save ●" } else { "Save" };
                        if ui
                            .add(self.bar_button(save, false))
                            .on_hover_text(
                                "Save the project file. This does not publish an extracted sprite.",
                            )
                            .clicked()
                        {
                            self.save_project_quick();
                            self.studio.state.notice = if self.dirty {
                                "The project save did not finish. Nothing was published.".into()
                            } else {
                                "Project file saved. The extraction session is still only in memory."
                                    .into()
                            };
                        }
                    });
                });
            });
    }

    fn bar_button(&self, label: &str, danger: bool) -> egui::Button<'_> {
        let color = if danger {
            Color32::WHITE
        } else {
            studio::unmaker_ink()
        };
        let fill = if danger {
            studio::unmaker_red()
        } else {
            studio::unmaker_paper()
        };
        egui::Button::new(RichText::new(label).size(16.0).color(color))
            .fill(fill)
            .stroke(Stroke::new(1.0_f32, studio::unmaker_line()))
            .min_size(Vec2::new(84.0, 36.0))
    }

    pub(crate) fn unmaker_studio(&mut self, ctx: &egui::Context) {
        if self.studio.state.legacy_tools {
            self.unmaker_shell(ctx);
            return;
        }
        egui::TopBottomPanel::bottom("unmaker-foot")
            .exact_height(40.0)
            .frame(
                egui::Frame::NONE
                    .fill(studio::unmaker_mist())
                    .inner_margin(8.0),
            )
            .show(ctx, |ui| {
                ui.label(
                    "Approval does not publish a sprite. Mario and Luigi have no tile mapping. Palette and timing are unverified.",
                );
            });
        egui::SidePanel::left("unmaker-nav")
            .exact_width(188.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(studio::unmaker_paper())
                    .inner_margin(8.0)
                    .stroke(Stroke::new(1.0_f32, studio::unmaker_line())),
            )
            .show(ctx, |ui| {
                if self.stage_button(ui, Stage::Home).clicked() {
                    self.studio.state.stage = Stage::Home;
                    self.studio.state.show_manual = false;
                }
                ui.add_space(8.0);
                for stage in Stage::journey() {
                    if self.stage_button(ui, stage).clicked() {
                        self.studio.state.stage = stage;
                        self.studio.state.show_manual = false;
                    }
                }
                ui.add_space(12.0);
                let details = self.studio.state.show_technical;
                if ui
                    .selectable_label(details, "Technical details")
                    .on_hover_text("Show hashes, indexes, and other diagnostics")
                    .clicked()
                {
                    self.studio.state.show_technical = !details;
                }
            });
        if self.studio.state.show_technical {
            egui::SidePanel::right("unmaker-tech")
                .exact_width(280.0)
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
                    .fill(studio::unmaker_paper())
                    .inner_margin(16.0),
            )
            .show(ctx, |ui| {
                if !self.studio.state.notice.is_empty() {
                    egui::Frame::NONE
                        .fill(studio::unmaker_mist())
                        .inner_margin(8.0)
                        .stroke(Stroke::new(3.0_f32, studio::unmaker_red()))
                        .show(ui, |ui| {
                            ui.label(RichText::new(&self.studio.state.notice).size(16.0));
                        });
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.studio.state.stage {
                        Stage::Home => self.home_stage(ui),
                        Stage::Import => self.import_stage(ui),
                        Stage::Investigate => self.investigate_stage(ui),
                        Stage::Discover => self.discover_stage(ui, ctx),
                        Stage::Review => self.review_stage(ui, ctx),
                        Stage::Create => self.create_stage(ui, ctx),
                        Stage::Save => self.save_stage(ui),
                    });
            });
    }

    fn stage_button(&self, ui: &mut egui::Ui, stage: Stage) -> egui::Response {
        let active = self.studio.state.stage == stage;
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 40.0), Sense::click());
        let fill = if active {
            Color32::from_rgb(255, 236, 236)
        } else if response.hovered() {
            studio::unmaker_mist()
        } else {
            studio::unmaker_paper()
        };
        ui.painter().rect_filled(rect, 4.0, fill);
        if active {
            ui.painter().rect_filled(
                Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
                0.0,
                studio::unmaker_red(),
            );
        }
        let icon = rect.min + Vec2::new(12.0, 12.0);
        paint_stage_icon(ui.painter(), icon, stage);
        ui.painter().text(
            rect.min + Vec2::new(36.0, 20.0),
            egui::Align2::LEFT_CENTER,
            stage.label(),
            egui::FontId::proportional(16.0),
            studio::unmaker_ink(),
        );
        response.on_hover_text(stage_tip(stage))
    }

    fn home_stage(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("What do you want to work on?").size(28.0));
        ui.label(
            "UnMaker investigates a cartridge, then shows what it could and could not identify.",
        );
        ui.add_space(16.0);
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
            Color32::from_rgb(255, 236, 236)
        } else {
            studio::unmaker_paper()
        };
        ui.painter().rect(
            rect,
            6.0,
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
        ui.label(RichText::new("Import").size(28.0));
        ui.label(
            "Open a cartridge. A reference page is optional and is not fetched until you ask.",
        );
        ui.add_space(8.0);
        if ui
            .add(self.primary("Open ROM"))
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
            if ui
                .button("Fetch reference")
                .on_hover_text("Download that HTTPS page. This does not send the ROM anywhere.")
                .clicked()
            {
                self.start_reference_fetch();
                self.studio.state.notice =
                    "Reference fetch started. It is not part of ROM identity.".into();
            }
            if ui
                .button("Start Investigation")
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
    }

    fn investigate_stage(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Investigation").size(28.0));
        let running = self
            .extract
            .job
            .as_ref()
            .map(|job| (job.stage_name.clone(), job.error.clone()));
        let mut cancel = false;
        if let Some((stage_name, error)) = &running {
            ui.label(format!("Current stage: {stage_name}"));
            if !error.is_empty() {
                ui.colored_label(studio::unmaker_red(), error);
            }
            if ui
                .button("Cancel")
                .on_hover_text("Stop this pass. Saved files stay where they are.")
                .clicked()
            {
                cancel = true;
            }
        } else if self.extract.session.is_some() {
            ui.label("Current stage: Ready for review");
        } else {
            ui.label("No investigation is running.");
            if ui.button("Go to Import").clicked() {
                self.studio.state.stage = Stage::Import;
            }
        }
        if cancel {
            if let Some(job) = self.extract.job.as_mut() {
                job_cancel(job);
                self.studio.state.notice = job.stage_name.clone();
            }
        }
        ui.add_space(8.0);
        ui.label("Stages reached");
        if self.studio.state.stage_log.is_empty() {
            ui.label("Nothing has been recorded yet.");
        }
        for (index, name) in self.studio.state.stage_log.iter().enumerate() {
            ui.label(format!("{}. {name}", index + 1));
        }
        ui.add_space(8.0);
        if let Some(session) = &self.extract.session {
            let unresolved = session
                .coverage
                .iter()
                .filter(|row| row.assembly == "unresolved")
                .count();
            ui.label(format!(
                "Candidates identified: {}",
                session.candidates.len()
            ));
            ui.label("Assets with reconstructed frames: 0");
            ui.label(format!("Unresolved coverage rows: {unresolved}"));
            ui.label("Reconstructed frames stay at 0 until a later step assembles them.");
            if ui.button("Open the gallery").clicked() {
                self.studio.state.stage = Stage::Discover;
            }
        } else if self.extract.job.is_some() {
            ui.label("Counts appear when this pass finishes.");
        }
    }

    fn discover_stage(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(RichText::new("Discover").size(28.0));
        let Some(session) = self.extract.session.clone() else {
            ui.label("There is no investigation to show. Start one from Import.");
            if ui.button("Go to Import").clicked() {
                self.studio.state.stage = Stage::Import;
            }
            return;
        };
        let items = gallery_items(&session.candidates, &session.coverage);
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(self.studio.state.filter.is_none(), "All")
                .clicked()
            {
                self.studio.state.filter = None;
            }
            for group in Group::all() {
                let on = self.studio.state.filter == Some(group);
                if ui.selectable_label(on, group.label()).clicked() {
                    self.studio.state.filter = Some(group);
                }
            }
        });
        ui.add_space(8.0);
        let filter = self.studio.state.filter;
        let shown: Vec<_> = items
            .iter()
            .filter(|item| filter.is_none_or(|group| item.group == group))
            .cloned()
            .collect();
        ui.columns(2, |columns| {
            columns[0].label("Candidates");
            egui::ScrollArea::vertical()
                .id_salt("gallery-cards")
                .max_height(520.0)
                .show(&mut columns[0], |ui| {
                    for item in &shown {
                        let selected =
                            self.studio.state.selected.as_deref() == Some(item.id.as_str());
                        let frame = egui::Frame::NONE
                            .fill(if selected {
                                Color32::from_rgb(255, 236, 236)
                            } else {
                                studio::unmaker_paper()
                            })
                            .stroke(Stroke::new(1.0_f32, studio::unmaker_line()))
                            .inner_margin(8.0);
                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                self.thumb(ui, ctx, item.tile_ids.first().copied());
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&item.name).size(18.0));
                                    ui.label(format!("{} · {}", item.group.label(), item.status));
                                    if item.frame_labels.is_empty() {
                                        ui.label("No proposed form");
                                    } else {
                                        ui.label(format!(
                                            "Proposed labels: {}",
                                            item.frame_labels.join(", ")
                                        ));
                                    }
                                    ui.label(if item.needs_user {
                                        "Needs a person"
                                    } else {
                                        "Reviewed, not published"
                                    });
                                    if !item.reconstructed {
                                        ui.label("Artwork not reconstructed");
                                    }
                                });
                            });
                            if ui.button("Look at this").clicked() {
                                self.studio.state.selected = Some(item.id.clone());
                                self.studio.state.evidence_index = 0;
                            }
                        });
                        ui.add_space(6.0);
                    }
                });
            let selected = items
                .iter()
                .find(|item| self.studio.state.selected.as_deref() == Some(item.id.as_str()));
            self.preview_column(&mut columns[1], ctx, selected, &session.candidates);
        });
        if ui
            .button("Review the open issue for this candidate")
            .clicked()
        {
            if let Some(id) = self.studio.state.selected.clone() {
                let issues = issue_queue(&session.candidates, &session.coverage);
                if let Some(index) = issues.iter().position(|issue| {
                    issue.candidate_id.as_deref() == Some(id.as_str())
                        || items
                            .iter()
                            .find(|item| item.id == id)
                            .is_some_and(|item| item.name == issue.name)
                }) {
                    self.studio.state.issue_index = index;
                    self.studio.state.stage = Stage::Review;
                }
            }
        }
    }

    fn review_stage(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(RichText::new("Review one issue").size(28.0));
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
        ui.label(format!("Issue {} of {}", index + 1, issues.len()));
        ui.label(RichText::new(&issue.name).size(22.0));
        ui.label(&issue.uncertainty);
        ui.label(&issue.evidence);
        ui.label(&issue.attempted);
        ui.label(&issue.next_action);
        ui.label("Suggested fix: none. No automatic correction engine is running.");
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
            if ui.button("Skip").on_hover_text("Leave this issue unchanged.").clicked() {
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
            .button("Mark candidate reviewed")
            .on_hover_text("Changes the review flag only. It does not publish a sprite.")
            .clicked()
        {
            self.mark_reviewed(&issue);
        }
        if ui.button("Return this candidate to the gallery").clicked() {
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
        ui.label(RichText::new("Create").size(28.0));
        ui.label("Original sprites live in the project. They are separate from ROM candidates.");
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
        } else {
            self.assemble_view(ui, ctx);
        }
    }

    fn save_stage(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Save").size(28.0));
        ui.label(
            "Save writes the project file. It does not store the extraction session, the ROM, or a published Goomba animation.",
        );
        ui.label(
            "Package export still paints only the first frame and is not offered here as a finished asset.",
        );
        if ui.button("Save project").clicked() {
            self.save_project_quick();
            self.studio.state.notice = if self.dirty {
                "The project save did not finish.".into()
            } else {
                "Project file saved. No extracted character was published.".into()
            };
        }
        ui.add_space(8.0);
        ui.label("Still true after this save:");
        ui.label("• approve_candidate changes a review flag and does not create a sprite.");
        ui.label("• A package would bake the first frame only, and every clip would share it.");
        ui.label("• Mario and Luigi have no reconstruction.");
        ui.label("• Reference pages are not matched to ROM tiles.");
        ui.label("• Palettes and animation timings are not verified.");
        ui.label("• Live OpenAI has not been accepted. This screen does not call it.");
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
        let frames = candidates
            .iter()
            .find(|candidate| candidate.id == item.id)
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
            if ui.button("Zoom in").clicked() {
                self.studio.state.zoom = (self.studio.state.zoom + 1.0).min(16.0);
            }
            if ui.button("Zoom out").clicked() {
                self.studio.state.zoom = (self.studio.state.zoom - 1.0).max(2.0);
            }
            let _ = ui
                .add_enabled(policy.can_play, egui::Button::new("Play"))
                .on_disabled_hover_text("There are no reconstructed frames to play.");
        });
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
    fn primary(&self, label: &str) -> egui::Button<'_> {
        egui::Button::new(RichText::new(label).size(18.0).color(studio::unmaker_ink()))
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.5_f32, studio::unmaker_red()))
            .min_size(Vec2::new(180.0, 40.0))
    }
}
