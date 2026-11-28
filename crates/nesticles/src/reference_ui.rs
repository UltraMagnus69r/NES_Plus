//! R4 reference workspace: side-by-side compare, association, and opt-in AI.
//! Drawing only. Persistence and compare math live in nesticles_core::ref_intel.

use super::NestApp;
use crate::design;
use crate::openai;
use crate::responsive::ResponsiveLayout;
use eframe::egui::{self, Color32, RichText, Sense, TextureOptions, Vec2};
use nesticles_core::ref_intel::{
    self, describe_ai_payload, gate_ai_send, load_reference_rgba, mark_active, refs_for,
    remove_association, AiAssistRecord, AiSendRequest, ObservationKind, ReferenceConfidence,
};
impl NestApp {
    pub(crate) fn open_reference_workspace(&mut self) {
        if self.studio.state.selected.is_none() {
            self.studio.state.notice =
                "Select a Discover or Review card before opening its reference workspace.".into();
            return;
        }
        self.studio.state.tool = Some(super::unmaker::StudioTool::Reference);
        // Keep the notice short so compact laptop clients still see both images.
        self.studio.state.notice =
            "AI stays off until a credential and an explicit send.".into();
    }

    pub(crate) fn reference_workspace_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let layout = ResponsiveLayout::from_context(ctx);
        let compact = matches!(layout.density(), crate::responsive::LayoutMode::Compact);
        let asset_id = self
            .studio
            .state
            .selected
            .clone()
            .unwrap_or_else(|| "(none)".into());
        let asset_name = self.reference_asset_label(&asset_id);

        if compact {
            ui.label(
                RichText::new(format!("{asset_name} · {asset_id}"))
                    .size(14.0)
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new(format!("Reference for {asset_name}"))
                    .size(22.0)
                    .strong(),
            );
            ui.label(format!("Stable asset id: {asset_id}"));
            ui.label(
                "A reference is evidence for you to examine. It is not an automatically approved reconstruction.",
            );
            ui.add_space(6.0);
        }

        self.reference_import_row(ui, &asset_id);

        // Comparison early so laptop and compact clients see both images.
        ui.horizontal_wrapped(|ui| {
            ui.label("Zoom");
            let mut zoom = self.extract.ref_compare_zoom;
            if ui
                .add(egui::Slider::new(&mut zoom, 1.0..=24.0).logarithmic(true))
                .changed()
            {
                self.extract.ref_compare_zoom = ref_intel::zoom_clamped(zoom);
            }
            if design::soft_button(ui, "Reset zoom").clicked() {
                self.extract.ref_compare_zoom = 8.0;
            }
        });
        let available = ui.available_width();
        let side_by_side = ref_intel::layout_side_by_side(available);
        if side_by_side {
            ui.columns(2, |cols| {
                cols[0].label(RichText::new("Reference").strong());
                self.paint_active_reference(&mut cols[0], ctx, &asset_id);
                cols[1].label(RichText::new("Reconstruction").strong());
                self.paint_reconstruction_panel(&mut cols[1], ctx, &asset_id);
            });
        } else {
            ui.label(
                RichText::new("Compact layout — panels stack so controls stay reachable.")
                    .size(12.0),
            );
            ui.columns(2, |cols| {
                cols[0].label(RichText::new("Reference").strong());
                self.paint_active_reference(&mut cols[0], ctx, &asset_id);
                cols[1].label(RichText::new("Reconstruction").strong());
                self.paint_reconstruction_panel(&mut cols[1], ctx, &asset_id);
            });
        }

        ui.separator();
        ui.label("Notes for this association");
        ui.add(
            egui::TextEdit::multiline(&mut self.extract.ref_workspace_notes)
                .desired_width(ui.available_width())
                .desired_rows(2),
        );
        self.reference_list_row(ui, &asset_id);
        ui.separator();
        self.reference_intelligence_panel(ui, &asset_id);
        ui.separator();
        self.reference_ai_panel(ui, &asset_id);
    }

    fn reference_asset_label(&self, asset_id: &str) -> String {
        if let Some(session) = &self.extract.session {
            if let Some(candidate) = session.candidates.iter().find(|c| c.id == asset_id) {
                return candidate.name.clone();
            }
        }
        if let Some(sprite) = self.project.sprites.iter().find(|s| s.id == asset_id) {
            return sprite.name.clone();
        }
        if let Some(asset) = self.project.assets.iter().find(|a| a.id == asset_id) {
            return asset.name.clone();
        }
        asset_id.to_string()
    }

    fn reference_import_row(&mut self, ui: &mut egui::Ui, asset_id: &str) {
        ui.horizontal_wrapped(|ui| {
            if design::soft_button(ui, "Import local image").clicked() {
                self.import_local_asset_reference(asset_id);
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.extract.ref_url).desired_width(280.0),
            );
            if design::soft_button(ui, "Add image URL").clicked() {
                self.start_direct_image_for_asset(asset_id);
            }
            if design::soft_button(ui, "Fetch page for gallery").clicked() {
                self.start_reference_fetch();
            }
        });
        if let Some(err) = &self.extract.ref_error {
            ui.label(RichText::new(err).color(Color32::from_rgb(160, 40, 40)));
        }
    }

    fn reference_list_row(&mut self, ui: &mut egui::Ui, asset_id: &str) {
        let items: Vec<_> = refs_for(&self.project.asset_references, asset_id)
            .into_iter()
            .cloned()
            .collect();
        if items.is_empty() {
            ui.label("No reference associated yet. Import a local image or a direct image URL.");
            return;
        }
        ui.label(format!("{} reference(s) for this asset", items.len()));
        let mut activate = None;
        let mut remove = None;
        let mut save_notes = None;
        for item in &items {
            ui.horizontal_wrapped(|ui| {
                let on = item.active
                    || ref_intel::active_for(&self.project.asset_references, asset_id)
                        .map(|a| a.id == item.id)
                        .unwrap_or(false);
                if ui.selectable_label(on, &item.title).clicked() {
                    activate = Some(item.id.clone());
                }
                ui.label(item.source_kind.label());
                ui.label(item.confidence.label());
                if !item.embedded {
                    ui.label("External file dependency");
                }
                if design::soft_button(ui, "Remove association").clicked() {
                    remove = Some(item.id.clone());
                }
            });
            ui.label(format!(
                "{}×{}  {}  {}",
                item.width,
                item.height,
                if item.source_url.is_empty() {
                    item.source_hint.as_str()
                } else {
                    item.source_url.as_str()
                },
                item.added_at
            ));
        }
        if design::soft_button(ui, "Save notes onto active reference").clicked() {
            save_notes = Some(true);
        }
        if let Some(id) = activate {
            mark_active(&mut self.project.asset_references, asset_id, &id);
            self.dirty = true;
            self.extract.ref_assoc_tex = None;
            self.studio.state.notice = "Active reference updated.".into();
        }
        if let Some(id) = remove {
            remove_association(&mut self.project.asset_references, &id);
            self.dirty = true;
            self.extract.ref_assoc_tex = None;
            self.studio.state.notice =
                "Association removed. Other project assets were not deleted.".into();
        }
        if save_notes == Some(true) {
            let notes = self.extract.ref_workspace_notes.clone();
            let idx = self
                .project
                .asset_references
                .iter()
                .position(|item| item.asset_id == asset_id && item.active)
                .or_else(|| {
                    self.project
                        .asset_references
                        .iter()
                        .position(|item| item.asset_id == asset_id)
                });
            if let Some(idx) = idx {
                self.project.asset_references[idx].notes = notes;
                self.dirty = true;
                self.studio.state.notice = "Reference notes saved on the association.".into();
            }
        }
    }

    fn paint_active_reference(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, asset_id: &str) {
        let Some(item) = ref_intel::active_for(&self.project.asset_references, asset_id).cloned()
        else {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width().min(320.0), 160.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, 12.0, Color32::from_rgb(245, 248, 252));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No reference selected",
                egui::FontId::proportional(14.0),
                Color32::from_rgb(90, 100, 120),
            );
            return;
        };
        match load_reference_rgba(&item) {
            Ok((w, h, rgba)) => {
                self.ensure_assoc_tex(ctx, &item.id, w, h, &rgba);
                let zoom = self.extract.ref_compare_zoom;
                if let Some(tex) = &self.extract.ref_assoc_tex {
                    let max_w = ui.available_width().min(360.0).max(64.0);
                    let max_h = 280.0;
                    let (dw, dh) = ResponsiveLayout::preview_fit(
                        max_w,
                        max_h,
                        (w as f32) * (zoom / 8.0).max(0.25),
                        (h as f32) * (zoom / 8.0).max(0.25),
                    );
                    // Nearest-neighbor for NES-like inspection.
                    ui.add(
                        egui::Image::from_texture(tex)
                            .fit_to_exact_size(Vec2::new(dw, dh))
                            .maintain_aspect_ratio(true),
                    );
                }
                ui.label(format!(
                    "{}  {}  {}×{}  {}",
                    item.title,
                    item.confidence.label(),
                    item.width,
                    item.height,
                    item.source_kind.label()
                ));
                if !item.notes.is_empty() {
                    ui.label(&item.notes);
                }
                if item.confidence == ReferenceConfidence::UserSupplied {
                    ui.label("User-supplied — not labeled as a verified original-game source.");
                }
            }
            Err(err) => {
                ui.label(RichText::new(err).color(Color32::from_rgb(160, 40, 40)));
            }
        }
    }

    fn paint_reconstruction_panel(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        asset_id: &str,
    ) {
        // Prefer direct RGBA paint so the compare pane works inside narrow columns.
        if let Some((w, h, rgba)) = self.reconstruction_rgba(asset_id) {
            let key = format!("{asset_id}-{w}x{h}");
            if self.extract.recon_compare_tex_id.as_deref() != Some(key.as_str())
                || self.extract.recon_compare_tex.is_none()
            {
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                self.extract.recon_compare_tex = Some(ctx.load_texture(
                    format!("r4-recon-{key}"),
                    image,
                    TextureOptions::NEAREST,
                ));
                self.extract.recon_compare_tex_id = Some(key);
            }
            let zoom = self.extract.ref_compare_zoom;
            let max_w = ui.available_width().min(360.0).max(64.0);
            let max_h = 280.0;
            let (dw, dh) = ResponsiveLayout::preview_fit(
                max_w,
                max_h,
                (w as f32) * (zoom / 8.0).max(0.25),
                (h as f32) * (zoom / 8.0).max(0.25),
            );
            if let Some(tex) = &self.extract.recon_compare_tex {
                ui.add(
                    egui::Image::from_texture(tex)
                        .fit_to_exact_size(Vec2::new(dw, dh))
                        .maintain_aspect_ratio(true),
                );
            }
            ui.label(format!("Reconstruction {w}×{h} (nearest-neighbor)"));
            design::status_chip(ui, "Review colors only — palette unverified");
            return;
        }
        if let Some(sprite) = self.project.sprites.iter().find(|s| s.id == asset_id) {
            ui.label(format!("Project sprite: {}", sprite.name));
            ui.label("Open Review or Discover for animated playback of this sprite.");
            return;
        }
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width().min(320.0), 160.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 12.0, Color32::from_rgb(245, 248, 252));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Unresolved — no reconstruction yet",
            egui::FontId::proportional(14.0),
            Color32::from_rgb(90, 100, 120),
        );
        ui.label("Mario, Luigi, and other unresolved rows stay placeholders until verified evidence exists.");
    }

    fn ensure_assoc_tex(
        &mut self,
        ctx: &egui::Context,
        id: &str,
        w: u32,
        h: u32,
        rgba: &[u8],
    ) {
        if self
            .extract
            .ref_assoc_tex_id
            .as_deref()
            .is_some_and(|existing| existing == id)
            && self.extract.ref_assoc_tex.is_some()
        {
            return;
        }
        let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba);
        self.extract.ref_assoc_tex = Some(ctx.load_texture(
            format!("r4-ref-{id}"),
            image,
            TextureOptions::NEAREST,
        ));
        self.extract.ref_assoc_tex_id = Some(id.into());
    }

    fn reference_intelligence_panel(&mut self, ui: &mut egui::Ui, asset_id: &str) {
        ui.label(RichText::new("Reference intelligence").size(18.0).strong());
        let Some(item) = ref_intel::active_for(&self.project.asset_references, asset_id).cloned()
        else {
            ui.label("Associate a reference to see deterministic comparison notes.");
            return;
        };
        let Ok((rw, rh, rrgba)) = load_reference_rgba(&item) else {
            ui.label("Reference image unavailable for comparison.");
            return;
        };
        let Some((cw, ch, crgba)) = self.reconstruction_rgba(asset_id) else {
            ui.label(
                "Reconstruction pixels are unavailable. Comparison waits until artwork exists.",
            );
            if !item.notes.is_empty() {
                ui.label(format!("Owner notes: {}", item.notes));
            }
            return;
        };
        let notes = ref_intel::compare_rgba(rw, rh, &rrgba, cw, ch, &crgba);
        for note in notes {
            let tag = match note.kind {
                ObservationKind::Observed => "Observed",
                ObservationKind::Derived => "Derived",
                ObservationKind::Uncertain => "Uncertain",
            };
            ui.label(format!("[{tag}] {}: {}", note.title, note.detail));
        }
    }

    fn reconstruction_rgba(&mut self, asset_id: &str) -> Option<(u32, u32, Vec<u8>)> {
        let session = self.extract.session.clone()?;
        // Match the selected candidate, or fall back to the Goomba reconstruction when
        // the owner opened compare from that card's workspace.
        let is_goomba = session
            .candidates
            .iter()
            .any(|c| c.id == asset_id && c.character_id == "goomba");
        let any_goomba = session
            .candidates
            .iter()
            .any(|c| c.character_id == "goomba");
        if !is_goomba && !(any_goomba && asset_id.contains("goomba")) {
            return None;
        }
        let assembly = self.assemble_goomba(&session)?;
        let pose = if self.studio.state.preview_pose.is_empty() {
            "walk".to_string()
        } else {
            self.studio.state.preview_pose.clone()
        };
        let frame = nesticles_core::reconstruct::pose_frame(
            &assembly,
            &pose,
            self.studio.state.preview_elapsed,
        )
        .map(|(_, frame)| frame)
        .or_else(|| assembly.frames.first())?;
        Some((frame.w, frame.h, frame.rgba.clone()))
    }

    fn reference_ai_panel(&mut self, ui: &mut egui::Ui, asset_id: &str) {
        ui.label(RichText::new("Optional AI assistance").size(18.0).strong());
        ui.label(
            "AI is optional. This workspace works with no credential. Requests never run when opening a project, selecting a card, importing a reference, or resizing.",
        );
        let has_key = !self.extract.openai_key.trim().is_empty();
        if !has_key {
            ui.label(
                RichText::new("No credential configured. Configure one under AI providers, or leave AI offline.")
                    .color(Color32::from_rgb(120, 80, 20)),
            );
        } else {
            ui.label(format!(
                "Credential present (session/env/keyring). Model: {}",
                self.extract.vision_model
            ));
        }
        ui.checkbox(
            &mut self.extract.ai_extract,
            "Enable AI analysis for this session",
        );

        let image_bytes = ref_intel::active_for(&self.project.asset_references, asset_id)
            .and_then(|item| {
                if item.png_base64.is_empty() {
                    None
                } else {
                    nesticles_core::pngutil::b64_decode(&item.png_base64)
                        .ok()
                        .map(|b| b.len())
                }
            })
            .unwrap_or(0);
        let preview = AiSendRequest {
            has_credential: has_key,
            user_confirmed: self.extract.ai_payload_confirmed,
            explicit_send: false,
            image_bytes,
            notes: self.extract.ref_workspace_notes.clone(),
            model: self.extract.vision_model.clone(),
        };
        ui.label(describe_ai_payload(&preview));
        ui.checkbox(
            &mut self.extract.ai_payload_confirmed,
            "I reviewed what will be sent (images/notes only; not the ROM)",
        );

        ui.horizontal_wrapped(|ui| {
            if design::soft_button(ui, "Send to AI").clicked() {
                self.send_reference_ai(asset_id, true);
            }
            if design::soft_button(ui, "Clear AI review").clicked() {
                self.extract.ai_review_text.clear();
                self.extract.ai_payload_confirmed = false;
            }
            if design::soft_button(ui, "Open AI providers").clicked() {
                self.studio.state.tool = Some(super::unmaker::StudioTool::Providers);
            }
        });

        if !self.extract.ai_review_text.is_empty() {
            ui.separator();
            ui.label(RichText::new("AI suggestion (not verified evidence)").strong());
            ui.label(&self.extract.ai_review_text);
            ui.label(
                "Any change to the reconstruction still requires a separate explicit owner action. AI does not override R3 Accept / Skip / Reject decisions.",
            );
        }

        if !self.project.ai_assist_records.is_empty() {
            ui.collapsing("Saved AI review records", |ui| {
                for record in self.project.ai_assist_records.iter().rev().take(5) {
                    ui.label(format!(
                        "{} / {} — {}",
                        record.provider,
                        record.model,
                        if record.success {
                            "suggestion"
                        } else {
                            "failed"
                        }
                    ));
                    if !record.response_text.is_empty() {
                        ui.label(&record.response_text);
                    }
                    if !record.error.is_empty() {
                        ui.label(RichText::new(&record.error).color(Color32::from_rgb(160, 40, 40)));
                    }
                }
            });
        }
    }

    fn send_reference_ai(&mut self, asset_id: &str, explicit: bool) {
        if !self.extract.ai_extract {
            self.studio.state.notice =
                "Enable AI analysis for this session before sending.".into();
            return;
        }
        let bytes = match ref_intel::active_for(&self.project.asset_references, asset_id) {
            Some(item) if !item.png_base64.is_empty() => {
                match nesticles_core::pngutil::b64_decode(&item.png_base64) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        self.studio.state.notice = err;
                        return;
                    }
                }
            }
            _ => {
                self.studio.state.notice =
                    "Select an embedded reference before sending to AI.".into();
                return;
            }
        };
        let req = AiSendRequest {
            has_credential: !self.extract.openai_key.trim().is_empty(),
            user_confirmed: self.extract.ai_payload_confirmed,
            explicit_send: explicit,
            image_bytes: bytes.len(),
            notes: self.extract.ref_workspace_notes.clone(),
            model: self.extract.vision_model.clone(),
        };
        if let Err(block) = gate_ai_send(&req) {
            self.studio.state.notice = block.message().into();
            return;
        }
        if self.extract.requests_used >= self.extract.request_limit {
            self.studio.state.notice = "This session reached its request limit.".into();
            return;
        }

        // Mock path for automated tests: never hit the network.
        if std::env::var("NESTICLES_AI_MOCK").ok().as_deref() == Some("1") {
            self.extract.requests_used += 1;
            let text = "MOCK AI SUGGESTION: dimensions and silhouette may differ; treat as unverified. No SpriteDef changes were applied.";
            self.extract.ai_review_text = text.into();
            self.project.ai_assist_records.push(AiAssistRecord {
                provider: "mock".into(),
                model: self.extract.vision_model.clone(),
                submitted_summary: describe_ai_payload(&req),
                response_text: text.into(),
                success: true,
                error: String::new(),
                suggestion_only: true,
                created_at: format!("asset:{asset_id}"),
            });
            self.dirty = true;
            self.studio.state.notice =
                "Mock AI suggestion recorded. Reconstruction was not modified.".into();
            return;
        }

        match openai::analyze_image(
            &self.extract.openai_key,
            &self.extract.vision_model,
            &bytes,
        ) {
            Ok(value) => {
                self.extract.requests_used += 1;
                let text = format!(
                    "Provider OpenAI / model {}. Response (suggestion only, not verified evidence):\n{}",
                    self.extract.vision_model,
                    value
                );
                let safe = openai::redact(&text, &self.extract.openai_key);
                self.extract.ai_review_text = safe.clone();
                self.project.ai_assist_records.push(AiAssistRecord {
                    provider: "openai".into(),
                    model: self.extract.vision_model.clone(),
                    submitted_summary: describe_ai_payload(&req),
                    response_text: safe,
                    success: true,
                    error: String::new(),
                    suggestion_only: true,
                    created_at: format!("asset:{asset_id}"),
                });
                self.dirty = true;
                self.studio.state.notice =
                    "AI suggestion ready for review. No reconstruction was modified.".into();
            }
            Err(err) => {
                let safe = openai::redact(&err, &self.extract.openai_key);
                self.extract.ai_review_text = format!("Request failed: {safe}");
                self.project.ai_assist_records.push(AiAssistRecord {
                    provider: "openai".into(),
                    model: self.extract.vision_model.clone(),
                    submitted_summary: describe_ai_payload(&req),
                    response_text: String::new(),
                    success: false,
                    error: safe.clone(),
                    suggestion_only: true,
                    created_at: format!("asset:{asset_id}"),
                });
                self.dirty = true;
                self.studio.state.notice = format!("AI request failed: {safe}");
            }
        }
    }

    fn import_local_asset_reference(&mut self, asset_id: &str) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
            .pick_file()
        else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        match std::fs::read(&path) {
            Ok(bytes) => match ref_intel::reference_from_local(
                asset_id,
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Local reference"),
                &path_str,
                &bytes,
                true,
            ) {
                Ok(mut item) => {
                    item.notes = self.extract.ref_workspace_notes.clone();
                    for existing in self
                        .project
                        .asset_references
                        .iter_mut()
                        .filter(|r| r.asset_id == asset_id)
                    {
                        existing.active = false;
                    }
                    item.active = true;
                    self.project.asset_references.push(item);
                    self.extract.ref_assoc_tex = None;
                    self.extract.ref_assoc_tex_id = None;
                    self.dirty = true;
                    self.studio.state.notice =
                        "Local reference associated and embedded in the project file.".into();
                }
                Err(err) => self.studio.state.notice = err,
            },
            Err(err) => {
                self.studio.state.notice = format!("Could not read {}: {err}", path.display());
            }
        }
    }

    fn start_direct_image_for_asset(&mut self, asset_id: &str) {
        let url = self.extract.ref_url.trim().to_string();
        if url.is_empty() {
            self.studio.state.notice = "Enter a direct image URL first.".into();
            return;
        }
        // Reuse the existing async fetch; association happens when bytes arrive
        // if NESTICLES binds the pending asset id.
        self.extract.pending_ref_asset_id = Some(asset_id.to_string());
        self.start_direct_image_reference();
    }

    /// After a direct image fetch, associate bytes with the pending asset when set.
    pub(crate) fn maybe_associate_fetched_reference(&mut self, url: &str, bytes: &[u8]) {
        let Some(asset_id) = self.extract.pending_ref_asset_id.take() else {
            return;
        };
        match ref_intel::reference_from_image_url(&asset_id, "URL reference", url, bytes) {
            Ok(mut item) => {
                item.notes = self.extract.ref_workspace_notes.clone();
                for existing in self
                    .project
                    .asset_references
                    .iter_mut()
                    .filter(|r| r.asset_id == asset_id)
                {
                    existing.active = false;
                }
                item.active = true;
                self.project.asset_references.push(item);
                self.extract.ref_assoc_tex = None;
                self.dirty = true;
                self.studio.state.notice =
                    "URL reference associated. It is observed evidence, not an approved extract."
                        .into();
            }
            Err(err) => {
                self.studio.state.notice = err;
            }
        }
    }
}
