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
use nesticles_core::reference::{self, RefClass, RefImage};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

enum RefEvent {
    Line(String),
    Page(Vec<RefImage>),
    Image { index: usize, bytes: Vec<u8> },
    Direct { url: String, bytes: Vec<u8> },
    Failed { detail: String },
}

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
    pub ref_log: Vec<String>,
    pub ref_items: Vec<RefImage>,
    pub ref_class: String,
    pub ref_category: String,
    pub ref_selected: Option<usize>,
    pub ref_bytes: Vec<u8>,
    pub ref_full: Option<(u32, u32)>,
    pub ref_rgba: Option<(u32, u32, Vec<u8>)>,
    pub ref_tex: Option<egui::TextureHandle>,
    ref_rx: Option<mpsc::Receiver<RefEvent>>,
    ref_cancel: Option<Arc<AtomicBool>>,
    pub ref_error: Option<String>,
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
            ref_log: vec!["A reference URL is optional.".into()],
            ref_items: Vec::new(),
            ref_class: "original".into(),
            ref_category: "All".into(),
            ref_selected: None,
            ref_bytes: Vec::new(),
            ref_full: None,
            ref_rgba: None,
            ref_tex: None,
            ref_rx: None,
            ref_cancel: None,
            ref_error: None,
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
        let stage_name = job.stage_name.clone();
        self.studio.note_stage(&stage_name);
        job_advance(job, &bytes, &name, &recipes, &[]);
        self.studio.note_stage(&job.stage_name);
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
                self.studio.state.stage = super::unmaker::Stage::Investigate;
                self.studio.state.investigated = true;
                self.studio.note_stage("Ready for review");
                self.studio.state.notice = self.status.clone();
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
            "This session starts empty. Disassembly and new work stay here until you save them.",
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
        self.reference_panel(ui);
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

    pub(crate) fn puzzle_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Guided assembly").font(serif(28.0)));
        let Some(mut puzzle) = self.extract.puzzle.clone() else {
            ui.label("Open a candidate from the review queue.");
            return;
        };
        ui.label(format!(
            "Canvas {}×{}  palette {}",
            puzzle.canvas_w, puzzle.canvas_h, puzzle.palette
        ));
        self.reference_picture(ui);
        if !puzzle.reference_note.is_empty() {
            ui.label(&puzzle.reference_note);
        }
        ui.label("No automatic visual match. Place and correct ROM tiles yourself.");
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
        if ui.button("Save local recipe").clicked() {
            self.save_puzzle_recipe(&puzzle);
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

    pub(super) fn reference_busy(&self) -> bool {
        self.extract.ref_rx.is_some()
    }

    pub fn pump_reference(&mut self) {
        let mut events = Vec::new();
        let mut disconnected = false;
        {
            let Some(rx) = &self.extract.ref_rx else {
                return;
            };
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        let mut done = disconnected && events.is_empty();
        for event in events {
            match event {
                RefEvent::Line(line) => {
                    self.extract.ref_note = line.clone();
                    self.extract.ref_log.push(line);
                }
                RefEvent::Page(items) => {
                    let count = items.len();
                    self.extract.ref_items = items;
                    let line = format!("Ready for reference selection. {count} image link(s).");
                    self.extract.ref_note = line.clone();
                    self.extract.ref_log.push(line);
                    if count == 0 {
                        self.extract.ref_error = Some(
                            "No images were found. Import a local image, or try another page."
                                .into(),
                        );
                    }
                    done = true;
                }
                RefEvent::Image { index, bytes } => {
                    self.accept_reference_bytes(index, bytes);
                    done = true;
                }
                RefEvent::Direct { url, bytes } => {
                    let item = RefImage {
                        title: "Direct image".into(),
                        category: "Other".into(),
                        class: RefClass::Custom,
                        page_url: url.clone(),
                        image_url: url,
                    };
                    self.extract.ref_items.push(item);
                    let index = self.extract.ref_items.len() - 1;
                    self.accept_reference_bytes(index, bytes);
                    done = true;
                }
                RefEvent::Failed { detail } => {
                    self.extract.ref_error = Some(detail.clone());
                    self.extract.ref_note = detail.clone();
                    self.extract.ref_log.push(detail);
                    done = true;
                }
            }
        }
        if done {
            self.extract.ref_rx = None;
            self.extract.ref_cancel = None;
        }
    }

    pub(crate) fn reference_panel(&mut self, ui: &mut egui::Ui) {
        let mut start = false;
        ui.horizontal_wrapped(|ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut self.extract.ref_url).desired_width(420.0),
            );
            if field.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                start = true;
            }
            if ui.button("Add reference URL").clicked() {
                start = true;
            }
            if ui.button("Continue without reference").clicked() {
                self.continue_without_reference();
            }
            if ui.button("Import local image").clicked() {
                self.import_local_reference();
            }
        });
        if start {
            self.start_reference_fetch();
        }
        for line in &self.extract.ref_log {
            ui.label(line);
        }
        if let Some(err) = &self.extract.ref_error.clone() {
            ui.label(RichText::new(err).strong());
            ui.horizontal_wrapped(|ui| {
                if ui.button("Continue without reference").clicked() {
                    self.continue_without_reference();
                }
                if ui.button("Fetch as a direct image").clicked() {
                    self.start_direct_image_reference();
                }
                if ui.button("Retry").clicked() {
                    self.start_reference_fetch();
                }
                if ui.button("Cancel").clicked() {
                    self.cancel_reference();
                    self.extract.ref_note = "The reference fetch was cancelled.".into();
                    self.extract.ref_log
                        .push("The reference fetch was cancelled.".into());
                    self.extract.ref_error = None;
                }
                if ui.button("Import local image").clicked() {
                    self.import_local_reference();
                }
            });
        } else if self.extract.ref_rx.is_some() && ui.button("Cancel").clicked() {
            self.cancel_reference();
            self.extract.ref_note = "The reference fetch was cancelled.".into();
            self.extract.ref_log
                .push("The reference fetch was cancelled.".into());
        }
        if self.extract.ref_items.is_empty() {
            return;
        }
        let original = self
            .extract
            .ref_items
            .iter()
            .filter(|item| item.class == RefClass::Original)
            .count();
        let custom = self
            .extract
            .ref_items
            .iter()
            .filter(|item| item.class == RefClass::Custom)
            .count();
        let chrome = self
            .extract
            .ref_items
            .iter()
            .filter(|item| item.class == RefClass::SiteChrome)
            .count();
        ui.label(format!(
            "Original-game references {original}. Custom artwork {custom}. Site chrome {chrome}. Nothing has been downloaded yet."
        ));
        ui.horizontal_wrapped(|ui| {
            for (id, label) in [
                ("original", "Original"),
                ("custom", "Custom"),
                ("all", "All"),
            ] {
                if ui
                    .selectable_label(self.extract.ref_class == id, label)
                    .clicked()
                {
                    self.extract.ref_class = id.into();
                }
            }
            for label in [
                "All",
                "Mario",
                "Luigi",
                "Characters",
                "Enemies",
                "Items",
                "Blocks",
                "Tiles",
                "Backgrounds",
                "Other",
            ] {
                if ui
                    .selectable_label(self.extract.ref_category == label, label)
                    .clicked()
                {
                    self.extract.ref_category = label.into();
                }
            }
        });
        let mut pick = None;
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .id_salt("reference-gallery")
            .show(ui, |ui| {
                for (index, item) in self.extract.ref_items.iter().enumerate() {
                    if !self.reference_visible(item) {
                        continue;
                    }
                    let on = self.extract.ref_selected == Some(index);
                    let state = if on && self.extract.ref_full.is_some() {
                        "preview ready"
                    } else {
                        "not downloaded"
                    };
                    let line = format!(
                        "{}  {}  {}  {state}",
                        item.title,
                        item.category,
                        item.class.label()
                    );
                    if ui.selectable_label(on, line).clicked() {
                        pick = Some(index);
                    }
                }
            });
        if let Some(index) = pick {
            self.start_reference_preview(index);
        }
        self.reference_picture(ui);
        if let Some((w, h)) = self.extract.ref_full {
            if let Some(index) = self.extract.ref_selected {
                if let Some(item) = self.extract.ref_items.get(index) {
                    ui.label(format!(
                        "{}  {}  {}  {}×{}  preview ready  {}",
                        item.title,
                        item.category,
                        item.class.label(),
                        w,
                        h,
                        item.image_url
                    ));
                    ui.label(reference::attribution(item));
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Open beside ROM tiles").clicked() {
                self.show_reference_with_tiles();
            }
            if ui.button("Open guided puzzle").clicked() {
                self.begin_reference_puzzle();
            }
        });
    }

    fn reference_visible(&self, item: &RefImage) -> bool {
        let class_ok = match self.extract.ref_class.as_str() {
            "custom" => item.class == RefClass::Custom,
            "all" => true,
            _ => matches!(item.class, RefClass::Original | RefClass::Local),
        };
        let cat = self.extract.ref_category.as_str();
        let cat_ok = cat == "All"
            || item.category == cat
            || (cat == "Mario" && item.category == "Characters")
            || (cat == "Luigi" && item.category == "Characters");
        class_ok && cat_ok
    }

    pub(super) fn reference_banner(&mut self, ui: &mut egui::Ui) {
        if self.extract.ref_rgba.is_none() {
            return;
        }
        ui.label("Reference beside ROM tiles. The picture is not CHR.");
        self.reference_picture(ui);
        if let Some(index) = self.extract.ref_selected {
            if let Some(item) = self.extract.ref_items.get(index) {
                ui.label(reference::attribution(item));
            }
        }
        if let Some(session) = &self.extract.session {
            ui.label(format!(
                "ROM payload {}  CHR tiles {}",
                &session.report.payload_sha256[..16.min(session.report.payload_sha256.len())],
                session.tiles.len()
            ));
        }
        if ui.button("Open guided puzzle").clicked() {
            self.begin_reference_puzzle();
        }
        ui.separator();
    }

    fn reference_picture(&mut self, ui: &mut egui::Ui) {
        self.ensure_ref_tex(ui.ctx());
        if let Some(tex) = &self.extract.ref_tex {
            ui.add(egui::Image::from_texture(tex).max_width(420.0));
        }
    }

    fn ensure_ref_tex(&mut self, ctx: &egui::Context) {
        if self.extract.ref_tex.is_some() {
            return;
        }
        let Some((w, h, rgba)) = &self.extract.ref_rgba else {
            return;
        };
        let image = egui::ColorImage::from_rgba_unmultiplied([*w as usize, *h as usize], rgba);
        self.extract.ref_tex = Some(ctx.load_texture(
            "reference-preview",
            image,
            egui::TextureOptions::LINEAR,
        ));
    }

    pub(crate) fn start_reference_fetch(&mut self) {
        let url = self.extract.ref_url.trim().to_string();
        if let Err(err) = nesticles_core::pipeline::allow_reference_url(&url) {
            let detail = format!("Failed before Connecting. {err}");
            self.extract.ref_error = Some(detail.clone());
            self.extract.ref_note = detail.clone();
            self.extract.ref_log.push(detail);
            return;
        }
        self.cancel_reference();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        self.extract.ref_cancel = Some(cancel.clone());
        self.extract.ref_rx = Some(rx);
        self.extract.ref_items.clear();
        self.extract.ref_selected = None;
        self.extract.ref_bytes.clear();
        self.extract.ref_full = None;
        self.extract.ref_rgba = None;
        self.extract.ref_tex = None;
        self.extract.ref_error = None;
        self.extract.ref_log = vec!["Connecting.".into()];
        self.extract.ref_note = "Connecting.".into();
        std::thread::spawn(move || {
            match openai::fetch_reference_page(&url, &cancel) {
                Err(err) => {
                    let _ = tx.send(RefEvent::Failed {
                        detail: reference::present_fetch_error("Page retrieval", &err),
                    });
                }
                Ok(bytes) => {
                    let _ = tx.send(RefEvent::Line(format!(
                        "Page retrieved. {} bytes. Scripts were not run.",
                        bytes.len()
                    )));
                    let _ = tx.send(RefEvent::Line("Inspecting page.".into()));
                    let html = String::from_utf8_lossy(&bytes);
                    let items = reference::scan_reference_html(&url, &html);
                    let _ = tx.send(RefEvent::Line(format!(
                        "Images discovered. {} reference image link(s).",
                        items.len()
                    )));
                    let _ = tx.send(RefEvent::Page(items));
                }
            }
        });
    }

    fn start_reference_preview(&mut self, index: usize) {
        let Some(item) = self.extract.ref_items.get(index).cloned() else {
            return;
        };
        if item.class == RefClass::Local {
            if let Ok(bytes) = fs::read(&item.image_url) {
                self.accept_reference_bytes(index, bytes);
            } else {
                let detail = "Failed during local import. The file could not be read.".to_string();
                self.extract.ref_error = Some(detail.clone());
                self.extract.ref_note = detail;
            }
            return;
        }
        self.cancel_reference();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        self.extract.ref_cancel = Some(cancel.clone());
        self.extract.ref_rx = Some(rx);
        self.extract.ref_error = None;
        self.extract.ref_log.push("Preparing previews.".into());
        self.extract.ref_note = "Preparing previews.".into();
        let url = item.image_url;
        std::thread::spawn(move || match openai::fetch_reference_image(&url, &cancel) {
            Ok(bytes) => {
                let _ = tx.send(RefEvent::Image { index, bytes });
            }
            Err(err) => {
                let _ = tx.send(RefEvent::Failed {
                    detail: reference::present_fetch_error("Preparing previews", &err),
                });
            }
        });
    }

    fn cancel_reference(&mut self) {
        if let Some(flag) = &self.extract.ref_cancel {
            flag.store(true, Ordering::Relaxed);
        }
        self.extract.ref_rx = None;
        self.extract.ref_cancel = None;
    }

    fn accept_reference_bytes(&mut self, index: usize, bytes: Vec<u8>) {
        let Some(item) = self.extract.ref_items.get(index).cloned() else {
            return;
        };
        match nesticles_core::pngutil::reference_preview(&bytes, 480) {
            Ok((w, h, rgba, full_w, full_h)) => {
                self.extract.ref_selected = Some(index);
                if bytes.starts_with(b"\x89PNG") {
                    self.extract.last_reference = bytes.clone();
                }
                self.extract.ref_bytes = bytes;
                self.extract.ref_full = Some((full_w, full_h));
                self.extract.ref_rgba = Some((w, h, rgba));
                self.extract.ref_tex = None;
                self.extract.ref_error = None;
                let line = format!(
                    "Preparing previews. {} is {}×{}. Ready for reference selection.",
                    item.title, full_w, full_h
                );
                self.extract.ref_note = line.clone();
                self.extract.ref_log.push(line);
            }
            Err(err) => {
                let detail = format!("Failed during image decoding. {err}");
                self.extract.ref_error = Some(detail.clone());
                self.extract.ref_note = detail.clone();
                self.extract.ref_log.push(detail);
            }
        }
    }

    pub(crate) fn continue_without_reference(&mut self) {
        self.cancel_reference();
        self.extract.ref_error = None;
        let line = "Continuing with the ROM bytes only.".to_string();
        self.extract.ref_note = line.clone();
        self.extract.ref_log.push(line);
        self.studio.state.notice =
            "Continuing without a reference. The cartridge investigation does not need one."
                .into();
    }

    pub(crate) fn reference_recovery_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Continue without reference").clicked() {
                self.continue_without_reference();
            }
            if ui.button("Import local image").clicked() {
                self.import_local_reference();
            }
            if ui.button("Fetch as a direct image").clicked() {
                self.start_direct_image_reference();
            }
            if ui.button("Retry").clicked() {
                self.start_reference_fetch();
            }
        });
    }

    pub(crate) fn start_direct_image_reference(&mut self) {
        let url = self.extract.ref_url.trim().to_string();
        if let Err(err) = nesticles_core::pipeline::allow_reference_url(&url) {
            let detail = format!("Failed before Connecting. {err}");
            self.extract.ref_error = Some(detail.clone());
            self.extract.ref_note = detail.clone();
            self.extract.ref_log.push(detail);
            return;
        }
        self.cancel_reference();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        self.extract.ref_cancel = Some(cancel.clone());
        self.extract.ref_rx = Some(rx);
        self.extract.ref_error = None;
        self.extract.ref_log
            .push("Fetching that address as an image.".into());
        self.extract.ref_note = "Fetching that address as an image.".into();
        std::thread::spawn(move || match openai::fetch_reference_image(&url, &cancel) {
            Ok(bytes) => {
                let _ = tx.send(RefEvent::Direct { url, bytes });
            }
            Err(err) => {
                let _ = tx.send(RefEvent::Failed {
                    detail: reference::present_fetch_error("image retrieval", &err),
                });
            }
        });
    }

    pub(crate) fn import_local_reference(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "gif"])
            .pick_file()
        else {
            return;
        };
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                let detail = format!("Failed during local import. {err}");
                self.extract.ref_error = Some(detail.clone());
                self.extract.ref_note = detail;
                return;
            }
        };
        let title = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("Local image");
        let item = reference::local_reference(title, &path.display().to_string());
        self.extract.ref_items.push(item);
        let index = self.extract.ref_items.len() - 1;
        self.accept_reference_bytes(index, bytes);
    }

    fn show_reference_with_tiles(&mut self) {
        if self.extract.ref_rgba.is_none() {
            self.status = "Select a reference and wait for its preview.".into();
            return;
        }
        if self.extract.session.is_none() {
            self.status = "Open a ROM before comparing tiles. The reference is not CHR.".into();
            return;
        }
        self.unmaker = UnmakerView::Tiles;
        self.status = "The reference is beside the ROM tile grid. Matching is manual.".into();
    }

    fn begin_reference_puzzle(&mut self) {
        if self.extract.ref_rgba.is_none() {
            self.status = "Select a reference and wait for its preview.".into();
            return;
        }
        let Some(index) = self.extract.ref_selected else {
            self.status = "Select a reference image first.".into();
            return;
        };
        let Some(item) = self.extract.ref_items.get(index).cloned() else {
            return;
        };
        let Some(session) = self.extract.session.as_mut() else {
            self.status = "Open a ROM before assembling tiles.".into();
            return;
        };
        let made = reference::reference_candidate(&item);
        let id = made.id.clone();
        session.candidates.push(made);
        let tile_count = session.tiles.len() as u32;
        let candidate = session
            .candidates
            .iter()
            .find(|candidate| candidate.id == id)
            .unwrap();
        let mut puzzle = puzzle_from(candidate, tile_count);
        puzzle.reference_note = reference::attribution(&item);
        puzzle
            .signals
            .push("No automatic visual match. Place ROM tiles by hand.".into());
        self.extract.puzzle = Some(puzzle);
        self.unmaker = UnmakerView::Puzzle;
        self.status = format!(
            "Guided puzzle opened for {}. ROM tiles stay in the CHR grid.",
            item.title
        );
    }

    fn save_puzzle_recipe(&mut self, puzzle: &Puzzle) {
        if self.extract.session.is_none() {
            self.status = "Open a ROM before saving a recipe.".into();
            return;
        }
        if puzzle.placed.is_empty() {
            self.status = "Place at least one ROM tile before saving a recipe.".into();
            return;
        }
        let id = puzzle.candidate_id.clone();
        {
            let Some(session) = self.extract.session.as_mut() else {
                return;
            };
            let Some(candidate) = session
                .candidates
                .iter_mut()
                .find(|candidate| candidate.id == id)
            else {
                self.status = "That puzzle is not attached to a candidate.".into();
                return;
            };
            if let Err(err) = puzzle_submit(candidate, puzzle) {
                self.status = err;
                return;
            }
        }
        let Some(session) = self.extract.session.as_ref() else {
            return;
        };
        let Some(candidate) = session.candidates.iter().find(|candidate| candidate.id == id) else {
            return;
        };
        let recipe = recipe_from(&session.report, candidate);
        let name = recipe.name.clone();
        match library().save_recipe(&recipe) {
            Ok(()) => {
                self.status = format!(
                    "Saved a local recipe for {name}. It was not shared and it is not an approved cartridge extract unless you approved it."
                );
            }
            Err(err) => self.status = err,
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
