use crate::audio_out;
use crate::export_game;
use crate::keys::KeyFile;
use crate::net::{self, AssistDone, ImageDone};
use eframe::egui::{
    self, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, TextureHandle, TextureOptions,
    Vec2,
};
use nesticles_core::audio::{scan_brr, scan_dpcm, scan_gb_waves, AudioHit};
use nesticles_core::codec::{
    decode_tile, default_palette, indices_to_rgba, scan_tiles, tile_bytes, Codec, TileCandidate,
};
use nesticles_core::detect::{detect_rom, find_strings, RomFormat, RomInfo, MAX_ROM_BYTES};
use nesticles_core::disasm::{disassemble6502, AsmLine};
use nesticles_core::enemy_ai::{archetype_label, override_label, EnemyAiStyle, SMB1_ROSTER};
use nesticles_core::physics::{play_surface_scale, CharacterPhysics, Input, Sim, TILE};
use nesticles_core::pngutil::{b64_decode, b64_encode};
use nesticles_core::project::{
    audit_course, nid, Asset, AssetKind, Cell, EntryDirection, Facing, Layer, PlacementWarp,
    Project, WarpTarget,
};
use nesticles_core::wav::{import_audio, read_pcm_wav, wav_from_samples};
use nesticles_core::{
    add_entrance, add_return, assign_cell, clear_exit, clear_named_exit, compose_fixture,
    demo_project, demo_rom, enemy_from_archetype, enemy_role, ensure_legacy_blueprints,
    ensure_legacy_transitions, gameplay_options, load_project_json, placed_fixture,
    placement_label, remove_entrance, remove_return, resolve_warp_arrival, set_exit,
    set_placement_warp, suggest_kind, update_entrance, LegacyBlock, LegacyHazard, LegacyItem,
};
use std::collections::{HashMap, HashSet};
use std::process::Child;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Cartridge,
    Tiles,
    Audio,
    Notes,
    Library,
    Courses,
    Overworld,
    Play,
    Export,
    Guide,
}

struct RomSession {
    name: String,
    bytes: Vec<u8>,
    info: RomInfo,
}

struct PlayState {
    from_map: bool,
    level_id: String,
    sim: Option<Sim>,
    cleared: HashSet<String>,
    message: String,
    accum: f32,
    select: usize,
}

enum Req {
    Assist {
        keys: KeyFile,
        history: Vec<(String, String)>,
        project: Project,
    },
    Image {
        keys: KeyFile,
        prompt: String,
        side: u32,
        kind: AssetKind,
    },
    Godot {
        executable: std::path::PathBuf,
        project_dir: std::path::PathBuf,
        version: String,
    },
}

enum Resp {
    Assist(Result<AssistDone, String>),
    Image {
        result: Result<ImageDone, String>,
        kind: AssetKind,
    },
    Godot {
        result: Result<Child, String>,
        version: String,
        executable: std::path::PathBuf,
    },
}

pub struct NestApp {
    project: Project,
    tab: Tab,
    status: String,
    rom: Option<RomSession>,
    rom_path: String,
    codec: Codec,
    tile_offset_text: String,
    columns: usize,
    rows: usize,
    page: usize,
    palette: Vec<[u8; 3]>,
    transparent_zero: bool,
    selected: Option<usize>,
    stamp: usize,
    tag: AssetKind,
    candidates: Vec<TileCandidate>,
    page_tex: Option<(u64, TextureHandle)>,
    hits: Vec<AudioHit>,
    strings: Vec<(usize, String)>,
    disasm_index_text: String,
    disasm_base_text: String,
    disasm: Vec<AsmLine>,
    lib_sel: Option<String>,
    level_sel: usize,
    paint_asset: Option<String>,
    paint_start: bool,
    active_style: String,
    tray_filter: String,
    node_sel: usize,
    link_from: Option<String>,
    play: Option<PlayState>,
    hold_left: bool,
    hold_right: bool,
    hold_up: bool,
    hold_down: bool,
    hold_jump: bool,
    hold_run: bool,
    jump_edge: bool,
    /// Selected enemy cell on the course being edited, if any.
    enemy_pick: Option<(i32, i32)>,
    /// Gameplay applied to parts painted from here on. 0 is decoration.
    brush_gameplay: usize,
    /// When set, a grid click selects a placement instead of painting.
    edit_placement: bool,
    placement_pick: Option<(i32, i32)>,
    warp_x_text: String,
    warp_y_text: String,
    /// "inherit", "cleared", "here", or a course id.
    warp_level_draft: String,
    warp_draft_for: Option<(String, i32, i32)>,
    entrance_pick: Option<String>,
    exit_level_draft: String,
    exit_entrance_draft: String,
    exit_draft_for: Option<(String, i32, i32)>,
    keys_open: bool,
    image_open: bool,
    keys: KeyFile,
    keys_status: String,
    chat: Vec<(String, String)>,
    chat_input: String,
    assist_busy: bool,
    image_prompt: String,
    image_side: u32,
    image_kind: AssetKind,
    image_busy: bool,
    image_status: String,
    tx: Sender<Req>,
    rx: Receiver<Resp>,
    textures: HashMap<String, TextureHandle>,
    brand_icon: TextureHandle,
    tex_epoch: u64,
    export_path: String,
    export_note: String,
    godot_path: String,
    godot_child: Option<Child>,
    godot_busy: bool,
    music: Option<Child>,
    last_cue: Instant,
}

impl NestApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_theme(&cc.egui_ctx);
        let (tx, rx_req) = mpsc::channel::<Req>();
        let (tx_resp, rx) = mpsc::channel::<Resp>();
        std::thread::spawn(move || {
            while let Ok(req) = rx_req.recv() {
                let resp = match req {
                    Req::Assist {
                        keys,
                        history,
                        project,
                    } => Resp::Assist(net::run_assistant(&keys, &history, project)),
                    Req::Image {
                        keys,
                        prompt,
                        side,
                        kind,
                    } => Resp::Image {
                        result: net::run_image(&keys, &prompt, side),
                        kind,
                    },
                    Req::Godot {
                        executable,
                        project_dir,
                        version,
                    } => {
                        let result =
                            nesticles_core::godot::import_project(&executable, &project_dir)
                                .and_then(|_| {
                                    nesticles_core::godot::launch_preview(&executable, &project_dir)
                                });
                        Resp::Godot {
                            result,
                            version,
                            executable,
                        }
                    }
                };
                if tx_resp.send(resp).is_err() {
                    break;
                }
            }
        });
        let mut project = demo_project();
        // In memory only. Seed Spirit's file on disk is not rewritten by opening the app.
        ensure_legacy_blueprints(&mut project);
        let paint_asset = project
            .assets
            .iter()
            .find(|a| a.kind == AssetKind::Terrain)
            .map(|a| a.id.clone());
        let active_style = project
            .styles
            .first()
            .map(|style| style.id.clone())
            .unwrap_or_default();
        let brand_icon = brand_texture(&cc.egui_ctx);
        Self {
            project,
            tab: Tab::Courses,
            status: "Seed Spirit is loaded. Moss Steps is on the board.".into(),
            rom: None,
            rom_path: String::new(),
            codec: Codec::Nes,
            tile_offset_text: "0".into(),
            columns: 16,
            rows: 8,
            page: 0,
            palette: default_palette(Codec::Nes),
            transparent_zero: true,
            selected: None,
            stamp: 1,
            tag: AssetKind::Player,
            candidates: Vec::new(),
            page_tex: None,
            hits: Vec::new(),
            strings: Vec::new(),
            disasm_index_text: "16".into(),
            disasm_base_text: "8000".into(),
            disasm: Vec::new(),
            lib_sel: None,
            level_sel: 0,
            paint_asset,
            paint_start: false,
            active_style,
            tray_filter: "course".into(),
            node_sel: 0,
            link_from: None,
            play: None,
            hold_left: false,
            hold_right: false,
            hold_up: false,
            hold_down: false,
            hold_jump: false,
            hold_run: false,
            jump_edge: false,
            enemy_pick: None,
            brush_gameplay: 0,
            edit_placement: false,
            placement_pick: None,
            warp_x_text: "0".into(),
            warp_y_text: "0".into(),
            warp_level_draft: "inherit".into(),
            warp_draft_for: None,
            entrance_pick: None,
            exit_level_draft: String::new(),
            exit_entrance_draft: String::new(),
            exit_draft_for: None,
            keys_open: false,
            image_open: false,
            keys: KeyFile::load(),
            keys_status: String::new(),
            chat: vec![(
                "assistant".into(),
                "I can rename courses, paint cells, set the start, and link the overworld. I do not draw. Use the Image maker for a new sprite.".into(),
            )],
            chat_input: String::new(),
            assist_busy: false,
            image_prompt: "an original seed spirit with a paper lantern, simple sprite, plain background".into(),
            image_side: 32,
            image_kind: AssetKind::Item,
            image_busy: false,
            image_status: "Generate uses only the image maker key, and only when you press Generate.".into(),
            tx,
            rx,
            textures: HashMap::new(),
            brand_icon,
            tex_epoch: 1,
            export_path: "export/seed-spirit".into(),
            export_note: String::new(),
            godot_path: String::new(),
            godot_child: None,
            godot_busy: false,
            music: None,
            last_cue: Instant::now(),
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let godot_wait = self.godot_child.as_mut().map(|child| child.try_wait());
        match godot_wait {
            Some(Ok(Some(status))) => {
                if !status.success() {
                    self.status = format!(
                        "Godot preview exited ({status}). The browser exporter was not used."
                    );
                }
                self.godot_child = None;
            }
            Some(Err(err)) => {
                self.status = format!("Could not check the Godot preview: {err}");
                self.godot_child = None;
            }
            _ => {}
        }
        while let Ok(resp) = self.rx.try_recv() {
            match resp {
                Resp::Assist(Ok(done)) => {
                    self.project = done.project;
                    self.textures.clear();
                    self.tex_epoch = self.tex_epoch.wrapping_add(1);
                    if self.level_sel >= self.project.levels.len() {
                        self.level_sel = 0;
                    }
                    self.chat.extend(done.lines);
                    self.assist_busy = false;
                    self.status = "Assistant updated the project.".into();
                }
                Resp::Assist(Err(err)) => {
                    self.chat.push(("assistant".into(), err));
                    self.assist_busy = false;
                }
                Resp::Image {
                    result: Ok(done),
                    kind,
                } => {
                    self.add_rgba_asset(
                        "Generated",
                        kind,
                        done.side,
                        done.side,
                        done.rgba,
                        done.png,
                    );
                    self.image_busy = false;
                    self.image_status = "Added the sprite to the library.".into();
                    self.tab = Tab::Library;
                }
                Resp::Image {
                    result: Err(err), ..
                } => {
                    self.image_busy = false;
                    self.image_status = err;
                }
                Resp::Godot {
                    result,
                    version,
                    executable,
                } => {
                    self.godot_busy = false;
                    match result {
                        Ok(child) => {
                            if let Some(mut old) = self.godot_child.take() {
                                std::thread::spawn(move || {
                                    let _ = old.wait();
                                });
                            }
                            self.godot_child = Some(child);
                            self.status = format!(
                                "Experimental Godot preview launched with {} ({}). It is only the player picture, solid fixtures, movement, jump, and a camera.",
                                executable.display(),
                                version
                            );
                        }
                        Err(err) => self.status = err,
                    }
                }
            }
        }
        if self.assist_busy || self.image_busy || self.play.is_some() {
            ctx.request_repaint();
        }
    }

    fn step_play(&mut self, ctx: &egui::Context) {
        if self.play.as_ref().map(|p| p.sim.is_none()).unwrap_or(true) {
            return;
        }
        let (left, right, up, down, jump_held, jump_key, run_held) = ctx.input(|i| {
            (
                i.key_down(egui::Key::ArrowLeft)
                    || i.key_down(egui::Key::A)
                    || i.key_down(egui::Key::H),
                i.key_down(egui::Key::ArrowRight)
                    || i.key_down(egui::Key::D)
                    || i.key_down(egui::Key::L),
                i.key_down(egui::Key::ArrowUp) || i.key_down(egui::Key::W),
                i.key_down(egui::Key::ArrowDown) || i.key_down(egui::Key::S),
                i.key_down(egui::Key::Space)
                    || i.key_down(egui::Key::Z)
                    || i.key_down(egui::Key::W)
                    || i.key_down(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Space)
                    || i.key_pressed(egui::Key::Z)
                    || i.key_pressed(egui::Key::W)
                    || i.key_pressed(egui::Key::ArrowUp),
                i.key_down(egui::Key::X) || i.modifiers.shift,
            )
        });
        let jump_edge = jump_key || self.jump_edge;
        self.jump_edge = false;
        let input = Input {
            left: left || self.hold_left,
            right: right || self.hold_right,
            jump_held: jump_held || self.hold_jump,
            jump_pressed: jump_edge,
            run_held: run_held || self.hold_run,
            up: up || self.hold_up,
            down: down || self.hold_down,
        };
        let preset = self.project.character_physics;
        let power = self.project.power_up_style;
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
        let (jumped, stomped, hurt, goal, level_id, warp) = {
            let play = self.play.as_mut().unwrap();
            let mut jumped = false;
            let mut stomped = false;
            let mut hurt = false;
            let mut goal = false;
            let frame = 1.0 / 60.0;
            play.accum += dt;
            let mut ticks = 0;
            while play.accum >= frame && ticks < 8 {
                play.accum -= frame;
                ticks += 1;
                let mut step_input = input;
                step_input.jump_pressed = input.jump_pressed && !jumped;
                if let Some(sim) = play.sim.as_mut() {
                    sim.tick_play(preset, power, &step_input);
                    jumped |= sim.events.jump;
                    stomped |= sim.events.stomp;
                    hurt |= sim.events.hurt;
                    goal |= sim.events.goal;
                }
            }
            if play.accum > frame {
                play.accum %= frame;
            }
            let warp = play.sim.as_mut().and_then(|sim| sim.take_warp());
            (jumped, stomped, hurt, goal, play.level_id.clone(), warp)
        };
        if let Some(arrival) = warp {
            match resolve_warp_arrival(
                &self.project,
                &level_id,
                &arrival.level_id,
                arrival.x,
                arrival.y,
            ) {
                Ok((dest, dx, dy)) => {
                    if dest != level_id {
                        self.start_level(dest, true);
                    }
                    if let Some(sim) = self.play.as_mut().and_then(|play| play.sim.as_mut()) {
                        sim.place_at_cell_facing(dx, dy, arrival.facing);
                    }
                }
                Err(err) => self.status = err,
            }
        }
        if jumped {
            self.cue("jump");
        }
        if stomped {
            self.cue("stomp");
        }
        if hurt {
            self.cue("hurt");
        }
        if goal {
            self.cue("goal");
            let name = self
                .project
                .level(&level_id)
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Course".into());
            if let Some(play) = self.play.as_mut() {
                play.cleared.insert(level_id);
                play.message = format!("{name} cleared. Linked paths are open.");
                play.sim = None;
                play.from_map = true;
            }
            self.stop_music();
        }
    }
}

impl eframe::App for NestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        if self.godot_busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.tab == Tab::Play {
            self.step_play(ctx);
        }
        self.ensure_textures(ctx);
        egui::TopBottomPanel::top("bar").show(ctx, |ui| self.top_bar(ui));
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(RichText::new(&self.status).color(paper()));
        });
        egui::SidePanel::right("assist")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| self.assist(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            self.tabs(ui);
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Cartridge => self.cartridge(ui),
                Tab::Tiles => self.tiles(ui, ctx),
                Tab::Audio => self.audio(ui),
                Tab::Notes => self.notes(ui),
                Tab::Library => self.library(ui),
                Tab::Courses => self.courses(ui),
                Tab::Overworld => self.overworld(ui),
                Tab::Play => self.play_view(ui),
                Tab::Export => self.export_tab(ui),
                Tab::Guide => self.guide(ui),
            });
        });
        self.keys_window(ctx);
        self.image_window(ctx);
    }
}

impl NestApp {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::Image::from_texture(&self.brand_icon)
                    .fit_to_exact_size(Vec2::splat(28.0))
                    .texture_options(TextureOptions::NEAREST)
                    .alt_text("Nesticles"),
            );
            ui.label(RichText::new("Nesticles").font(serif(30.0)).color(ember()));
            ui.label(
                RichText::new("local workshop")
                    .font(serif(16.0))
                    .color(paper()),
            );
            if ui.button("Demo cartridge").clicked() {
                self.load_rom_bytes("seed-spirit.nes", demo_rom());
            }
            if ui.button("API keys").clicked() {
                self.keys_open = true;
            }
            if ui.button("Image maker").clicked() {
                self.image_open = true;
            }
            if ui.button("Play").clicked() {
                self.start_level(self.current_level_id(), false);
                self.tab = Tab::Play;
            }
        });
    }

    fn tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                (Tab::Cartridge, "Cartridge"),
                (Tab::Tiles, "Tiles"),
                (Tab::Audio, "Audio"),
                (Tab::Notes, "Notes"),
                (Tab::Library, "Library"),
                (Tab::Courses, "Courses"),
                (Tab::Overworld, "Overworld"),
                (Tab::Play, "Play"),
                (Tab::Export, "Export"),
                (Tab::Guide, "Guide"),
            ] {
                if ui.selectable_label(self.tab == tab, label).clicked() {
                    if self.tab == Tab::Play && tab != Tab::Play {
                        self.stop_music();
                    }
                    self.tab = tab;
                }
            }
        });
    }

    fn cartridge(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Cartridge").font(serif(26.0)));
        ui.label("Open an NES, Game Boy, Game Boy Color, Super NES, or Game Boy Advance image you supply. Nesticles keeps tiles and sounds only, as parts you can drag into a course. Each file becomes its own style. A second game adds a style and does not replace the first. Files over 64 MB are refused. Encrypted Switch, 3DS, and Wii U images are not decrypted. Unknown files still open as a tile pager.");
        ui.horizontal(|ui| {
            if ui.button("Open file…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Images and data",
                        &["nes", "gb", "gbc", "gba", "sfc", "smc", "bin", "rom"],
                    )
                    .pick_file()
                {
                    self.rom_path = path.display().to_string();
                    self.load_rom_path();
                }
            }
            if ui.button("Load demo cartridge").clicked() {
                self.load_rom_bytes("seed-spirit.nes", demo_rom());
            }
        });
        ui.horizontal(|ui| {
            ui.label("Path");
            ui.add(egui::TextEdit::singleline(&mut self.rom_path).desired_width(420.0));
            if ui.button("Load path").clicked() {
                self.load_rom_path();
            }
        });
        let Some(rom) = self.rom.as_ref() else {
            ui.label("No cartridge yet. The Seed Spirit style under Courses is already playable.");
            return;
        };
        let file_name = rom.name.clone();
        let title = rom.info.title.clone();
        let details = rom.info.details.clone();
        let warnings = rom.info.warnings.clone();
        ui.label(format!("File: {file_name}"));
        ui.label(RichText::new(&title).font(serif(20.0)).color(ember()));
        for (label, value) in &details {
            ui.horizontal(|ui| {
                ui.label(RichText::new(label).color(ember()));
                ui.label(value);
            });
        }
        for warning in &warnings {
            ui.label(RichText::new(warning).color(Color32::from_rgb(232, 196, 150)));
        }
        if let Some(style) = self
            .project
            .styles
            .iter()
            .find(|style| style.source == file_name)
        {
            ui.label(format!(
                "Style \"{}\" holds parts from this file. Styles already in the project stay in the tray.",
                style.name
            ));
        }
    }

    fn tiles(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(RichText::new("Tile pager").font(serif(26.0)));
        ui.label("Uncompressed tiles, nearest-neighbor. A suggestion is only a guess. You choose the tag, because a cartridge does not label player or enemy.");
        if self.rom.is_none() {
            ui.label("Open a cartridge or the demo image first.");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("codec")
                .selected_text(self.codec.label())
                .show_ui(ui, |ui| {
                    for codec in [
                        Codec::Nes,
                        Codec::Gb,
                        Codec::Snes,
                        Codec::Gba,
                        Codec::OneBpp,
                    ] {
                        ui.selectable_value(&mut self.codec, codec, codec.label());
                    }
                });
            ui.label("Offset hex");
            ui.add(egui::TextEdit::singleline(&mut self.tile_offset_text).desired_width(90.0));
            if ui.button("Apply offset").clicked() {
                self.page = 0;
            }
            ui.add(egui::Slider::new(&mut self.columns, 4..=32).text("columns"));
            ui.add(egui::Slider::new(&mut self.rows, 2..=24).text("rows"));
            ui.checkbox(&mut self.transparent_zero, "Index 0 clear");
            if ui.button("Previous").clicked() && self.page > 0 {
                self.page -= 1;
            }
            if ui.button("Next").clicked() {
                self.page += 1;
            }
            ui.label(format!("page {}", self.page + 1));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Palette");
            for color in &mut self.palette {
                ui.color_edit_button_srgb(color);
            }
            if ui.button("Reset palette").clicked() {
                self.palette = default_palette(self.codec);
            }
        });
        let offset = parse_hex_or_dec(&self.tile_offset_text);
        let bytes_per = tile_bytes(self.codec);
        let per_page = self.columns.saturating_mul(self.rows).max(1);
        let page_origin =
            offset.saturating_add(self.page.saturating_mul(per_page).saturating_mul(bytes_per));
        self.draw_page(ui, ctx, page_origin, per_page);
        ui.horizontal_wrapped(|ui| {
            ui.label("Stamp");
            ui.selectable_value(&mut self.stamp, 1, "1×1");
            ui.selectable_value(&mut self.stamp, 2, "2×2");
            egui::ComboBox::from_id_salt("tag")
                .selected_text(self.tag.label())
                .show_ui(ui, |ui| {
                    for kind in visual_kinds() {
                        ui.selectable_value(&mut self.tag, kind, kind.label());
                    }
                });
            if ui.button("Add as part").clicked() {
                if let Some((w, h, rgba, png)) = self.grab_stamp(page_origin) {
                    let style = self.project.style_name(&self.active_style);
                    self.add_rgba_asset("Tile", self.tag, w, h, rgba, png);
                    self.status =
                        format!("Added a part to style {style}. Drag it from the course tray.");
                }
            }
            if ui.button("Export PNG").clicked() {
                self.export_stamp_png(page_origin);
            }
            if ui.button("Scan candidates").clicked() {
                if let Some(rom) = &self.rom {
                    self.candidates = scan_tiles(&rom.bytes, self.codec, offset, 4096);
                    self.status = format!("{} unique candidates", self.candidates.len());
                }
            }
        });
        if let Some(sel) = self.selected {
            if let Some(indices) = self.tile_indices(page_origin, sel) {
                let guess = suggest_kind(&indices, 8, 8);
                ui.label(format!(
                    "Selected tile {sel} at ${:X}. Suggestion: {}.",
                    page_origin + sel * bytes_per,
                    guess.label()
                ));
            }
        }
        ui.label(RichText::new("Candidates").font(serif(18.0)));
        let mut jump = None;
        for candidate in &self.candidates {
            ui.horizontal(|ui| {
                if ui
                    .button(format!(
                        "${:X} {}",
                        candidate.offset,
                        candidate.suggestion.label()
                    ))
                    .clicked()
                {
                    jump = Some(candidate.offset);
                }
            });
        }
        if let Some(off) = jump {
            self.tile_offset_text = format!("{off:X}");
            self.page = 0;
            self.selected = Some(0);
        }
    }

    fn draw_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        page_origin: usize,
        per_page: usize,
    ) {
        let Some(rom) = &self.rom else { return };
        let mut indices = vec![0u8; per_page * 64];
        for i in 0..per_page {
            let off = page_origin + i * tile_bytes(self.codec);
            if let Some(tile) = decode_tile(&rom.bytes, off, self.codec) {
                indices[i * 64..(i + 1) * 64].copy_from_slice(&tile);
            }
        }
        let (w, h, rgba) = indices_to_rgba(
            &indices,
            self.columns.max(1),
            self.rows.max(1),
            &self.palette,
            self.transparent_zero,
        );
        let sig = page_sig(
            page_origin,
            self.codec,
            &self.palette,
            self.columns,
            self.rows,
            self.transparent_zero,
            rgba.len(),
        );
        if self.page_tex.as_ref().map(|(s, _)| *s) != Some(sig)
            && w > 0
            && h > 0
            && rgba.len() == (w * h * 4) as usize
        {
            let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let tex = ctx.load_texture(format!("page-{sig}"), image, TextureOptions::NEAREST);
            self.page_tex = Some((sig, tex));
        }
        let scale = 3.0;
        if let Some((_, tex)) = &self.page_tex {
            let size = Vec2::new(w as f32 * scale, h as f32 * scale);
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            ui.painter().image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local = pos - rect.min;
                    let tx = (local.x / (8.0 * scale)) as usize;
                    let ty = (local.y / (8.0 * scale)) as usize;
                    if tx < self.columns && ty < self.rows {
                        self.selected = Some(ty * self.columns + tx);
                    }
                }
            }
        }
    }

    fn audio(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Audio").font(serif(26.0)));
        ui.label("NES DPCM, Game Boy wave tables, and Super NES BRR. Preview, then keep a clip as a sound or as music. PCM WAV import is built in. Ogg and MP3 are read when the decoder can open them.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Scan DPCM").clicked() {
                self.scan_audio("dpcm");
            }
            if ui.button("Scan waves").clicked() {
                self.scan_audio("wave");
            }
            if ui.button("Scan BRR").clicked() {
                self.scan_audio("brr");
            }
            if ui.button("Import wav/ogg/mp3…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio", &["wav", "ogg", "mp3"])
                    .pick_file()
                {
                    self.import_audio_path(&path);
                }
            }
        });
        let mut action = None;
        for (index, hit) in self.hits.iter().enumerate() {
            ui.group(|ui| {
                ui.label(format!("{}  score {:.2}", hit.label, hit.score));
                waveform(ui, &hit.samples);
                ui.horizontal(|ui| {
                    if ui.button("Preview").clicked() {
                        action = Some(("preview", index));
                    }
                    if ui.button("Add as sound").clicked() {
                        action = Some(("sfx", index));
                    }
                    if ui.button("Add as music").clicked() {
                        action = Some(("music", index));
                    }
                });
            });
        }
        if let Some((kind, index)) = action {
            if kind == "preview" {
                let wav = wav_from_samples(&self.hits[index].samples, self.hits[index].rate);
                let _ = audio_out::preview_wav(&wav);
            } else {
                let hit = &self.hits[index];
                let wav = wav_from_samples(&hit.samples, hit.rate);
                let asset_kind = if kind == "music" {
                    AssetKind::Music
                } else {
                    AssetKind::Sfx
                };
                self.project.assets.push(Asset {
                    id: nid(if kind == "music" { "music-" } else { "sfx-" }),
                    name: hit.label.clone(),
                    kind: asset_kind,
                    style_id: self.active_style.clone(),
                    w: 0,
                    h: 0,
                    rgba_base64: String::new(),
                    png_base64: String::new(),
                    wav_base64: b64_encode(&wav),
                    solid: false,
                    cue: String::new(),
                    item_tag: nesticles_core::ItemTag::Auto,
                    block_tag: nesticles_core::BlockTag::Auto,
                    hazard_tag: nesticles_core::HazardTag::Auto,
                    warp: None,
                });
                self.status = format!("Added {} to the library.", hit.label);
            }
        }
    }

    fn notes(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Notes").font(serif(26.0)));
        ui.label("Scripts are custom per game, so Nesticles does not extract them. What you get here is ASCII text, plus a linear 6502 listing for NES program bytes. The listing will misread data as code. Physics is not extracted. The player you test in Play is an original engine.");
        ui.horizontal(|ui| {
            ui.label("File offset hex");
            ui.add(egui::TextEdit::singleline(&mut self.disasm_index_text).desired_width(80.0));
            ui.label("CPU address hex");
            ui.add(egui::TextEdit::singleline(&mut self.disasm_base_text).desired_width(80.0));
            if ui.button("Disassemble").clicked() {
                self.refresh_disasm();
            }
        });
        for line in &self.disasm {
            ui.monospace(format!(
                "{:04X}  {:<12} {}",
                line.address, line.bytes, line.text
            ));
        }
        ui.separator();
        ui.label(RichText::new("Strings").font(serif(18.0)));
        for (off, text) in &self.strings {
            ui.monospace(format!("${off:X}  {text}"));
        }
    }

    fn library(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Library").font(serif(26.0)));
        ui.label("Parts are grouped by style. A style is one cartridge, or the original Seed Spirit set. Importing another game adds a style.");
        let mut remove = None;
        let ids: Vec<String> = self.project.assets.iter().map(|a| a.id.clone()).collect();
        for id in ids {
            let Some(asset) = self.project.asset(&id) else {
                continue;
            };
            let selected = self.lib_sel.as_deref() == Some(id.as_str());
            let style = self.project.style_name(&asset.style_id);
            let label = format!("{}  {}  ·  {style}", asset.name, asset.kind.label());
            ui.horizontal(|ui| {
                if ui.selectable_label(selected, label).clicked() {
                    self.lib_sel = Some(id.clone());
                    self.active_style = self
                        .project
                        .asset(&id)
                        .map(|item| item.style_id.clone())
                        .unwrap_or_default();
                }
            });
        }
        if let Some(id) = self.lib_sel.clone() {
            if let Some(asset) = self.project.asset_mut(&id) {
                ui.separator();
                ui.label(RichText::new("Selected").font(serif(18.0)));
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut asset.name);
                });
                let mut kind = asset.kind;
                egui::ComboBox::from_id_salt("lib-kind")
                    .selected_text(kind.label())
                    .show_ui(ui, |ui| {
                        for item in AssetKind::ALL {
                            ui.selectable_value(&mut kind, item, item.label());
                        }
                    });
                if kind != asset.kind {
                    asset.kind = kind;
                    asset.solid = kind.default_solid();
                }
                ui.label(format!("{}×{}", asset.w, asset.h));
            }
            if let Some(tex) = self.textures.get(&id) {
                let size = self
                    .project
                    .asset(&id)
                    .map(|a| Vec2::new(a.w as f32 * 4.0, a.h as f32 * 4.0))
                    .unwrap_or(Vec2::new(64.0, 64.0));
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                ui.painter().image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.horizontal(|ui| {
                if ui.button("Use as paint").clicked() {
                    self.paint_asset = Some(id.clone());
                    self.tab = Tab::Courses;
                }
                if ui.button("Export PNG").clicked() {
                    self.export_asset_png(&id);
                }
                if ui.button("Preview sound").clicked() {
                    self.preview_asset_wav(&id);
                }
                if ui.button("Remove").clicked() {
                    remove = Some(id.clone());
                }
            });
        }
        if let Some(id) = remove {
            self.project.assets.retain(|a| a.id != id);
            self.textures.remove(&id);
            self.lib_sel = None;
        }
    }

    fn courses(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Courses").font(serif(26.0)));
        ui.label("Drag a part from the tray onto the grid, or select it and paint. The course has a home style. All styles stays available, so parts from more than one game can sit on the same course.");
        if self.project.levels.is_empty() {
            ui.label("No courses yet.");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for (index, level) in self.project.levels.iter().enumerate() {
                if ui
                    .selectable_label(self.level_sel == index, &level.name)
                    .clicked()
                {
                    self.level_sel = index;
                }
            }
            if ui.button("New course").clicked() {
                let id = nid("course-");
                let style_id = if self.active_style.is_empty() {
                    self.project
                        .styles
                        .first()
                        .map(|style| style.id.clone())
                        .unwrap_or_default()
                } else {
                    self.active_style.clone()
                };
                self.project.levels.push(nesticles_core::Level {
                    id,
                    name: "New course".into(),
                    style_id,
                    w: 32,
                    h: 15,
                    sky: "#24343c".into(),
                    music_id: None,
                    player: nesticles_core::project::Spawn { x: 2.0, y: 2.0 },
                    cells: Vec::new(),
                    enemy_ai: EnemyAiStyle::Smb1Nes,
                    entrances: Vec::new(),
                });
                self.level_sel = self.project.levels.len() - 1;
            }
        });
        let level_id = self.project.levels[self.level_sel].id.clone();
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.project.levels[self.level_sel].name);
            if ui.button("Play this course").clicked() {
                self.start_level(level_id.clone(), false);
                self.tab = Tab::Play;
            }
        });
        if let Ok(audit) = audit_course(&self.project, &level_id) {
            ui.label(audit);
        }
        {
            let styles = self.project.styles.clone();
            let mut style_id = self.project.levels[self.level_sel].style_id.clone();
            ui.horizontal(|ui| {
                ui.label("Home style");
                egui::ComboBox::from_id_salt("course-style")
                    .selected_text(self.project.style_name(&style_id))
                    .show_ui(ui, |ui| {
                        for style in &styles {
                            ui.selectable_value(&mut style_id, style.id.clone(), &style.name);
                        }
                    });
            });
            self.project.levels[self.level_sel].style_id = style_id;
        }
        self.physics_dropdown(ui);
        self.powerup_dropdown(ui);
        self.enemy_ai_dropdown(ui, self.level_sel, true);
        self.gameplay_controls(ui);
        self.entrance_editor(ui, &level_id);
        self.parts_tray(ui);
        ui.label("Right click erases. Dragging Pip, or any player part, moves the start.");
        self.paint_grid(ui);
    }

    fn parts_tray(&mut self, ui: &mut egui::Ui) {
        let course_style = self
            .project
            .levels
            .get(self.level_sel)
            .map(|level| level.style_id.clone())
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.label("Tray");
            if ui
                .selectable_label(self.tray_filter == "course", "This style")
                .clicked()
            {
                self.tray_filter = "course".into();
            }
            if ui
                .selectable_label(self.tray_filter.is_empty(), "All styles")
                .clicked()
            {
                self.tray_filter.clear();
            }
            let styles = self.project.styles.clone();
            for style in styles {
                if ui
                    .selectable_label(self.tray_filter == style.id, &style.name)
                    .clicked()
                {
                    self.tray_filter = style.id.clone();
                }
            }
        });
        let selected_style = if self.tray_filter == "course" {
            course_style
        } else {
            self.tray_filter.clone()
        };
        let show_all = self.tray_filter.is_empty() || selected_style.is_empty();
        const GROUPS: [(&str, &[AssetKind]); 7] = [
            ("Terrain", &[AssetKind::Terrain]),
            ("Items", &[AssetKind::Item]),
            ("Characters", &[AssetKind::Player, AssetKind::Enemy]),
            ("Hazards", &[AssetKind::Hazard]),
            ("Goals", &[AssetKind::Goal]),
            ("Scenery", &[AssetKind::Background, AssetKind::Foreground]),
            ("Sound", &[AssetKind::Music, AssetKind::Sfx]),
        ];
        for (label, kinds) in GROUPS {
            let parts: Vec<String> = self
                .project
                .assets
                .iter()
                .filter(|asset| {
                    kinds.contains(&asset.kind)
                        && (show_all
                            || asset.style_id == selected_style
                            || asset.style_id.is_empty())
                })
                .map(|asset| asset.id.clone())
                .collect();
            if parts.is_empty() {
                continue;
            }
            ui.label(RichText::new(label).color(ember()));
            ui.horizontal_wrapped(|ui| {
                for id in parts {
                    self.part_chip(ui, &id);
                }
            });
        }
    }

    fn part_chip(&mut self, ui: &mut egui::Ui, id: &str) {
        let Some(asset) = self.project.asset(id) else {
            return;
        };
        let name = asset.name.clone();
        let kind = asset.kind;
        let asset_id = asset.id.clone();
        let on = self.paint_asset.as_deref() == Some(asset_id.as_str());
        let drag_id = egui::Id::new(("part", asset_id.clone()));
        let chip = ui.dnd_drag_source(drag_id, asset_id.clone(), |ui| {
            ui.horizontal(|ui| {
                if let Some(tex) = self.textures.get(&asset_id) {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                    ui.painter().image(
                        tex.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                ui.selectable_label(on, name)
            })
            .inner
        });
        if chip.inner.clicked() {
            self.paint_asset = Some(asset_id);
            self.paint_start = kind == AssetKind::Player;
        }
    }

    fn paint_grid(&mut self, ui: &mut egui::Ui) {
        if self.level_sel >= self.project.levels.len() {
            return;
        }
        let cell = 20.0;
        let w = self.project.levels[self.level_sel].w;
        let h = self.project.levels[self.level_sel].h;
        let size = Vec2::new(w as f32 * cell, h as f32 * cell);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let sky = parse_hex_color(&self.project.levels[self.level_sel].sky);
        painter.rect_filled(rect, 0.0, sky);
        let level = &self.project.levels[self.level_sel];
        for cell_ref in &level.cells {
            let color = self
                .project
                .asset(&cell_ref.asset_id)
                .map(|a| kind_color(a.kind))
                .unwrap_or(Color32::GRAY);
            let r = Rect::from_min_size(
                rect.min + Vec2::new(cell_ref.x as f32 * cell, cell_ref.y as f32 * cell),
                Vec2::splat(cell - 1.0),
            );
            if let Some(tex) = self.textures.get(&cell_ref.asset_id) {
                painter.image(
                    tex.id(),
                    r,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                painter.rect_filled(r, 1.0, color);
            }
        }
        let player = level.player;
        let pr = Rect::from_min_size(
            rect.min + Vec2::new(player.x * cell, player.y * cell),
            Vec2::splat(cell - 1.0),
        );
        painter.rect_stroke(
            pr,
            2.0,
            egui::Stroke::new(2.0_f32, ember()),
            egui::StrokeKind::Inside,
        );
        if self.edit_placement {
            if let Some((x, y)) = self.placement_pick {
                let picked = Rect::from_min_size(
                    rect.min + Vec2::new(x as f32 * cell, y as f32 * cell),
                    Vec2::splat(cell - 1.0),
                );
                painter.rect_stroke(
                    picked,
                    1.0,
                    egui::Stroke::new(2.0_f32, paper()),
                    egui::StrokeKind::Inside,
                );
            }
        }
        if let Some(payload) = response.dnd_release_payload::<String>() {
            if let Some(pos) = response
                .interact_pointer_pos()
                .or(ui.input(|i| i.pointer.interact_pos()))
            {
                let local = pos - rect.min;
                self.place_at(
                    (local.x / cell) as i32,
                    (local.y / cell) as i32,
                    (*payload).clone(),
                    false,
                );
            }
        } else if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let local = pos - rect.min;
                let x = (local.x / cell) as i32;
                let y = (local.y / cell) as i32;
                let erase = ui.input(|i| i.pointer.secondary_down());
                if self.edit_placement && !erase {
                    if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                        let occupied = self.project.levels[self.level_sel]
                            .cells
                            .iter()
                            .any(|placed| placed.x == x && placed.y == y);
                        if occupied {
                            self.placement_pick = Some((x, y));
                        } else {
                            self.placement_pick = None;
                            self.status = format!("No part at {x},{y}.");
                        }
                    }
                } else if self.paint_start && !erase {
                    if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                        self.project.levels[self.level_sel].player.x = x as f32;
                        self.project.levels[self.level_sel].player.y = y as f32;
                    }
                } else if let Some(asset_id) = self.paint_asset.clone() {
                    self.place_at(x, y, asset_id, erase);
                } else if erase {
                    self.place_at(x, y, String::new(), true);
                }
            }
        }
    }

    fn place_at(&mut self, x: i32, y: i32, asset_id: String, erase: bool) {
        if self.level_sel >= self.project.levels.len() {
            return;
        }
        let w = self.project.levels[self.level_sel].w as i32;
        let h = self.project.levels[self.level_sel].h as i32;
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        if erase {
            self.project.levels[self.level_sel]
                .cells
                .retain(|cell| !(cell.x == x && cell.y == y));
            return;
        }
        let Some(asset) = self.project.asset(&asset_id).cloned() else {
            return;
        };
        if asset.kind == AssetKind::Player {
            self.project.levels[self.level_sel].player.x = x as f32;
            self.project.levels[self.level_sel].player.y = y as f32;
            return;
        }
        if asset.kind == AssetKind::Music {
            self.project.levels[self.level_sel].music_id = Some(asset_id);
            return;
        }
        if asset.kind.is_audio() {
            return;
        }
        let layer = asset.kind.default_layer();
        let level = &mut self.project.levels[self.level_sel];
        level
            .cells
            .retain(|cell| !(cell.x == x && cell.y == y && cell.layer == layer));
        level.cells.push(Cell {
            x,
            y,
            asset_id,
            layer,
            ..Cell::default()
        });
        let role = gameplay_options()
            .into_iter()
            .nth(self.brush_gameplay)
            .map(|(_, role)| role)
            .unwrap_or(nesticles_core::LegacyRole::Inert);
        let level_id = self.project.levels[self.level_sel].id.clone();
        if let Err(err) = assign_cell(&mut self.project, &level_id, x, y, role) {
            self.status = err;
        }
    }

    fn gameplay_controls(&mut self, ui: &mut egui::Ui) {
        if self.level_sel >= self.project.levels.len() {
            return;
        }
        let options = gameplay_options();
        let level_id = self.project.levels[self.level_sel].id.clone();
        if let Some((x, y)) = self.placement_pick {
            let still_there = self.project.levels[self.level_sel]
                .cells
                .iter()
                .any(|cell| cell.x == x && cell.y == y);
            if !still_there {
                self.placement_pick = None;
            }
        }
        let mut chosen: Option<usize> = None;
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.edit_placement, "Edit a placed part");
            let caption = if self.edit_placement {
                "Selected gameplay"
            } else {
                "Gameplay for new stamps"
            };
            ui.label(caption);
            let selected = if self.edit_placement {
                self.placement_pick
                    .and_then(|(x, y)| placement_label(&self.project, &level_id, x, y))
                    .unwrap_or_else(|| "Click a part on the grid".into())
            } else {
                options
                    .get(self.brush_gameplay)
                    .map(|(label, _)| (*label).to_string())
                    .unwrap_or_else(|| "Decoration".into())
            };
            egui::ComboBox::from_id_salt("placement-gameplay")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for (index, (label, _)) in options.iter().enumerate() {
                        if ui.selectable_label(false, *label).clicked() {
                            chosen = Some(index);
                        }
                    }
                });
        });
        ui.label(
            "The picture stays a picture. This menu is the only thing that gives it gameplay.",
        );
        if let Some(index) = chosen {
            if self.edit_placement {
                let Some((x, y)) = self.placement_pick else {
                    self.status = "Click a placed part first.".into();
                    return;
                };
                let role = options[index].1.clone();
                let label = options[index].0;
                match assign_cell(&mut self.project, &level_id, x, y, role) {
                    Ok(_) => self.status = format!("Gameplay set to {label}."),
                    Err(err) => self.status = err,
                }
            } else {
                self.brush_gameplay = index;
                self.status = format!("New stamps use {}.", options[index].0);
            }
        }
        if self.edit_placement {
            if let Some((x, y)) = self.placement_pick {
                self.fixture_editor(ui, &level_id, x, y);
            }
        }
    }

    fn fixture_editor(&mut self, ui: &mut egui::Ui, level_id: &str, x: i32, y: i32) {
        let parts = match placed_fixture(&self.project, level_id, x, y) {
            Ok(Some(parts)) => parts,
            Ok(None) => {
                ui.label(
                    "Fixture properties show up when this part is a block, item, hazard, or pipe.",
                );
                return;
            }
            Err(err) => {
                self.status = err;
                return;
            }
        };
        ui.label(RichText::new("Fixture properties").color(ember()));
        let blocks = [
            (None, "None"),
            (Some(LegacyBlock::Solid), "Solid"),
            (Some(LegacyBlock::Question), "Question block"),
            (Some(LegacyBlock::Brick), "Brick"),
            (Some(LegacyBlock::Coin), "Coin block"),
        ];
        let items = [
            (None, "None"),
            (Some(LegacyItem::Mushroom), "Mushroom"),
            (Some(LegacyItem::OneUp), "1-Up"),
            (Some(LegacyItem::Flower), "Fire flower"),
            (Some(LegacyItem::Star), "Star"),
            (Some(LegacyItem::Plain), "Item"),
            (Some(LegacyItem::Coin), "Coin"),
        ];
        let hazards = [
            (None, "None"),
            (Some(LegacyHazard::Contact), "Contact"),
            (Some(LegacyHazard::Spring), "Spring"),
            (Some(LegacyHazard::Vine), "Vine"),
            (Some(LegacyHazard::FireBar { balls: 6 }), "Fire bar"),
            (Some(LegacyHazard::FireBar { balls: 12 }), "Long fire bar"),
            (Some(LegacyHazard::PipeDown), "Pipe down"),
            (Some(LegacyHazard::PipeUp), "Pipe up"),
            (Some(LegacyHazard::PipeLeft), "Pipe left"),
            (Some(LegacyHazard::PipeRight), "Pipe right"),
        ];
        let mut next_block = None;
        let mut next_item = None;
        let mut next_hazard = None;
        ui.horizontal_wrapped(|ui| {
            ui.label("Block");
            let current = blocks
                .iter()
                .find(|(kind, _)| *kind == parts.block)
                .map(|(_, label)| *label)
                .unwrap_or("None");
            egui::ComboBox::from_id_salt("fixture-block")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for (kind, label) in blocks {
                        if ui.selectable_label(parts.block == kind, label).clicked() {
                            next_block = Some(kind);
                        }
                    }
                });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Contains");
            let current = items
                .iter()
                .find(|(kind, _)| *kind == parts.item)
                .map(|(_, label)| *label)
                .unwrap_or("None");
            egui::ComboBox::from_id_salt("fixture-item")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for (kind, label) in items {
                        if ui.selectable_label(parts.item == kind, label).clicked() {
                            next_item = Some(kind);
                        }
                    }
                });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Hazard");
            let current = hazards
                .iter()
                .find(|(kind, _)| *kind == parts.hazard)
                .map(|(_, label)| *label)
                .unwrap_or("None");
            egui::ComboBox::from_id_salt("fixture-hazard")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for (kind, label) in hazards {
                        if ui.selectable_label(parts.hazard == kind, label).clicked() {
                            next_hazard = Some(kind);
                        }
                    }
                });
        });
        let (block, item, hazard) = (
            next_block.unwrap_or(parts.block),
            next_item.unwrap_or(parts.item),
            next_hazard.unwrap_or(parts.hazard),
        );
        if next_block.is_some() || next_item.is_some() || next_hazard.is_some() {
            match compose_fixture(block, item, hazard) {
                Ok(role) => match assign_cell(&mut self.project, level_id, x, y, role) {
                    Ok(_) => self.status = "Fixture updated.".into(),
                    Err(err) => self.status = err,
                },
                Err(err) => self.status = err,
            }
        }
        let hazard = placed_fixture(&self.project, level_id, x, y)
            .ok()
            .flatten()
            .map(|parts| parts.hazard)
            .unwrap_or(parts.hazard);
        let pipe = matches!(
            hazard,
            Some(
                LegacyHazard::PipeDown
                    | LegacyHazard::PipeUp
                    | LegacyHazard::PipeLeft
                    | LegacyHazard::PipeRight
            )
        );
        if !pipe {
            ui.label("A pipe up, down, left, or right can send the player to an entrance.");
            return;
        }
        self.exit_editor(ui, level_id, x, y);
        self.load_warp_draft(level_id, x, y);
        let levels: Vec<(String, String)> = self
            .project
            .levels
            .iter()
            .map(|level| (level.id.clone(), level.name.clone()))
            .collect();
        let mut picked_mode: Option<String> = None;
        ui.horizontal_wrapped(|ui| {
            ui.label("Warp");
            let selected = if self.warp_level_draft == "inherit" {
                "Blueprint destination".to_string()
            } else if self.warp_level_draft == "cleared" {
                "Warp off".to_string()
            } else if self.warp_level_draft == "here" {
                "This course".to_string()
            } else {
                levels
                    .iter()
                    .find(|(id, _)| id == &self.warp_level_draft)
                    .map(|(_, name)| name.clone())
                    .unwrap_or_else(|| self.warp_level_draft.clone())
            };
            egui::ComboBox::from_id_salt("fixture-warp")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            self.warp_level_draft == "inherit",
                            "Blueprint destination",
                        )
                        .clicked()
                    {
                        picked_mode = Some("inherit".into());
                    }
                    if ui
                        .selectable_label(self.warp_level_draft == "here", "This course")
                        .clicked()
                    {
                        picked_mode = Some("here".into());
                    }
                    for (id, name) in &levels {
                        if ui
                            .selectable_label(self.warp_level_draft == *id, name)
                            .clicked()
                        {
                            picked_mode = Some(id.clone());
                        }
                    }
                    if ui
                        .selectable_label(self.warp_level_draft == "cleared", "Warp off")
                        .clicked()
                    {
                        picked_mode = Some("cleared".into());
                    }
                });
            ui.label("X");
            ui.add(egui::TextEdit::singleline(&mut self.warp_x_text).desired_width(48.0));
            ui.label("Y");
            ui.add(egui::TextEdit::singleline(&mut self.warp_y_text).desired_width(48.0));
            if ui.button("Apply warp").clicked() {
                picked_mode = Some(self.warp_level_draft.clone());
            }
        });
        ui.label(
            "A named exit wins over this coordinate warp. Warp off clears both. An empty course means this course.",
        );
        if let Some(mode) = picked_mode {
            self.warp_level_draft = mode.clone();
            if mode == "cleared" {
                self.status = match clear_exit(&mut self.project, level_id, x, y) {
                    Ok(()) => "Exit cleared.".into(),
                    Err(err) => err,
                };
                return;
            }
            let choice = if mode == "inherit" {
                PlacementWarp::Inherit
            } else {
                let Ok(dest_x) = self.warp_x_text.trim().parse::<i32>() else {
                    self.status = "Warp X and Y need whole numbers.".into();
                    return;
                };
                let Ok(dest_y) = self.warp_y_text.trim().parse::<i32>() else {
                    self.status = "Warp X and Y need whole numbers.".into();
                    return;
                };
                PlacementWarp::Destination(WarpTarget {
                    level_id: if mode == "here" { String::new() } else { mode },
                    x: dest_x,
                    y: dest_y,
                })
            };
            match set_placement_warp(&mut self.project, level_id, x, y, choice) {
                Ok(()) => self.status = "Warp updated.".into(),
                Err(err) => self.status = err,
            }
        }
    }

    fn entrance_editor(&mut self, ui: &mut egui::Ui, level_id: &str) {
        ui.label(RichText::new("Entrances").color(ember()));
        ui.label("An entrance is where the player appears. It does not send them anywhere.");
        let snapshot: Vec<(String, String, i32, i32, Facing)> = self
            .project
            .level(level_id)
            .map(|level| {
                level
                    .entrances
                    .iter()
                    .map(|entrance| {
                        (
                            entrance.id.clone(),
                            entrance.name.clone(),
                            entrance.x,
                            entrance.y,
                            entrance.facing,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        if snapshot.is_empty() {
            ui.label("This course has no extra entrances. Play still starts at the course start.");
        }
        let mut remove_id = None;
        let mut facing_edit = None;
        for (id, name, x, y, facing) in &snapshot {
            ui.horizontal_wrapped(|ui| {
                let selected = self.entrance_pick.as_deref() == Some(id.as_str());
                if ui
                    .selectable_label(selected, format!("{name} at {x},{y}"))
                    .clicked()
                {
                    self.entrance_pick = Some(id.clone());
                }
                let face = if *facing == Facing::Left {
                    "Face left"
                } else {
                    "Face right"
                };
                if ui.button(face).clicked() {
                    facing_edit = Some((
                        id.clone(),
                        if *facing == Facing::Left {
                            Facing::Right
                        } else {
                            Facing::Left
                        },
                        name.clone(),
                        *x,
                        *y,
                    ));
                }
                if ui.button("Remove").clicked() {
                    remove_id = Some(id.clone());
                }
            });
        }
        if let Some((id, facing, name, x, y)) = facing_edit {
            self.status =
                match update_entrance(&mut self.project, level_id, &id, &name, x, y, facing) {
                    Ok(()) => "Entrance updated.".into(),
                    Err(err) => err,
                };
        }
        if let Some(id) = remove_id {
            self.status = match remove_entrance(&mut self.project, level_id, &id) {
                Ok(()) => {
                    if self.entrance_pick.as_deref() == Some(id.as_str()) {
                        self.entrance_pick = None;
                    }
                    "Entrance removed.".into()
                }
                Err(err) => err,
            };
        }
        if let Some(id) = self.entrance_pick.clone() {
            let bounds = self
                .project
                .level(level_id)
                .map(|level| {
                    (
                        level.w.saturating_sub(1) as i32,
                        level.h.saturating_sub(1) as i32,
                    )
                })
                .unwrap_or((0, 0));
            if let Some(level) = self.project.level_mut(level_id) {
                if let Some(entrance) = level
                    .entrances
                    .iter_mut()
                    .find(|entrance| entrance.id == id)
                {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(&mut entrance.name).desired_width(140.0));
                        ui.label("X");
                        ui.add(egui::DragValue::new(&mut entrance.x));
                        ui.label("Y");
                        ui.add(egui::DragValue::new(&mut entrance.y));
                    });
                    entrance.x = entrance.x.clamp(0, bounds.0);
                    entrance.y = entrance.y.clamp(0, bounds.1);
                    if entrance.name.trim().is_empty() {
                        entrance.name = "Entrance".into();
                    }
                }
            }
        }
        if ui.button("Turn coordinate warps into entrances").clicked() {
            ensure_legacy_transitions(&mut self.project);
            self.status =
                "Coordinate warps now have entrances. Return trips were not added.".into();
        }
        if ui.button("Add entrance").clicked() {
            let level = self.project.level(level_id);
            let (w, h) = level.map(|level| (level.w, level.h)).unwrap_or((1, 1));
            let x = 2.min(w.saturating_sub(1) as i32);
            let y = 2.min(h.saturating_sub(1) as i32);
            match add_entrance(&mut self.project, level_id, "Entrance", x, y, Facing::Right) {
                Ok(id) => {
                    self.entrance_pick = Some(id);
                    self.status = "Entrance added.".into();
                }
                Err(err) => self.status = err,
            }
        }
    }

    fn exit_editor(&mut self, ui: &mut egui::Ui, level_id: &str, x: i32, y: i32) {
        ui.label(RichText::new("Exit destination").color(ember()));
        let current = self
            .project
            .level(level_id)
            .and_then(|level| {
                level
                    .cells
                    .iter()
                    .rev()
                    .find(|cell| cell.x == x && cell.y == y)
            })
            .and_then(|cell| cell.transition_id.clone());
        let summary = current.as_ref().and_then(|id| {
            self.project
                .transitions
                .iter()
                .find(|link| &link.id == id)
                .map(|link| {
                    let dest = if link.destination_level_id.is_empty() {
                        level_id.to_string()
                    } else {
                        link.destination_level_id.clone()
                    };
                    let name = self
                        .project
                        .level(&dest)
                        .and_then(|level| {
                            level
                                .entrances
                                .iter()
                                .find(|entrance| entrance.id == link.destination_entrance_id)
                        })
                        .map(|entrance| entrance.name.clone())
                        .unwrap_or_else(|| "missing entrance".into());
                    format!("Sends the player to {name}.")
                })
        });
        ui.label(
            summary.unwrap_or_else(|| "No named exit. A coordinate warp can still apply.".into()),
        );
        self.load_exit_draft(level_id, x, y);
        let levels: Vec<(String, String)> = self
            .project
            .levels
            .iter()
            .map(|level| (level.id.clone(), level.name.clone()))
            .collect();
        let entrances: Vec<(String, String)> = self
            .project
            .level(&self.exit_level_draft)
            .map(|level| {
                level
                    .entrances
                    .iter()
                    .map(|entrance| {
                        (
                            entrance.id.clone(),
                            format!("{} at {},{}", entrance.name, entrance.x, entrance.y),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut use_exit = false;
        let mut make_return = false;
        let mut drop_return = false;
        let mut use_coordinates = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Destination level");
            let level_name = levels
                .iter()
                .find(|(id, _)| id == &self.exit_level_draft)
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| "Pick a course".into());
            egui::ComboBox::from_id_salt("exit-level")
                .selected_text(level_name)
                .show_ui(ui, |ui| {
                    for (id, name) in &levels {
                        if ui
                            .selectable_label(self.exit_level_draft == *id, name)
                            .clicked()
                        {
                            self.exit_level_draft = id.clone();
                            self.exit_entrance_draft.clear();
                        }
                    }
                });
            ui.label("Destination entrance");
            let entrance_name = entrances
                .iter()
                .find(|(id, _)| id == &self.exit_entrance_draft)
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| "Pick an entrance".into());
            egui::ComboBox::from_id_salt("exit-entrance")
                .selected_text(entrance_name)
                .show_ui(ui, |ui| {
                    for (id, name) in &entrances {
                        if ui
                            .selectable_label(self.exit_entrance_draft == *id, name)
                            .clicked()
                        {
                            self.exit_entrance_draft = id.clone();
                        }
                    }
                });
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Use this entrance").clicked() {
                use_exit = true;
            }
            if ui.button("Create return connection").clicked() {
                make_return = true;
            }
            if ui.button("Remove return connection").clicked() {
                drop_return = true;
            }
            if ui.button("Use coordinate warp instead").clicked() {
                use_coordinates = true;
            }
        });
        ui.label("One-way is the default. A return connection uses the pipe in front of the destination entrance and comes back to this pipe's opening.");
        if use_exit {
            let entry = pipe_entry(hazard_of(&self.project, level_id, x, y));
            let dest_level = if self.exit_level_draft == level_id {
                String::new()
            } else {
                self.exit_level_draft.clone()
            };
            self.status = match set_exit(
                &mut self.project,
                level_id,
                x,
                y,
                &dest_level,
                &self.exit_entrance_draft,
                entry,
            ) {
                Ok(_) => "Exit updated.".into(),
                Err(err) => err,
            };
        }
        if make_return {
            let Some(forward) = current.clone().or_else(|| {
                self.project
                    .level(level_id)
                    .and_then(|level| {
                        level
                            .cells
                            .iter()
                            .rev()
                            .find(|cell| cell.x == x && cell.y == y)
                    })
                    .and_then(|cell| cell.transition_id.clone())
            }) else {
                self.status = "Set a named exit before adding a return.".into();
                return;
            };
            let forward_link = self
                .project
                .transitions
                .iter()
                .find(|link| link.id == forward)
                .cloned();
            let Some(forward_link) = forward_link else {
                self.status = "That named exit is missing.".into();
                return;
            };
            let dest_level = if forward_link.destination_level_id.is_empty() {
                level_id.to_string()
            } else {
                forward_link.destination_level_id.clone()
            };
            let spot = self.project.level(&dest_level).and_then(|level| {
                level
                    .entrances
                    .iter()
                    .find(|entrance| entrance.id == forward_link.destination_entrance_id)
                    .map(|entrance| (entrance.x, entrance.y))
            });
            let Some((ex, ey)) = spot else {
                self.status = "The destination entrance is missing.".into();
                return;
            };
            let Some((rx, ry, back)) = pipe_at_opening(&self.project, &dest_level, ex, ey) else {
                self.status =
                    "Place a pipe in front of the destination entrance before adding a return."
                        .into();
                return;
            };
            let (hx, hy) = opening_cell(hazard_of(&self.project, level_id, x, y), x, y);
            let home_entrance = self.project.level(level_id).and_then(|level| {
                level
                    .entrances
                    .iter()
                    .find(|entrance| entrance.x == hx && entrance.y == hy)
                    .map(|entrance| entrance.id.clone())
            });
            let home_entrance = if let Some(id) = home_entrance {
                id
            } else {
                match add_entrance(&mut self.project, level_id, "Return", hx, hy, Facing::Right) {
                    Ok(id) => id,
                    Err(err) => {
                        self.status = err;
                        return;
                    }
                }
            };
            self.status =
                match add_return(&mut self.project, &forward, rx, ry, &home_entrance, back) {
                    Ok(_) => "Return connection added.".into(),
                    Err(err) => err,
                };
        }
        if drop_return {
            if let Some(id) = current.clone() {
                self.status = match remove_return(&mut self.project, &id) {
                    Ok(()) => "Return connection removed.".into(),
                    Err(err) => err,
                };
            }
        }
        if use_coordinates {
            self.status = match clear_named_exit(&mut self.project, level_id, x, y) {
                Ok(()) => "Named exit cleared. The coordinate warp applies again.".into(),
                Err(err) => err,
            };
        }
    }

    fn load_exit_draft(&mut self, level_id: &str, x: i32, y: i32) {
        if self
            .exit_draft_for
            .as_ref()
            .map(|(id, cx, cy)| id == level_id && *cx == x && *cy == y)
            == Some(true)
        {
            return;
        }
        self.exit_draft_for = Some((level_id.to_string(), x, y));
        self.exit_level_draft = level_id.to_string();
        self.exit_entrance_draft.clear();
        let transition_id = self.project.level(level_id).and_then(|level| {
            level
                .cells
                .iter()
                .rev()
                .find(|cell| cell.x == x && cell.y == y)
                .and_then(|cell| cell.transition_id.clone())
        });
        if let Some(id) = transition_id {
            if let Some(link) = self.project.transitions.iter().find(|link| link.id == id) {
                self.exit_level_draft = if link.destination_level_id.is_empty() {
                    level_id.to_string()
                } else {
                    link.destination_level_id.clone()
                };
                self.exit_entrance_draft = link.destination_entrance_id.clone();
            }
        }
    }

    fn load_warp_draft(&mut self, level_id: &str, x: i32, y: i32) {
        if self
            .warp_draft_for
            .as_ref()
            .map(|(id, cx, cy)| id == level_id && *cx == x && *cy == y)
            == Some(true)
        {
            return;
        }
        self.warp_draft_for = Some((level_id.to_string(), x, y));
        let choice = self
            .project
            .level(level_id)
            .and_then(|level| {
                level
                    .cells
                    .iter()
                    .rev()
                    .find(|cell| cell.x == x && cell.y == y)
            })
            .map(|cell| cell.warp_choice.clone())
            .unwrap_or(PlacementWarp::Inherit);
        match choice {
            PlacementWarp::Inherit => {
                self.warp_level_draft = "inherit".into();
                self.warp_x_text = "0".into();
                self.warp_y_text = "0".into();
            }
            PlacementWarp::Cleared => {
                self.warp_level_draft = "cleared".into();
            }
            PlacementWarp::Destination(target) => {
                self.warp_level_draft = if target.level_id.is_empty() {
                    "here".into()
                } else {
                    target.level_id
                };
                self.warp_x_text = target.x.to_string();
                self.warp_y_text = target.y.to_string();
            }
        }
    }

    fn overworld(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Overworld").font(serif(26.0)));
        ui.label("Nodes open courses. Clearing a course unlocks the nodes it links to. The start node is open immediately.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Add node for this course").clicked() {
                if let Some(level) = self.project.levels.get(self.level_sel) {
                    let id = nid("node-");
                    let name = level.name.clone();
                    let level_id = level.id.clone();
                    self.project.nodes.push(nesticles_core::project::OverNode {
                        id: id.clone(),
                        name,
                        level_id,
                        x: 80.0 + self.project.nodes.len() as f32 * 40.0,
                        y: 180.0,
                        links: Vec::new(),
                    });
                    if self.project.start_node_id.is_empty() {
                        self.project.start_node_id = id;
                    }
                }
            }
            if ui.button("Set selected as start").clicked() {
                if let Some(node) = self.project.nodes.get(self.node_sel) {
                    self.project.start_node_id = node.id.clone();
                }
            }
            if ui.button("Link selected to next click").clicked() {
                if let Some(node) = self.project.nodes.get(self.node_sel) {
                    self.link_from = Some(node.id.clone());
                    self.status = "Click another node to link them.".into();
                }
            }
            if ui.button("Play from overworld").clicked() {
                self.play = Some(PlayState {
                    from_map: true,
                    level_id: String::new(),
                    sim: None,
                    cleared: self
                        .play
                        .as_ref()
                        .map(|p| p.cleared.clone())
                        .unwrap_or_default(),
                    message: "Choose a path".into(),
                    accum: 0.0,
                    select: 0,
                });
                self.tab = Tab::Play;
            }
        });
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(640.0, 320.0), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 8.0, Color32::from_rgb(22, 18, 16));
        for node in &self.project.nodes {
            let a = rect.min + Vec2::new(node.x, node.y);
            for link in &node.links {
                if let Some(other) = self.project.nodes.iter().find(|n| &n.id == link) {
                    let b = rect.min + Vec2::new(other.x, other.y);
                    painter.line_segment(
                        [a, b],
                        egui::Stroke::new(2.0_f32, Color32::from_rgb(138, 90, 58)),
                    );
                }
            }
        }
        let mut clicked = None;
        for (index, node) in self.project.nodes.iter().enumerate() {
            let center = rect.min + Vec2::new(node.x, node.y);
            let open = node.id == self.project.start_node_id;
            let color = if index == self.node_sel {
                ember()
            } else if open {
                paper()
            } else {
                Color32::from_rgb(90, 70, 60)
            };
            painter.circle_filled(center, 10.0, color);
            painter.text(
                center + Vec2::new(14.0, -6.0),
                egui::Align2::LEFT_TOP,
                &node.name,
                FontId::new(14.0, FontFamily::Name("serif".into())),
                paper(),
            );
            if response.clicked() || response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if pos.distance(center) < 16.0 {
                        clicked = Some(index);
                    }
                }
            }
        }
        if let Some(index) = clicked {
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local = pos - rect.min;
                    self.project.nodes[index].x = local.x;
                    self.project.nodes[index].y = local.y;
                }
            }
            if response.clicked() {
                if let Some(from) = self.link_from.clone() {
                    if let Some(to) = self.project.nodes.get(index).map(|n| n.id.clone()) {
                        if let Some(node) = self.project.nodes.iter_mut().find(|n| n.id == from) {
                            if !node.links.contains(&to) && node.id != to {
                                node.links.push(to);
                            }
                        }
                        self.link_from = None;
                        self.status = "Linked the nodes.".into();
                    }
                }
                self.node_sel = index;
            }
        }
        if let Some(node) = self.project.nodes.get_mut(self.node_sel) {
            ui.horizontal(|ui| {
                ui.label("Node name");
                ui.text_edit_singleline(&mut node.name);
            });
        }
    }

    fn physics_dropdown(&mut self, ui: &mut egui::Ui) {
        let before = self.project.character_physics;
        ui.horizontal(|ui| {
            ui.label("Character Game Style Physics");
            egui::ComboBox::from_id_salt("character-game-style-physics")
                .selected_text(self.project.character_physics.label())
                .show_ui(ui, |ui| {
                    for preset in CharacterPhysics::ALL {
                        ui.selectable_value(
                            &mut self.project.character_physics,
                            preset,
                            preset.label(),
                        );
                    }
                });
        });
        if self.project.character_physics != before {
            if let Some(sim) = self.play.as_mut().and_then(|play| play.sim.as_mut()) {
                sim.bind_character_physics(self.project.character_physics);
            }
        }
    }

    fn powerup_dropdown(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Power Up Styles");
            egui::ComboBox::from_id_salt("power-up-styles")
                .selected_text(self.project.power_up_style.label())
                .show_ui(ui, |ui| {
                    for style in nesticles_core::PowerUpStyle::ALL {
                        ui.selectable_value(&mut self.project.power_up_style, style, style.label());
                    }
                });
        });
    }

    fn enemy_ai_dropdown(&mut self, ui: &mut egui::Ui, level_index: usize, with_enemies: bool) {
        if level_index >= self.project.levels.len() {
            return;
        }
        let enemies: Vec<(i32, i32)> = self.project.levels[level_index]
            .cells
            .iter()
            .filter(|cell| {
                self.project
                    .asset(&cell.asset_id)
                    .map(|asset| asset.kind == AssetKind::Enemy)
                    .unwrap_or(false)
            })
            .map(|cell| (cell.x, cell.y))
            .collect();
        let mut pick = self
            .enemy_pick
            .filter(|cell| enemies.iter().any(|enemy| enemy == cell));
        if !with_enemies {
            pick = None;
        }
        let mut chosen_level: Option<EnemyAiStyle> = None;
        let mut chosen_enemy: Option<Option<EnemyAiStyle>> = None;
        let mut chosen_roster: Option<nesticles_core::EnemyArchetype> = None;
        ui.horizontal(|ui| {
            ui.label("Enemy, Hazards & Collision Rules Style");
            if with_enemies {
                egui::ComboBox::from_id_salt("enemy-ai-target")
                    .selected_text(match pick {
                        Some((x, y)) => format!("Enemy {x},{y}"),
                        None => "This course".to_string(),
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(pick.is_none(), "This course").clicked() {
                            pick = None;
                        }
                        for (x, y) in &enemies {
                            let label = format!("Enemy {x},{y}");
                            if ui.selectable_label(pick == Some((*x, *y)), label).clicked() {
                                pick = Some((*x, *y));
                            }
                        }
                    });
            }
            let selected = if let Some((x, y)) = pick {
                let style = self.project.levels[level_index]
                    .cells
                    .iter()
                    .find(|cell| cell.x == x && cell.y == y)
                    .and_then(|cell| cell.enemy_ai);
                override_label(style)
            } else {
                self.project.levels[level_index].enemy_ai.label()
            };
            egui::ComboBox::from_id_salt(("enemy-ai-style", with_enemies, level_index))
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if pick.is_some() {
                        if ui
                            .selectable_label(
                                selected == "Inherit Level Style",
                                "Inherit Level Style",
                            )
                            .clicked()
                        {
                            chosen_enemy = Some(None);
                        }
                        let smb1 = EnemyAiStyle::Smb1Nes.label();
                        if ui.selectable_label(selected == smb1, smb1).clicked() {
                            chosen_enemy = Some(Some(EnemyAiStyle::Smb1Nes));
                        }
                    } else if ui
                        .selectable_label(
                            selected == EnemyAiStyle::Smb1Nes.label(),
                            EnemyAiStyle::Smb1Nes.label(),
                        )
                        .clicked()
                    {
                        chosen_level = Some(EnemyAiStyle::Smb1Nes);
                    }
                });
        });
        if with_enemies {
            if let Some((x, y)) = pick {
                let current = self.project.levels[level_index]
                    .cells
                    .iter()
                    .find(|cell| cell.x == x && cell.y == y)
                    .map(|cell| cell.archetype.clone());
                let selected = current
                    .as_ref()
                    .map(archetype_label)
                    .unwrap_or_else(|| "Goomba".into());
                ui.horizontal(|ui| {
                    ui.label("Roster");
                    egui::ComboBox::from_id_salt(("enemy-roster", x, y))
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for kind in &SMB1_ROSTER {
                                let label = archetype_label(kind);
                                if ui
                                    .selectable_label(current.as_ref() == Some(kind), label)
                                    .clicked()
                                {
                                    chosen_roster = Some(kind.clone());
                                }
                            }
                        });
                });
            }
            self.enemy_pick = pick;
        }
        if let Some(style) = chosen_level {
            self.project.levels[level_index].enemy_ai = style;
            self.push_enemy_styles(level_index);
        }
        if let Some((x, y)) = pick {
            if let Some(style) = chosen_enemy {
                if let Some(cell) = self.project.levels[level_index]
                    .cells
                    .iter_mut()
                    .find(|cell| cell.x == x && cell.y == y)
                {
                    cell.enemy_ai = style;
                }
                if let Some(sim) = self.play.as_mut().and_then(|play| play.sim.as_mut()) {
                    sim.set_enemy_override(x, y, style);
                }
            }
        }
        if let Some(kind) = chosen_roster {
            if let Some((x, y)) = pick {
                if let Some(enemy) = enemy_from_archetype(&kind) {
                    let level_id = self.project.levels[level_index].id.clone();
                    if let Err(err) =
                        assign_cell(&mut self.project, &level_id, x, y, enemy_role(enemy))
                    {
                        self.status = err;
                    }
                } else if let Some(cell) = self.project.levels[level_index]
                    .cells
                    .iter_mut()
                    .find(|cell| cell.x == x && cell.y == y)
                {
                    cell.archetype = kind;
                }
            }
        }
    }

    fn push_enemy_styles(&mut self, level_index: usize) {
        let Some(level) = self.project.levels.get(level_index) else {
            return;
        };
        let style = level.enemy_ai;
        let id = level.id.clone();
        let global = self.project.enemy_ai;
        if let Some(play) = self.play.as_mut() {
            if play.level_id == id {
                if let Some(sim) = play.sim.as_mut() {
                    sim.set_enemy_styles(style, global);
                }
            }
        }
    }

    fn play_view(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Play").font(serif(26.0)));
        self.godot_preview_ui(ui);
        self.physics_dropdown(ui);
        self.powerup_dropdown(ui);
        let level_index = self
            .play
            .as_ref()
            .and_then(|play| {
                self.project
                    .levels
                    .iter()
                    .position(|level| level.id == play.level_id)
            })
            .unwrap_or(self.level_sel);
        self.enemy_ai_dropdown(ui, level_index, false);
        if self.play.is_none() {
            self.start_level(self.current_level_id(), false);
        }
        let from_map = self
            .play
            .as_ref()
            .map(|p| p.from_map && p.sim.is_none())
            .unwrap_or(false);
        if from_map {
            self.draw_play_map(ui);
        } else {
            self.draw_play_level(ui);
        }
        ui.horizontal(|ui| {
            let left = ui.add(egui::Button::new("Left").min_size(Vec2::new(72.0, 36.0)));
            let right = ui.add(egui::Button::new("Right").min_size(Vec2::new(72.0, 36.0)));
            let up = ui.add(egui::Button::new("Up").min_size(Vec2::new(72.0, 36.0)));
            let down = ui.add(egui::Button::new("Down").min_size(Vec2::new(72.0, 36.0)));
            let jump = ui.add(egui::Button::new("Jump").min_size(Vec2::new(72.0, 36.0)));
            let run = ui.add(egui::Button::new("Run").min_size(Vec2::new(72.0, 36.0)));
            self.hold_left = left.is_pointer_button_down_on();
            self.hold_right = right.is_pointer_button_down_on();
            self.hold_up = up.is_pointer_button_down_on();
            self.hold_down = down.is_pointer_button_down_on();
            let down = jump.is_pointer_button_down_on();
            if down && !self.hold_jump {
                self.jump_edge = true;
            }
            self.hold_jump = down;
            self.hold_run = run.is_pointer_button_down_on();
            if ui.button("Overworld").clicked() {
                let cleared = self
                    .play
                    .as_ref()
                    .map(|p| p.cleared.clone())
                    .unwrap_or_default();
                self.stop_music();
                self.play = Some(PlayState {
                    from_map: true,
                    level_id: String::new(),
                    sim: None,
                    cleared,
                    message: "Choose a path".into(),
                    accum: 0.0,
                    select: 0,
                });
            }
        });
        ui.label("Arrows or A/D move. Jump is Z, W, Up, or Space (the Jump button is A). Run is Left Shift, X, or the Run button.");
        if let Some(play) = &self.play {
            if !play.message.is_empty() {
                ui.label(
                    RichText::new(&play.message)
                        .font(serif(18.0))
                        .color(ember()),
                );
            }
        }
    }

    fn draw_play_map(&mut self, ui: &mut egui::Ui) {
        let unlocked = self.unlocked();
        let (left_edge, right_edge, jump_edge) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D),
                i.key_pressed(egui::Key::Space)
                    || i.key_pressed(egui::Key::Z)
                    || i.key_pressed(egui::Key::ArrowUp)
                    || i.key_pressed(egui::Key::W),
            )
        });
        if let Some(play) = self.play.as_mut() {
            if left_edge {
                play.select = play.select.saturating_sub(1);
            }
            if right_edge && !unlocked.is_empty() {
                play.select = (play.select + 1).min(unlocked.len() - 1);
            }
        }
        let select = self.play.as_ref().map(|p| p.select).unwrap_or(0);
        if (jump_edge || self.jump_edge) && !unlocked.is_empty() {
            self.jump_edge = false;
            let index = select.min(unlocked.len() - 1);
            if let Some(id) = unlocked.get(index) {
                if let Some(node) = self.project.nodes.iter().find(|n| &n.id == id) {
                    let level_id = node.level_id.clone();
                    self.start_level(level_id, true);
                }
            }
        }
        let (rect, response) = ui.allocate_exact_size(Vec2::new(640.0, 300.0), Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, Color32::from_rgb(22, 18, 16));
        for node in &self.project.nodes {
            let a = rect.min + Vec2::new(node.x * 0.9, node.y * 0.75);
            for link in &node.links {
                if let Some(other) = self.project.nodes.iter().find(|n| &n.id == link) {
                    let b = rect.min + Vec2::new(other.x * 0.9, other.y * 0.75);
                    painter.line_segment(
                        [a, b],
                        egui::Stroke::new(2.0_f32, Color32::from_rgb(138, 90, 58)),
                    );
                }
            }
        }
        for (index, id) in unlocked.iter().enumerate() {
            let Some(node) = self.project.nodes.iter().find(|n| &n.id == id) else {
                continue;
            };
            let center = rect.min + Vec2::new(node.x * 0.9, node.y * 0.75);
            let color = if index == select { ember() } else { paper() };
            painter.circle_filled(center, 12.0, color);
            painter.text(
                center + Vec2::new(16.0, -8.0),
                egui::Align2::LEFT_CENTER,
                &node.name,
                FontId::new(16.0, FontFamily::Name("serif".into())),
                paper(),
            );
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if pos.distance(center) < 18.0 {
                        let level_id = node.level_id.clone();
                        self.start_level(level_id, true);
                        return;
                    }
                }
            }
        }
        for node in &self.project.nodes {
            if unlocked.iter().any(|id| id == &node.id) {
                continue;
            }
            let center = rect.min + Vec2::new(node.x * 0.9, node.y * 0.75);
            painter.circle_filled(center, 8.0, Color32::from_rgb(70, 60, 54));
            painter.text(
                center + Vec2::new(14.0, -6.0),
                egui::Align2::LEFT_CENTER,
                &node.name,
                FontId::new(14.0, FontFamily::Name("serif".into())),
                Color32::from_rgb(140, 130, 120),
            );
        }
    }

    fn draw_play_level(&mut self, ui: &mut egui::Ui) {
        let Some(play) = self.play.as_ref() else {
            return;
        };
        let Some(sim) = play.sim.as_ref() else { return };
        let Some(level) = self.project.level(&play.level_id) else {
            return;
        };
        let surface_w = ui.available_width().clamp(320.0, 1920.0);
        let surface_h = (ui.clip_rect().height() - 150.0).clamp(180.0, 1080.0);
        let scale = play_surface_scale(surface_w, surface_h, sim.level_w, sim.level_h);
        let view = Vec2::new(surface_w, surface_h);
        let sky = level.sky.clone();
        let level_name = level.name.clone();
        let level_id = play.level_id.clone();
        let (rect, _) = ui.allocate_exact_size(view, Sense::hover());
        if let Some(sim) = self.play.as_mut().and_then(|play| play.sim.as_mut()) {
            sim.update_view(rect.width() / scale, rect.height() / scale);
        }
        let Some(play) = self.play.as_ref() else {
            return;
        };
        let Some(sim) = play.sim.as_ref() else { return };
        let Some(level) = self.project.level(&level_id) else {
            return;
        };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, parse_hex_color(&sky));
        let cam_x = sim.view.x;
        let cam_y = sim.view.y;
        let mut cells = level.cells.clone();
        cells.sort_by_key(|c| match c.layer {
            Layer::Back => 0,
            Layer::Main => 1,
            Layer::Front => 2,
        });
        for cell in &cells {
            let Some(asset) = self.project.asset(&cell.asset_id) else {
                continue;
            };
            if asset.kind == AssetKind::Enemy {
                continue;
            }
            if sim.hidden.iter().any(|(x, y)| *x == cell.x && *y == cell.y) {
                continue;
            }
            if asset.kind == AssetKind::Item
                && sim.items.iter().any(|(r, alive)| {
                    !*alive
                        && (r.x - cell.x as f32 * TILE).abs() < 1.0
                        && (r.y - cell.y as f32 * TILE).abs() < 1.0
                })
            {
                continue;
            }
            let lift = sim
                .bumps
                .iter()
                .find(|(x, y, _)| *x == cell.x && *y == cell.y)
                .map(|(_, _, offset)| *offset)
                .unwrap_or(0.0);
            self.blit(
                &painter,
                rect.min,
                cam_x,
                cam_y,
                scale,
                &cell.asset_id,
                cell.x as f32 * TILE,
                cell.y as f32 * TILE - lift,
                false,
            );
        }
        for enemy in sim.enemies.iter().filter(|e| e.alive) {
            self.blit(
                &painter,
                rect.min,
                cam_x,
                cam_y,
                scale,
                &enemy.asset_id,
                enemy.x - 1.0,
                enemy.y - 2.0,
                enemy.dir < 0,
            );
        }
        for marker in &sim.markers {
            let p = rect.min + Vec2::new((marker.x - cam_x) * scale, (marker.y - cam_y) * scale);
            let size = Vec2::new(marker.w.max(4.0) * scale, marker.h.max(4.0) * scale);
            painter.rect_filled(
                Rect::from_min_size(p, size),
                2.0,
                Color32::from_rgb(marker.rgb[0], marker.rgb[1], marker.rgb[2]),
            );
        }
        if !sim.hide_player {
            if let Some(player) = self
                .project
                .assets
                .iter()
                .find(|a| a.kind == AssetKind::Player)
            {
                self.blit(
                    &painter,
                    rect.min,
                    cam_x,
                    cam_y,
                    scale,
                    &player.id,
                    sim.player.x - 2.0,
                    sim.player.y - 2.0,
                    sim.player.facing < 0,
                );
            }
        }
        painter.text(
            rect.min + Vec2::new(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            &level_name,
            FontId::new(16.0, FontFamily::Name("serif".into())),
            paper(),
        );
    }

    fn blit(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        cam_x: f32,
        cam_y: f32,
        scale: f32,
        id: &str,
        x: f32,
        y: f32,
        flip: bool,
    ) {
        let p = origin + Vec2::new((x - cam_x) * scale, (y - cam_y) * scale);
        let size = Vec2::splat(TILE * scale);
        let rect = Rect::from_min_size(p, size);
        if let Some(tex) = self.textures.get(id) {
            let uv = if flip {
                Rect::from_min_max(Pos2::new(1.0, 0.0), Pos2::new(0.0, 1.0))
            } else {
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
            };
            painter.image(tex.id(), rect, uv, Color32::WHITE);
        }
    }

    fn export_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Export").font(serif(26.0)));
        self.godot_preview_ui(ui);
        ui.separator();
        ui.label("The desktop app is the editor and the Linux and Windows player. Android play is the browser build: open index.html in Chrome, Firefox, or Edge. Add to Home Screen needs https. A file open is enough to play. This does not build an Android package.");
        if crate::READY {
            ui.label(
                "The wasm player is embedded in this build and will be written into the folder.",
            );
        } else {
            ui.label(RichText::new(crate::FAILURE).color(Color32::from_rgb(232, 180, 120)));
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.export_path).desired_width(360.0));
            if ui.button("Choose folder…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.export_path = path.display().to_string();
                }
            }
            if ui.button("Export game").clicked() {
                match export_game::export_folder(
                    std::path::Path::new(&self.export_path),
                    &self.project,
                ) {
                    Ok(note) => {
                        self.export_note = note.clone();
                        self.status = note;
                    }
                    Err(err) => {
                        self.export_note = err.clone();
                        self.status = err;
                    }
                }
            }
            if ui.button("Save project JSON…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Project", &["json"])
                    .save_file()
                {
                    match serde_json::to_string_pretty(&self.project) {
                        Ok(text) => match std::fs::write(&path, text) {
                            Ok(()) => self.status = format!("Saved {}", path.display()),
                            Err(err) => self.status = err.to_string(),
                        },
                        Err(err) => self.status = err.to_string(),
                    }
                }
            }
            if ui.button("Open project JSON…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Project", &["json"])
                    .pick_file()
                {
                    match std::fs::read_to_string(&path).and_then(|text| {
                        // Migrates in memory. The chosen file is not written back.
                        load_project_json(&text)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    }) {
                        Ok(project) => {
                            self.project = project;
                            self.textures.clear();
                            self.level_sel = 0;
                            self.status = format!("Opened {}", path.display());
                        }
                        Err(err) => self.status = err.to_string(),
                    }
                }
            }
        });
        if !self.export_note.is_empty() {
            ui.label(&self.export_note);
        }
    }

    fn guide(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Guide").font(serif(26.0)));
        for paragraph in GUIDE.split("\n\n") {
            ui.add_space(6.0);
            ui.label(paragraph);
        }
    }

    fn assist(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Workshop assistant")
                .font(serif(22.0))
                .color(ember()),
        );
        ui.label("This chat edits the project. It does not draw. The Image maker is a separate window with its own key.");
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(420.0)
            .show(ui, |ui| {
                for (role, text) in &self.chat {
                    let title = if role == "assistant" {
                        "Assistant"
                    } else if role == "tool" {
                        "Tool"
                    } else {
                        "You"
                    };
                    ui.label(RichText::new(title).color(ember()).font(serif(14.0)));
                    ui.label(text);
                    ui.add_space(4.0);
                }
            });
        ui.add(
            egui::TextEdit::multiline(&mut self.chat_input)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .hint_text("Ask for a course edit"),
        );
        ui.horizontal(|ui| {
            let busy = self.assist_busy;
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new(if busy { "Working…" } else { "Send" }),
                )
                .clicked()
            {
                let text = self.chat_input.trim().to_string();
                if !text.is_empty() {
                    self.chat.push(("user".into(), text));
                    self.chat_input.clear();
                    let history = self.chat.clone();
                    self.assist_busy = true;
                    let _ = self.tx.send(Req::Assist {
                        keys: self.keys.clone(),
                        history,
                        project: self.project.clone(),
                    });
                }
            }
            if ui.button("API keys").clicked() {
                self.keys_open = true;
            }
        });
        ui.label(format!(
            "Provider {} · model {}",
            self.keys.assistant_provider,
            if self.keys.assistant_model.is_empty() {
                self.keys.suggested_assistant_model()
            } else {
                self.keys.assistant_model.as_str()
            }
        ));
    }

    fn keys_window(&mut self, ctx: &egui::Context) {
        let mut open = self.keys_open;
        egui::Window::new("API keys")
            .open(&mut open)
            .resizable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("Two separate keys").font(serif(20.0)));
                ui.label("Keys stay in the OS config folder, mode 0600 where the system allows it. They are not stored in the project and they are not written into an export.");
                ui.separator();
                ui.label(RichText::new("1. Workshop assistant").font(serif(18.0)).color(ember()));
                egui::ComboBox::from_id_salt("asst-provider")
                    .selected_text(provider_label(&self.keys.assistant_provider))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.keys.assistant_provider, "openai".into(), "OpenAI");
                        ui.selectable_value(&mut self.keys.assistant_provider, "anthropic".into(), "Anthropic");
                        ui.selectable_value(&mut self.keys.assistant_provider, "custom".into(), "Custom OpenAI-compatible");
                    });
                ui.label("Base URL (custom, or blank for the provider default)");
                ui.text_edit_singleline(&mut self.keys.assistant_base_url);
                ui.label(format!("Suggested model: {}", self.keys.suggested_assistant_model()));
                ui.horizontal(|ui| {
                    ui.label("Model");
                    ui.text_edit_singleline(&mut self.keys.assistant_model);
                    if ui.button("Use suggestion").clicked() {
                        self.keys.assistant_model = self.keys.suggested_assistant_model().into();
                    }
                });
                ui.label("Assistant key");
                ui.add(egui::TextEdit::singleline(&mut self.keys.assistant_key).password(true));
                ui.separator();
                ui.label(RichText::new("2. Image maker").font(serif(18.0)).color(ember()));
                ui.label("Used only when you press Generate in the Image maker. The assistant cannot call it.");
                egui::ComboBox::from_id_salt("img-provider")
                    .selected_text(if self.keys.image_provider == "custom" { "Custom OpenAI-compatible" } else { "OpenAI images" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.keys.image_provider, "openai".into(), "OpenAI images");
                        ui.selectable_value(&mut self.keys.image_provider, "custom".into(), "Custom OpenAI-compatible");
                    });
                ui.label("Base URL for a custom /images/generations endpoint");
                ui.text_edit_singleline(&mut self.keys.image_base_url);
                ui.horizontal(|ui| {
                    ui.label("Model");
                    ui.text_edit_singleline(&mut self.keys.image_model);
                    if ui.button("gpt-image-1").clicked() {
                        self.keys.image_model = "gpt-image-1".into();
                    }
                    if ui.button("dall-e-3").clicked() {
                        self.keys.image_model = "dall-e-3".into();
                    }
                });
                ui.label("Image key");
                ui.add(egui::TextEdit::singleline(&mut self.keys.image_key).password(true));
                if ui.button("Save keys").clicked() {
                    match self.keys.save() {
                        Ok(path) => self.keys_status = format!("Saved to {}", path.display()),
                        Err(err) => self.keys_status = err,
                    }
                }
                if !self.keys_status.is_empty() {
                    ui.label(&self.keys_status);
                }
            });
        self.keys_open = open;
    }

    fn image_window(&mut self, ctx: &egui::Context) {
        let mut open = self.image_open;
        egui::Window::new("Image maker")
            .open(&mut open)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("Original sprites only").font(serif(20.0)));
                ui.label("This is not the workshop assistant. Nothing is sent until you press Generate, and that call uses the image key only.");
                ui.label("Prompts that name commercial game characters are refused. Ask for an original design.");
                ui.add(egui::TextEdit::multiline(&mut self.image_prompt).desired_rows(4).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.image_side, 16, "16");
                    ui.selectable_value(&mut self.image_side, 32, "32");
                    ui.selectable_value(&mut self.image_side, 48, "48");
                    egui::ComboBox::from_id_salt("img-kind")
                        .selected_text(self.image_kind.label())
                        .show_ui(ui, |ui| {
                            for kind in visual_kinds() {
                                ui.selectable_value(&mut self.image_kind, kind, kind.label());
                            }
                        });
                });
                let busy = self.image_busy;
                if ui.add_enabled(!busy, egui::Button::new(if busy { "Generating…" } else { "Generate" })).clicked() {
                    let prompt = self.image_prompt.trim().to_string();
                    if nesticles_core::image_prompt_blocked(&prompt).is_some() {
                        self.image_status = "That prompt is refused because it names a commercial character. Describe an original design.".into();
                    } else if prompt.is_empty() {
                        self.image_status = "Write a prompt first.".into();
                    } else {
                        self.image_busy = true;
                        self.image_status = "Sending the prompt to the image endpoint…".into();
                        let _ = self.tx.send(Req::Image {
                            keys: self.keys.clone(),
                            prompt,
                            side: self.image_side,
                            kind: self.image_kind,
                        });
                    }
                }
                ui.label(&self.image_status);
            });
        self.image_open = open;
    }
}

impl NestApp {
    fn load_rom_path(&mut self) {
        let path = self.rom_path.clone();
        match std::fs::read(&path) {
            Ok(bytes) => {
                let name = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cartridge")
                    .to_string();
                self.load_rom_bytes(name, bytes);
            }
            Err(err) => self.status = err.to_string(),
        }
    }

    fn load_rom_bytes(&mut self, name: impl Into<String>, bytes: Vec<u8>) {
        let name = name.into();
        if bytes.len() > MAX_ROM_BYTES {
            self.status = format!(
                "{} is {} bytes. Nesticles stops at 64 MB.",
                name,
                bytes.len()
            );
            return;
        }
        let info = detect_rom(&bytes, &name);
        self.codec = info.codec;
        self.palette = default_palette(info.codec);
        self.tile_offset_text = format!("{:X}", info.tile_offset);
        self.page = 0;
        self.selected = Some(1);
        self.strings = find_strings(&bytes, 80);
        self.disasm_index_text = format!("{:X}", info.prg_start);
        self.disasm_base_text = if info.format == RomFormat::Nes {
            "8000"
        } else {
            "0"
        }
        .into();
        let style_name = if info.title.trim().is_empty() {
            name.clone()
        } else {
            info.title.clone()
        };
        self.rom = Some(RomSession {
            name: name.clone(),
            bytes,
            info,
        });
        let style_id = self.project.adopt_style(&style_name, &name);
        self.active_style = style_id;
        self.tray_filter.clear();
        self.refresh_disasm();
        self.tab = Tab::Cartridge;
        self.status = format!("Style \"{style_name}\" is ready. Parts you keep join it. Styles already in the project stay.");
    }

    fn refresh_disasm(&mut self) {
        let Some(rom) = &self.rom else {
            self.disasm.clear();
            return;
        };
        let index = parse_hex_or_dec(&self.disasm_index_text);
        let base = parse_hex_or_dec(&self.disasm_base_text) as u16;
        self.disasm = disassemble6502(&rom.bytes, index, base, 48);
    }

    fn scan_audio(&mut self, kind: &str) {
        let Some(rom) = &self.rom else {
            self.status = "Open a cartridge before scanning audio.".into();
            return;
        };
        let end = rom.bytes.len().min(256 * 1024);
        self.hits = match kind {
            "wave" => scan_gb_waves(&rom.bytes),
            "brr" => scan_brr(&rom.bytes),
            _ => scan_dpcm(&rom.bytes, 0, end),
        };
        self.tab = Tab::Audio;
        self.status = format!("Found {} audio candidates", self.hits.len());
    }

    fn import_audio_path(&mut self, path: &std::path::Path) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(err) => {
                self.status = err.to_string();
                return;
            }
        };
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("audio");
        match import_audio(&bytes, name) {
            Ok((samples, rate)) => {
                let wav = if read_pcm_wav(&bytes).is_ok() {
                    bytes
                } else {
                    wav_from_samples(&samples, rate)
                };
                let style_id = self.part_style();
                self.project.assets.push(Asset {
                    id: nid("sfx-"),
                    name: name.into(),
                    kind: AssetKind::Sfx,
                    style_id,
                    w: 0,
                    h: 0,
                    rgba_base64: String::new(),
                    png_base64: String::new(),
                    wav_base64: b64_encode(&wav),
                    solid: false,
                    cue: String::new(),
                    item_tag: nesticles_core::ItemTag::Auto,
                    block_tag: nesticles_core::BlockTag::Auto,
                    hazard_tag: nesticles_core::HazardTag::Auto,
                    warp: None,
                });
                self.status = format!("Imported {name}");
                self.tab = Tab::Library;
            }
            Err(err) => self.status = err,
        }
    }

    fn tile_indices(&self, page_origin: usize, selected: usize) -> Option<Vec<u8>> {
        let rom = self.rom.as_ref()?;
        let off = page_origin + selected * tile_bytes(self.codec);
        decode_tile(&rom.bytes, off, self.codec)
    }

    fn grab_stamp(&self, page_origin: usize) -> Option<(u32, u32, Vec<u8>, Vec<u8>)> {
        let selected = self.selected?;
        let stamp = self.stamp.max(1);
        let mut indices = vec![0u8; stamp * stamp * 64];
        for ty in 0..stamp {
            for tx in 0..stamp {
                let tile = selected + ty * self.columns.max(1) + tx;
                let decoded = self
                    .tile_indices(page_origin, tile)
                    .unwrap_or_else(|| vec![0; 64]);
                let dest = (ty * stamp + tx) * 64;
                indices[dest..dest + 64].copy_from_slice(&decoded[..64]);
            }
        }
        let (w, h, rgba) =
            indices_to_rgba(&indices, stamp, stamp, &self.palette, self.transparent_zero);
        let png = nesticles_core::pngutil::rgba_to_png(&rgba, w, h).unwrap_or_default();
        Some((w, h, rgba, png))
    }

    fn export_stamp_png(&mut self, page_origin: usize) {
        let Some((_, _, _, png)) = self.grab_stamp(page_origin) else {
            self.status = "Select a tile first.".into();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, png) {
            Ok(()) => self.status = format!("Wrote {}", path.display()),
            Err(err) => self.status = err.to_string(),
        }
    }

    fn export_asset_png(&mut self, id: &str) {
        let Some(asset) = self.project.asset(id) else {
            return;
        };
        let bytes = if !asset.png_base64.is_empty() {
            b64_decode(&asset.png_base64).unwrap_or_default()
        } else if !asset.rgba_base64.is_empty() {
            let rgba = b64_decode(&asset.rgba_base64).unwrap_or_default();
            nesticles_core::pngutil::rgba_to_png(&rgba, asset.w, asset.h).unwrap_or_default()
        } else {
            Vec::new()
        };
        if bytes.is_empty() {
            self.status = "That asset has no picture.".into();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        match std::fs::write(&path, bytes) {
            Ok(()) => self.status = format!("Wrote {}", path.display()),
            Err(err) => self.status = err.to_string(),
        }
    }

    fn add_rgba_asset(
        &mut self,
        name: &str,
        kind: AssetKind,
        w: u32,
        h: u32,
        rgba: Vec<u8>,
        png: Vec<u8>,
    ) {
        let id = nid("sprite-");
        let style_id = self.part_style();
        self.project.assets.push(Asset {
            id: id.clone(),
            name: format!("{name} {}", kind.label()),
            kind,
            style_id,
            w,
            h,
            rgba_base64: b64_encode(&rgba),
            png_base64: b64_encode(&png),
            wav_base64: String::new(),
            solid: kind.default_solid(),
            cue: String::new(),
            item_tag: nesticles_core::ItemTag::Auto,
            block_tag: nesticles_core::BlockTag::Auto,
            hazard_tag: nesticles_core::HazardTag::Auto,
            warp: None,
        });
        self.lib_sel = Some(id);
        self.textures.clear();
        self.tex_epoch = self.tex_epoch.wrapping_add(1);
    }

    fn ensure_textures(&mut self, ctx: &egui::Context) {
        for asset in &self.project.assets {
            if self.textures.contains_key(&asset.id)
                || asset.rgba_base64.is_empty()
                || asset.w == 0
                || asset.h == 0
            {
                continue;
            }
            let Ok(rgba) = b64_decode(&asset.rgba_base64) else {
                continue;
            };
            let need = asset.w as usize * asset.h as usize * 4;
            if rgba.len() < need {
                continue;
            }
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [asset.w as usize, asset.h as usize],
                &rgba[..need],
            );
            let tex = ctx.load_texture(
                format!("{}-{}", asset.id, self.tex_epoch),
                image,
                TextureOptions::NEAREST,
            );
            self.textures.insert(asset.id.clone(), tex);
        }
    }

    fn part_style(&mut self) -> String {
        if !self.active_style.is_empty() {
            return self.active_style.clone();
        }
        if let Some(style) = self.project.styles.first() {
            return style.id.clone();
        }
        let id = self.project.adopt_style("Original drawings", "generated");
        self.active_style = id.clone();
        id
    }

    fn godot_preview_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Experimental Godot preview. It writes a small Godot scene from this course: the player picture, placed solid fixtures, gravity, horizontal movement, jump, and a following camera. Enemies, items, pipes, entrances, and transitions stay in the Rust Play view. This is not the full game.");
        ui.horizontal(|ui| {
            ui.label("Godot 4.7.2");
            ui.add(
                egui::TextEdit::singleline(&mut self.godot_path)
                    .desired_width(280.0)
                    .hint_text("executable, or blank to search"),
            );
            if ui.button("Preview in Godot (Experimental)").clicked() {
                self.launch_godot_preview();
            }
        });
    }

    fn launch_godot_preview(&mut self) {
        let configured = {
            let text = self.godot_path.trim();
            if text.is_empty() {
                None
            } else {
                Some(std::path::PathBuf::from(text))
            }
        };
        let tool = match nesticles_core::godot::discover(configured.as_deref()) {
            Ok(tool) => tool,
            Err(err) => {
                self.status = err;
                return;
            }
        };
        let dest = nesticles_core::godot::preview_dir();
        let level_id = self.current_level_id();
        if let Err(err) = nesticles_core::godot::generate_proof(&self.project, &level_id, &dest) {
            self.status = err;
            return;
        }
        self.godot_busy = true;
        self.status = "Importing the experimental Godot preview.".into();
        if self
            .tx
            .send(Req::Godot {
                executable: tool.executable,
                project_dir: dest,
                version: tool.version,
            })
            .is_err()
        {
            self.godot_busy = false;
            self.status = "Could not start the Godot preview.".into();
        }
    }

    fn current_level_id(&self) -> String {
        self.project
            .levels
            .get(self.level_sel)
            .map(|l| l.id.clone())
            .unwrap_or_default()
    }

    fn start_level(&mut self, level_id: String, keep_cleared: bool) {
        let cleared = if keep_cleared {
            self.play
                .as_ref()
                .map(|p| p.cleared.clone())
                .unwrap_or_default()
        } else {
            self.play
                .as_ref()
                .map(|p| p.cleared.clone())
                .unwrap_or_default()
        };
        let Some(level) = self.project.level(&level_id).cloned() else {
            self.status = "Pick a course first.".into();
            return;
        };
        let sim = Sim::from_level(&self.project, &level);
        let faults = sim.blueprint_faults().to_vec();
        self.stop_music();
        if let Some(music_id) = &level.music_id {
            if let Some(asset) = self.project.asset(music_id) {
                if let Ok(bytes) = b64_decode(&asset.wav_base64) {
                    if let Ok(child) = audio_out::loop_wav(&bytes) {
                        self.music = Some(child);
                    }
                }
            }
        }
        self.play = Some(PlayState {
            from_map: keep_cleared,
            level_id,
            sim: Some(sim),
            cleared,
            message: String::new(),
            accum: 0.0,
            select: 0,
        });
        self.status = if faults.is_empty() {
            format!("Playing {}", level.name)
        } else {
            format!("Playing {}. {}", level.name, faults.join(" "))
        };
    }

    fn unlocked(&self) -> Vec<String> {
        let cleared = self.play.as_ref().map(|p| &p.cleared);
        let mut open = Vec::new();
        let mut seen = HashSet::new();
        let mut queue = vec![self.project.start_node_id.clone()];
        while let Some(id) = queue.pop() {
            if id.is_empty() || !seen.insert(id.clone()) {
                continue;
            }
            let Some(node) = self.project.nodes.iter().find(|n| n.id == id) else {
                continue;
            };
            open.push(id);
            if cleared.map(|c| c.contains(&node.level_id)).unwrap_or(false) {
                queue.extend(node.links.iter().cloned());
            }
        }
        open
    }

    fn cue(&mut self, name: &str) {
        if self.last_cue.elapsed() < std::time::Duration::from_millis(40) && name == "jump" {
            // still play the first jump
        }
        self.last_cue = Instant::now();
        let Some(asset) = self.project.assets.iter().find(|a| a.cue == name) else {
            return;
        };
        if let Ok(bytes) = b64_decode(&asset.wav_base64) {
            let _ = audio_out::preview_wav(&bytes);
        }
    }

    fn preview_asset_wav(&mut self, id: &str) {
        let Some(asset) = self.project.asset(id) else {
            return;
        };
        match b64_decode(&asset.wav_base64) {
            Ok(bytes) => {
                if let Err(err) = audio_out::preview_wav(&bytes) {
                    self.status = err;
                }
            }
            Err(err) => self.status = err,
        }
    }

    fn stop_music(&mut self) {
        if let Some(mut child) = self.music.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn hazard_of(project: &Project, level_id: &str, x: i32, y: i32) -> Option<LegacyHazard> {
    placed_fixture(project, level_id, x, y)
        .ok()
        .flatten()
        .and_then(|parts| parts.hazard)
}

fn pipe_entry(hazard: Option<LegacyHazard>) -> EntryDirection {
    match hazard {
        Some(LegacyHazard::PipeUp) => EntryDirection::Up,
        Some(LegacyHazard::PipeLeft) => EntryDirection::Left,
        Some(LegacyHazard::PipeRight) => EntryDirection::Right,
        _ => EntryDirection::Down,
    }
}

fn opening_cell(hazard: Option<LegacyHazard>, x: i32, y: i32) -> (i32, i32) {
    match hazard {
        Some(LegacyHazard::PipeUp) => (x, y + 1),
        Some(LegacyHazard::PipeLeft) => (x + 1, y),
        Some(LegacyHazard::PipeRight) => (x - 1, y),
        _ => (x, y - 1),
    }
}

fn pipe_at_opening(
    project: &Project,
    level_id: &str,
    entrance_x: i32,
    entrance_y: i32,
) -> Option<(i32, i32, EntryDirection)> {
    let level = project.level(level_id)?;
    for cell in &level.cells {
        let Some(hazard) = hazard_of(project, level_id, cell.x, cell.y) else {
            continue;
        };
        let (ox, oy) = opening_cell(Some(hazard), cell.x, cell.y);
        if ox == entrance_x && oy == entrance_y {
            return Some((cell.x, cell.y, pipe_entry(Some(hazard))));
        }
    }
    let hazard = hazard_of(project, level_id, entrance_x, entrance_y)?;
    if matches!(
        hazard,
        LegacyHazard::PipeDown
            | LegacyHazard::PipeUp
            | LegacyHazard::PipeLeft
            | LegacyHazard::PipeRight
    ) {
        return Some((entrance_x, entrance_y, pipe_entry(Some(hazard))));
    }
    None
}

fn brand_texture(ctx: &egui::Context) -> TextureHandle {
    let icon = crate::icon::icon_data();
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [icon.width as usize, icon.height as usize],
        &icon.rgba,
    );
    ctx.load_texture("nesticles-icon", image, TextureOptions::NEAREST)
}

fn install_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::from_rgb(36, 28, 25);
    visuals.panel_fill = Color32::from_rgb(43, 34, 30);
    visuals.extreme_bg_color = Color32::from_rgb(20, 16, 14);
    visuals.faint_bg_color = Color32::from_rgb(54, 42, 36);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(48, 38, 33);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(70, 50, 40);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(176, 84, 42);
    visuals.widgets.active.bg_fill = Color32::from_rgb(204, 102, 52);
    visuals.selection.bg_fill = Color32::from_rgb(140, 68, 36);
    visuals.hyperlink_color = Color32::from_rgb(232, 168, 96);
    visuals.override_text_color = Some(paper());
    visuals.window_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(120, 72, 48));
    ctx.set_visuals(visuals);
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "liberation".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/LiberationSerif-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "liberation_bold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/LiberationSerif-Bold.ttf"
        ))),
    );
    fonts.families.insert(
        FontFamily::Name("serif".into()),
        vec!["liberation_bold".into(), "liberation".into()],
    );
    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "liberation".into());
    }
    ctx.set_fonts(fonts);
}

fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif".into()))
}
fn ember() -> Color32 {
    Color32::from_rgb(224, 122, 61)
}
fn paper() -> Color32 {
    Color32::from_rgb(244, 236, 224)
}
fn kind_color(kind: AssetKind) -> Color32 {
    match kind {
        AssetKind::Player => ember(),
        AssetKind::Enemy => Color32::from_rgb(176, 64, 78),
        AssetKind::Terrain => Color32::from_rgb(110, 140, 74),
        AssetKind::Background => Color32::from_rgb(70, 110, 130),
        AssetKind::Foreground => Color32::from_rgb(90, 140, 80),
        AssetKind::Item => Color32::from_rgb(230, 186, 84),
        AssetKind::Hazard => Color32::from_rgb(160, 48, 58),
        AssetKind::Goal => Color32::from_rgb(240, 210, 120),
        AssetKind::Sfx | AssetKind::Music => Color32::from_rgb(120, 120, 140),
    }
}
fn visual_kinds() -> [AssetKind; 8] {
    [
        AssetKind::Player,
        AssetKind::Enemy,
        AssetKind::Terrain,
        AssetKind::Background,
        AssetKind::Foreground,
        AssetKind::Item,
        AssetKind::Hazard,
        AssetKind::Goal,
    ]
}
fn parse_hex_or_dec(text: &str) -> usize {
    // Offset and address fields are labeled hex. "4010" is 0x4010, not four thousand ten.
    let t = text
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    usize::from_str_radix(t, 16).unwrap_or_else(|_| t.parse().unwrap_or(0))
}
fn parse_hex_color(text: &str) -> Color32 {
    let t = text.trim().trim_start_matches('#');
    if t.len() >= 6 {
        let r = u8::from_str_radix(&t[0..2], 16).unwrap_or(36);
        let g = u8::from_str_radix(&t[2..4], 16).unwrap_or(52);
        let b = u8::from_str_radix(&t[4..6], 16).unwrap_or(60);
        Color32::from_rgb(r, g, b)
    } else {
        Color32::from_rgb(36, 52, 60)
    }
}
fn page_sig(
    origin: usize,
    codec: Codec,
    palette: &[[u8; 3]],
    cols: usize,
    rows: usize,
    clear: bool,
    len: usize,
) -> u64 {
    let mut h = origin as u64 ^ (cols as u64) << 16 ^ (rows as u64) << 24 ^ len as u64;
    h ^= codec as u64;
    if clear {
        h ^= 0x9e37;
    }
    for color in palette {
        h = h.wrapping_mul(0x100000001b3)
            ^ color[0] as u64
            ^ ((color[1] as u64) << 8)
            ^ ((color[2] as u64) << 16);
    }
    h
}
fn waveform(ui: &mut egui::Ui, samples: &[f32]) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(300.0, 48.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 16, 14));
    if samples.is_empty() {
        return;
    }
    let step = (samples.len() / 140).max(1);
    let mid = rect.center().y;
    let mut last = None;
    for (i, sample) in samples.iter().step_by(step).take(140).enumerate() {
        let x = rect.left() + i as f32 * rect.width() / 140.0;
        let y = mid - sample.clamp(-1.0, 1.0) * rect.height() * 0.45;
        let p = Pos2::new(x, y);
        if let Some(prev) = last {
            painter.line_segment([prev, p], egui::Stroke::new(1.2_f32, ember()));
        }
        last = Some(p);
    }
}
fn provider_label(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "Anthropic",
        "custom" => "Custom OpenAI-compatible",
        _ => "OpenAI",
    }
}

const GUIDE: &str = "\
Nesticles is a local workshop. You bring a cartridge image, page it as tiles, keep the pieces you tag, and build an original game around them. The desktop window is the editor and the player on Linux and Windows. The export folder is a browser player, so the same game opens in Chrome, Firefox, and Edge on Linux, Windows, and Android.

Open a file from Cartridge, or load the bundled demo image. Nesticles reads NES / iNES / NES 2.0 (including the trainer bit), Game Boy, Game Boy Color, Super NES LoROM and HiROM (including a 512-byte copier header), and Game Boy Advance. Anything it does not recognize still pages as tiles. Files are capped around 64 MB. Encrypted Switch, 3DS, and Wii U images are not decrypted.

Tiles are uncompressed only: NES 2bpp with separated planes, Game Boy 2bpp with interleaved rows, Super NES 4bpp, Game Boy Advance 4bpp with the low nibble first, and 1bpp. The preview is nearest-neighbor. A deduped scan suggests empty, sprite-like, scenery-like, text-like, or noise. You still tag player, enemy, terrain, background, foreground, item, hazard, or goal. Cartridges do not label the player versus an enemy, so the tag has to be yours. Add as part converts that stamp into a drag-and-drop piece on the current style. Export PNG writes the picture without putting it on a course.

Audio can preview NES DPCM (least-significant bit first, starting at level 64, stepping by 2, clamped to 0–126), Game Boy 16-byte wave tables, and Super NES BRR. A kept clip becomes a sound or music. You can also import PCM WAV, and Ogg or MP3 when the file decodes.

Notes show ASCII strings and, for NES, a linear 6502 disassembly. That listing does not follow banks and it will read data as code. Scripts are custom per game. Physics is not extracted. Play runs an original 1080p engine at 60 Hz. Character Game Style Physics chooses SMB1 - Mario, Luigi, Toad, or Peach. Power Up Styles chooses SMB1. Enemy, Hazards & Collision Rules Style chooses SMB1. A selected enemy can also pick a roster entry. Inherit Level Style on one enemy uses the course style.

Each cartridge becomes a style, the way a maker keeps several game looks in one tray. Dropping in another game adds a style and leaves the earlier parts where they are. A course has a home style, and the tray can show that style or every style. Drag a part onto the grid. Parts from more than one style can share a course. Character Game Style Physics chooses the controller used in Play, not the cartridge's code. The overworld nodes link to courses. Clearing a course unlocks the nodes it links to. Export writes index.html, the project JSON, a short readme, a web manifest, and icon.png for the browser tab and home screen. When the wasm player was built, the folder also contains the wasm and its JavaScript glue, and the page inlines them so opening the HTML file is enough to play. Adding that page to an Android home screen needs https.

Nesticles does not replace tools that already understand compressed graphics or a particular game's formats:

Dazzlie (https://github.com/obskyr/dazzlie) converts Game Boy and NES tiles to and from PNG.

nesrip (https://github.com/FitzRoyX/nesrip) builds NES and Super NES tilesheets. It needs its graphics database.

RetroTool (https://github.com/danielburgess/RetroTool) covers Super NES codecs and scripts, and it can emit Godot or Tiled data.

TileBravo (https://github.com/taihendarou/tilebravo) is a tile editor.

gbgraphics (https://github.com/drpaneas/gbgraphics) finds Game Boy tiles from a screenshot.

Mesen (https://github.com/SourMesen/Mesen2) and SameBoy (https://github.com/LIJI32/SameBoy) are debuggers and tile viewers for when a game builds its patterns in RAM.

Tiled (https://github.com/mapeditor/tiled) and Godot (https://godotengine.org/) are the next stop if you later want a native build for mobile stores.

The two keys in API keys are independent. The workshop assistant uses one key (OpenAI, Anthropic, or a custom OpenAI-compatible base URL) and the model id is editable. Suggested starting points are gpt-4.1-mini and claude-sonnet-4-5. The image maker uses a second key and only runs when you press Generate. It speaks OpenAI images (gpt-image-1 or dall-e-3) or a custom OpenAI-compatible images endpoint, then downscales and quantizes into a 16, 32, or 48 pixel sprite. The assistant cannot call that model. If you ask it to draw, it will point you at the Image maker.";
