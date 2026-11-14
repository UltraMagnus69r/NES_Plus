//! UnMaker entry, extraction review, puzzle, providers, and behavior authoring.
//! Decisions live in nesticles_core. This file draws them and saves the alpha library.

use super::{serif, NestApp};
use crate::openai;
use crate::studio::UnmakerView;
use eframe::egui::{self, RichText};
use nesticles_core::behavior::{launch_from, BehaviorGraph, BehaviorNode};
use nesticles_core::pipeline::{
    self, approve_candidate, export_gate, fail_candidate, ignore_candidate, job_advance,
    job_cancel, job_start, package_character, parse_package, puzzle_expand, puzzle_from,
    puzzle_submit, recipe_from, reinspect, restore_candidate, AlphaLibrary, ExtractJob,
    ExtractionSession, FailReason, Puzzle, ReviewStatus,
};
use nesticles_core::project::nid;
use std::fs;

pub struct ExtractUi {
    pub session: Option<ExtractionSession>,
    pub job: Option<ExtractJob>,
    pub openai_key: String,
    pub key_note: String,
    pub vision_model: String,
    pub image_model: String,
    pub behavior_model: String,
    pub ai_extract: bool,
    pub ai_generate: bool,
    pub ask_images: bool,
    pub request_limit: u32,
    pub requests_used: u32,
    pub api_status: String,
    pub last_reference: Vec<u8>,
    pub ref_url: String,
    pub ref_note: String,
    pub consent_send: bool,
    pub puzzle: Option<Puzzle>,
    pub fail_reason: usize,
    pub fail_note: String,
    pub export_hold: bool,
    pub prompt: String,
}

impl Default for ExtractUi {
    fn default() -> Self {
        let (key, note) = if let Some(key) = openai::key_from_env() {
            (
                key,
                "Using NESTICLES_OPENAI_API_KEY for this process.".into(),
            )
        } else if let Some(key) = openai::load_keyring() {
            (key, "Loaded the key from Secret Service.".into())
        } else {
            (String::new(), "No key stored. A typed key stays in this session unless Secret Service accepts it.".into())
        };
        Self {
            session: None,
            job: None,
            openai_key: key,
            key_note: note,
            vision_model: openai::DEFAULT_VISION.into(),
            image_model: openai::DEFAULT_IMAGE.into(),
            behavior_model: openai::DEFAULT_BEHAVIOR.into(),
            ai_extract: false,
            ai_generate: false,
            ask_images: true,
            request_limit: 4,
            requests_used: 0,
            api_status: "Not tested".into(),
            last_reference: Vec::new(),
            ref_url: String::new(),
            ref_note: "A reference URL is optional.".into(),
            consent_send: false,
            puzzle: None,
            fail_reason: 0,
            fail_note: String::new(),
            export_hold: false,
            prompt: String::new(),
        }
    }
}

fn library() -> AlphaLibrary {
    let root = directories::ProjectDirs::from("dev", "Nesticles", "Nesticles")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("nesticles-data"));
    AlphaLibrary::open(&root)
}

impl NestApp {
    pub fn pump_extract(&mut self) {
        let Some(rom) = self.rom.as_ref() else {
            return;
        };
        if self.extract.job.is_none() {
            return;
        }
        let bytes = rom.bytes.clone();
        let name = rom.name.clone();
        let recipes = library().recipes().unwrap_or_default();
        let job = self.extract.job.as_mut().unwrap();
        job_advance(job, &bytes, &name, &recipes, &[]);
        if job.session.is_some() || !job.error.is_empty() || job.cancelled {
            let done = self.extract.job.take().unwrap();
            if let Some(session) = done.session {
                self.status = format!(
                    "Payload {} — {}. {} candidates, {} unresolved. Raw tiles do not mean every object was reconstructed.",
                    &session.report.payload_sha256[..12.min(session.report.payload_sha256.len())],
                    session.report.profile_label,
                    session.candidates.len(),
                    session.coverage.iter().filter(|row| row.assembly == "unresolved").count()
                );
                self.extract.session = Some(session);
                self.unmaker = UnmakerView::Review;
            } else if !done.error.is_empty() {
                self.status = done.error;
            }
        }
    }

    pub fn begin_extract_job(&mut self) {
        self.extract.job = Some(job_start());
        self.extract.session = None;
        self.extract.export_hold = false;
    }

    pub(super) fn home_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("UnMaker Lab").font(serif(32.0)));
        ui.label(
            "Disassembly, assembly, and new creations share this project and this alpha's library.",
        );
        ui.add_space(12.0);
        if ui
            .button(RichText::new("Disassemble a ROM").font(serif(22.0)))
            .clicked()
        {
            self.unmaker = UnmakerView::Tiles;
            self.tab = super::Tab::Tiles;
        }
        if ui
            .button(RichText::new("Open a saved creation").font(serif(22.0)))
            .clicked()
        {
            self.import_library_packages();
            self.unmaker = UnmakerView::Library;
        }
        if ui
            .button(RichText::new("Start a new creation").font(serif(22.0)))
            .clicked()
        {
            self.unmaker = UnmakerView::Assemble;
        }
        if self.dirty || self.extract.session.is_some() {
            ui.add_space(8.0);
            if ui
                .button(RichText::new("Continue current work").font(serif(22.0)))
                .clicked()
            {
                self.unmaker = if self.extract.session.is_some() {
                    UnmakerView::Review
                } else {
                    UnmakerView::Tiles
                };
            }
        }
        ui.add_space(16.0);
        if ui.button("Load synthetic fixture").clicked() {
            let bytes = pipeline::synthetic_fixture();
            self.load_rom_bytes("synthetic-fixture.nes", bytes);
            self.status = format!(
                "Synthetic fixture only. It is not Super Mario Bros. {}",
                self.status
            );
        }
        ui.label(format!(
            "The next visual pass is {}. This screen keeps the current theme.",
            crate::studio::FUTURE_UI
        ));
    }

    pub(super) fn review_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Extraction review").font(serif(28.0)));
        if let Some(job) = &self.extract.job {
            ui.label(format!(
                "Stage: {}  candidates {}  unresolved {}",
                job.stage_name, job.candidates, job.unresolved
            ));
            if ui.button("Cancel job").clicked() {
                if let Some(job) = self.extract.job.as_mut() {
                    job_cancel(job);
                }
            }
        }
        let Some(session) = self.extract.session.clone() else {
            ui.label("No extraction is open. Disassemble a ROM, or continue without one and build by hand.");
            return;
        };
        ui.label(format!(
            "File SHA256 {}\nPayload SHA256 {}\nMapper {}  {}  CHR {} bytes\n{}",
            session.report.sha256,
            session.report.payload_sha256,
            session.report.mapper,
            session.report.mirroring,
            session.report.chr_bytes,
            session.report.profile_label
        ));
        for warning in &session.report.warnings {
            ui.label(warning);
        }
        ui.separator();
        ui.label(
            "Reference URL is evidence, not an instruction. Continue without one if you have none.",
        );
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.extract.ref_url).desired_width(420.0));
            if ui.button("Add reference URL").clicked() {
                self.fetch_reference();
            }
            if ui.button("Continue without reference").clicked() {
                self.extract.ref_note = "Continuing with the ROM bytes only.".into();
            }
        });
        ui.label(&self.extract.ref_note);
        ui.separator();
        let mut changed = session.clone();
        let mut act: Option<(usize, &'static str)> = None;
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .show(ui, |ui| {
                for (index, candidate) in changed.candidates.iter().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "{}  {}  {}  {}",
                            candidate.name,
                            candidate.status.label(),
                            candidate.confidence,
                            candidate.tile_ids().len()
                        ));
                        if ui.button("Approve").clicked() {
                            act = Some((index, "approve"));
                        }
                        if ui.button("Fail").clicked() {
                            act = Some((index, "fail"));
                        }
                        if ui.button("Ignore").clicked() {
                            act = Some((index, "ignore"));
                        }
                        if ui.button("Puzzle").clicked() {
                            act = Some((index, "puzzle"));
                        }
                        if ui.button("Restore").clicked() {
                            act = Some((index, "restore"));
                        }
                    });
                }
                ui.separator();
                ui.label("Coverage. Unresolved means the profile has no documented assembly.");
                for row in &changed.coverage {
                    ui.label(format!(
                        "{}: raw {}, assembly {}, animation {}, palette {}, {}",
                        row.name, row.raw, row.assembly, row.animation, row.palette, row.review
                    ));
                }
            });
        if let Some((index, kind)) = act {
            let candidate = &mut changed.candidates[index];
            match kind {
                "approve" => {
                    if let Err(err) = approve_candidate(candidate) {
                        self.status = err;
                    }
                }
                "fail" => {
                    let reason = FAIL[self.extract.fail_reason.min(FAIL.len() - 1)];
                    fail_candidate(candidate, reason, &self.extract.fail_note);
                }
                "ignore" => ignore_candidate(candidate),
                "restore" => restore_candidate(candidate),
                "puzzle" => {
                    let puzzle = puzzle_from(candidate, changed.tiles.len() as u32);
                    self.extract.puzzle = Some(puzzle);
                    self.unmaker = UnmakerView::Puzzle;
                }
                _ => {}
            }
        }
        ui.horizontal(|ui| {
            for (index, reason) in FAIL.iter().enumerate() {
                ui.radio_value(&mut self.extract.fail_reason, index, reason.label());
            }
        });
        ui.add(egui::TextEdit::singleline(&mut self.extract.fail_note).hint_text("Failure note"));
        if ui.button("Reinspect selected failed grouping").clicked() {
            if let Some(candidate) = changed
                .candidates
                .iter_mut()
                .find(|c| c.status == ReviewStatus::Failed)
            {
                let tiles = candidate.tile_ids();
                self.status = reinspect(candidate, &tiles).unwrap_or_else(|err| err);
            }
        }
        let gate = export_gate(&changed);
        if gate.needs_confirmation {
            ui.label(format!(
                "Ignored bucket: {} item(s). They stay in the session and are left out of an ordinary export.",
                gate.ignored_names.len()
            ));
            for name in &gate.ignored_names {
                ui.label(name);
            }
        }
        if ui.button("Export approved to Maker Engine").clicked() {
            if gate.needs_confirmation && !self.extract.export_hold {
                self.extract.export_hold = true;
                self.status = "Confirm the ignored bucket before export.".into();
            } else {
                self.finish_export(&changed);
            }
        }
        if self.extract.export_hold && ui.button("Confirm export without ignored assets").clicked()
        {
            self.finish_export(&changed);
            self.extract.export_hold = false;
        }
        if ui.button("Export knowledge package").clicked() {
            self.export_knowledge_file();
        }
        if ui.button("Import knowledge package").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Knowledge", &["nknowledge", "json"])
                .pick_file()
            {
                match std::fs::read_to_string(&path) {
                    Ok(text) => self.import_knowledge_file(&text),
                    Err(err) => self.status = err.to_string(),
                }
            }
        }
        self.extract.session = Some(changed);
    }

    fn finish_export(&mut self, session: &ExtractionSession) {
        let Some(rom) = self.rom.as_ref() else {
            self.status = "The ROM is not open, so pixels cannot be copied. Approved recipes can still be saved.".into();
            return;
        };
        let bytes = rom.bytes.clone();
        let approved: Vec<String> = session
            .candidates
            .iter()
            .filter(|c| c.status == ReviewStatus::Approved)
            .map(|c| c.character_id.clone())
            .collect();
        let mut done = 0usize;
        let mut seen = Vec::new();
        for id in approved {
            if seen.contains(&id) {
                continue;
            }
            seen.push(id.clone());
            match package_character(session, &bytes, &id) {
                Ok(package) => {
                    if let Err(err) = library().save_package(&package) {
                        self.status = err;
                        return;
                    }
                    match pipeline::import_package(&mut self.project, &package) {
                        Ok(sprite) => {
                            self.sprite_id = Some(sprite);
                            done += 1;
                        }
                        Err(err) => {
                            self.status = err;
                            return;
                        }
                    }
                    if let Some(candidate) =
                        session.candidates.iter().find(|c| c.character_id == id)
                    {
                        let _ = library().save_recipe(&recipe_from(&session.report, candidate));
                    }
                }
                Err(err) => {
                    self.status = err;
                    return;
                }
            }
        }
        self.dirty = true;
        self.workspace = crate::studio::Workspace::Maker;
        self.status = format!(
            "Exported {done} library item(s). Ignored candidates stayed in the session and were not deleted."
        );
    }

    pub(super) fn puzzle_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Guided assembly").font(serif(28.0)));
        let Some(mut puzzle) = self.extract.puzzle.clone() else {
            ui.label("Open a candidate from the review queue.");
            return;
        };
        ui.label(format!(
            "Canvas {}×{}  palette {}",
            puzzle.canvas_w, puzzle.canvas_h, puzzle.palette
        ));
        ui.horizontal(|ui| {
            if ui.button("Expand adjacent tiles").clicked() {
                let count = self
                    .extract
                    .session
                    .as_ref()
                    .map(|s| s.tiles.len())
                    .unwrap_or(0) as u32;
                puzzle_expand(&mut puzzle, count);
            }
            if ui.button("Mirror placed").clicked() {
                for spot in &mut puzzle.placed {
                    spot.flip_x = !spot.flip_x;
                }
            }
        });
        ui.label("Pool");
        let mut add = None;
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            for (index, tile) in puzzle.pool.iter().enumerate() {
                if ui.button(format!("{tile}")).clicked() {
                    add = Some(*tile);
                }
                if ui.small_button("wrong").clicked() {
                    remove = Some(index);
                }
            }
        });
        if let Some(tile) = add {
            puzzle.placed.push(pipeline::TileSpot {
                tile,
                x: (puzzle.placed.len() as i32 % 4) * 8,
                y: (puzzle.placed.len() as i32 / 4) * 8,
                flip_x: false,
                flip_y: false,
            });
        }
        if let Some(index) = remove {
            if let Some(tile) = puzzle.pool.get(index).copied() {
                puzzle.marked_wrong.push(tile);
                puzzle.pool.remove(index);
                puzzle.placed.retain(|spot| spot.tile != tile);
            }
        }
        ui.label(format!("Placed {}", puzzle.placed.len()));
        ui.horizontal(|ui| {
            ui.label("State");
            ui.text_edit_singleline(&mut puzzle.state);
        });
        for signal in &puzzle.signals {
            ui.label(signal);
        }
        if ui.button("Submit correction").clicked() {
            if let Some(session) = self.extract.session.as_mut() {
                if let Some(candidate) = session
                    .candidates
                    .iter_mut()
                    .find(|c| c.id == puzzle.candidate_id)
                {
                    self.status = match puzzle_submit(candidate, &puzzle) {
                        Ok(()) => "Correction is back in the review queue.".into(),
                        Err(err) => err,
                    };
                }
            }
            self.unmaker = UnmakerView::Review;
        }
        self.extract.puzzle = Some(puzzle);
    }

    pub(super) fn providers_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("AI providers").font(serif(28.0)));
        ui.label("OpenAI is the first provider. The key is not stored in the project.");
        ui.label(&self.extract.key_note);
        ui.label("API key");
        ui.add(
            egui::TextEdit::singleline(&mut self.extract.openai_key)
                .password(true)
                .desired_width(360.0),
        );
        ui.horizontal(|ui| {
            if ui.button("Save credentials").clicked() {
                match openai::store_keyring(&self.extract.openai_key) {
                    Ok(()) => self.extract.key_note = "Saved in Secret Service.".into(),
                    Err(err) => {
                        self.extract.key_note = format!("{err} Session only.");
                    }
                }
            }
            if ui.button("Test connection").clicked() {
                let status =
                    openai::test_connection(&self.extract.openai_key, &self.extract.vision_model);
                self.extract.api_status = status.label().into();
            }
            if ui.button("Remove credentials").clicked() {
                let _ = openai::clear_keyring();
                self.extract.openai_key.clear();
                self.extract.key_note =
                    "Removed from this session. Secret Service was asked to clear it.".into();
                self.extract.api_status = "Not tested".into();
            }
        });
        ui.label(format!("Connection: {}", self.extract.api_status));
        ui.label("Vision / analysis model");
        ui.text_edit_singleline(&mut self.extract.vision_model);
        ui.label("Image generation / editing model");
        ui.text_edit_singleline(&mut self.extract.image_model);
        ui.label("Behavior assistance model");
        ui.text_edit_singleline(&mut self.extract.behavior_model);
        ui.checkbox(&mut self.extract.ai_extract, "Enable AI extraction");
        ui.checkbox(&mut self.extract.ai_generate, "Enable image generation");
        ui.checkbox(&mut self.extract.ask_images, "Ask before sending an image");
        ui.label(format!(
            "Requests this session: {} / {}  (no price is invented here)",
            self.extract.requests_used, self.extract.request_limit
        ));
        ui.separator();
        ui.label("Generation prompt for an original or corrected character. Commercial names are still refused by the image guard.");
        ui.text_edit_multiline(&mut self.extract.prompt);
        if ui.button("Generate image").clicked() {
            self.generate_image();
        }
        if ui.button("Analyze reference with vision").clicked() {
            self.analyze_reference();
        }
    }

    pub(super) fn behavior_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Behavior").font(serif(28.0)));
        ui.label("Graphs are data. Patrol and jump on a Hammer Bro derivative are labels copied from the parent. The returning projectile is the part Rust Play steps.");
        if ui.button("New Hammer Bro graph").clicked() {
            let mut graph = BehaviorGraph::new(nid("behavior-"), "Hammer Bro");
            graph.derived_from = "Hammer Bro".into();
            graph.nodes = vec![
                BehaviorNode::Patrol,
                BehaviorNode::Jump,
                BehaviorNode::DetectPlayer,
                BehaviorNode::FireStraight { speed: 2.0 },
            ];
            self.project.behaviors.push(graph);
            self.dirty = true;
        }
        if ui.button("Derive returning projectile").clicked() {
            let parent = self.project.behaviors.last().cloned().unwrap_or_else(|| {
                let mut graph = BehaviorGraph::new(nid("behavior-"), "Hammer Bro");
                graph.nodes = vec![
                    BehaviorNode::Patrol,
                    BehaviorNode::Jump,
                    BehaviorNode::FireStraight { speed: 2.0 },
                ];
                graph
            });
            let derived = parent.derive_returning(nid("behavior-"), "Boomerang Bro", 3.0, 48.0);
            self.project.behaviors.push(derived);
            self.dirty = true;
            self.status = "Derived a returning projectile. Attach it to a blueprint, then place that blueprint.".into();
        }
        if ui
            .button("Attach last graph to selected blueprint")
            .clicked()
        {
            let blueprint = self.project.blueprints.last().map(|b| b.id.clone());
            if let (Some(blueprint), Some(graph)) = (blueprint, self.project.behaviors.last_mut()) {
                graph.blueprint_id = blueprint;
                self.dirty = true;
                self.status =
                    "The graph is attached. Place that blueprint's picture and press Play.".into();
            } else {
                self.status = "Add a blueprint and a graph first.".into();
            }
        }
        for graph in &self.project.behaviors {
            ui.label(format!(
                "{}  parent {}  blueprint {}",
                graph.name, graph.parent_id, graph.blueprint_id
            ));
            for note in graph.unavailable() {
                ui.label(note);
            }
            if launch_from(graph, 0.0, 0.0, 1.0).is_some() {
                ui.label("Rust Play will launch a returning projectile for this graph.");
            }
        }
    }

    fn fetch_reference(&mut self) {
        let url = self.extract.ref_url.clone();
        match openai::fetch_reference(&url) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                let images = openai::image_urls(&text);
                self.extract.last_reference = bytes.clone();
                self.extract.ref_note = format!(
                    "Fetched {} bytes. Scripts were not run. {} image URL(s) can be reviewed.",
                    bytes.len(),
                    images.len()
                );
            }
            Err(err) => self.extract.ref_note = err,
        }
    }

    fn analyze_reference(&mut self) {
        if !self.extract.ai_extract {
            self.status = "AI extraction is off.".into();
            return;
        }
        if self.extract.ask_images && !self.extract.consent_send {
            self.extract.consent_send = true;
            self.status =
                "Press Analyze again to send the reference image. The ROM is not included.".into();
            return;
        }
        if self.extract.requests_used >= self.extract.request_limit {
            self.status = "This session reached its request limit.".into();
            return;
        }
        if self
            .extract
            .last_reference
            .get(..8)
            .map(|b| b.starts_with(b"\x89PNG"))
            .unwrap_or(false)
        {
            match openai::analyze_image(
                &self.extract.openai_key,
                &self.extract.vision_model,
                &self.extract.last_reference,
            ) {
                Ok(value) => {
                    self.extract.requests_used += 1;
                    let count = self
                        .extract
                        .session
                        .as_ref()
                        .map(|s| s.tiles.len())
                        .unwrap_or(512) as u32;
                    match nesticles_core::pipeline::validate_ai_proposal(&value, count.max(1)) {
                        Ok(candidate) => {
                            if let Some(session) = self.extract.session.as_mut() {
                                session.candidates.push(candidate);
                            }
                            self.status =
                                "Vision proposal is in the review queue. It is not approved."
                                    .into();
                        }
                        Err(err) => self.status = err,
                    }
                }
                Err(err) => self.status = err,
            }
        } else {
            self.status = "Provide a PNG reference. HTML was not sent to the model, and the ROM was not attached.".into();
        }
    }

    fn generate_image(&mut self) {
        if !self.extract.ai_generate {
            self.status = "Image generation is off.".into();
            return;
        }
        if self.extract.openai_key.is_empty() {
            self.extract.api_status = nesticles_core::pipeline::ApiStatus::InvalidCredentials
                .label()
                .into();
            self.status = "There is no API key.".into();
            return;
        }
        if self.extract.requests_used >= self.extract.request_limit {
            self.status = "This session reached its request limit.".into();
            return;
        }
        if self.extract.ask_images && !self.extract.consent_send {
            self.extract.consent_send = true;
            self.status =
                "Press Generate again to send the prompt. No project file is attached.".into();
            return;
        }
        match openai::generate_image(
            &self.extract.openai_key,
            &self.extract.image_model,
            &self.extract.prompt,
        ) {
            Ok(bytes) => {
                self.extract.requests_used += 1;
                match nesticles_core::pngutil::sprite_from_image(&bytes, 32) {
                    Ok((w, h, rgba, png)) => {
                        self.add_rgba_asset(
                            "Generated",
                            nesticles_core::AssetKind::Enemy,
                            w,
                            h,
                            rgba,
                            png,
                        );
                        self.status = "Generated image is a sprite candidate. It is not approved, and the original bytes stay in this result.".into();
                    }
                    Err(err) => self.status = err,
                }
            }
            Err(err) => {
                self.extract.api_status = err.clone();
                self.status = err;
            }
        }
    }

    fn import_library_packages(&mut self) {
        let root = library().root.join("packages");
        let Ok(dir) = fs::read_dir(&root) else {
            self.status = "This alpha's library has no packages yet.".into();
            return;
        };
        let mut count = 0;
        for entry in dir.filter_map(|e| e.ok()) {
            let Ok(text) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let Ok(package) = parse_package(&text) else {
                continue;
            };
            if pipeline::import_package(&mut self.project, &package).is_ok() {
                count += 1;
            }
        }
        if count > 0 {
            self.dirty = true;
        }
        self.status = format!("Opened {count} saved creation(s) from this alpha's library.");
    }

    pub fn export_knowledge_file(&mut self) {
        let recipes = library().recipes().unwrap_or_default();
        match pipeline::export_knowledge(&recipes) {
            Ok(text) => {
                let path = library().root.join("export.nknowledge");
                if fs::write(&path, text).is_ok() {
                    self.status = format!(
                        "Knowledge package written to {}. It was not uploaded.",
                        path.display()
                    );
                }
            }
            Err(err) => self.status = err,
        }
    }

    pub fn import_knowledge_file(&mut self, text: &str) {
        match pipeline::import_knowledge(text) {
            Ok(recipes) => {
                for recipe in recipes {
                    if let Err(err) = library().save_recipe(&recipe) {
                        self.status = err;
                        return;
                    }
                }
                self.status = "Knowledge imported into this alpha's library.".into();
            }
            Err(err) => self.status = err,
        }
    }
}

const FAIL: [FailReason; 8] = [
    FailReason::WrongIdentity,
    FailReason::WrongTiles,
    FailReason::WrongPalette,
    FailReason::WrongAlignment,
    FailReason::MissingFrames,
    FailReason::WrongAnimation,
    FailReason::Duplicate,
    FailReason::Other,
];
