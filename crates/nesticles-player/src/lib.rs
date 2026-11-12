//! Browser player. Physics comes from nesticles-core, the same crate the desktop app uses.

use nesticles_core::physics::{play_surface_scale, Input, Sim, TILE};
use nesticles_core::pngutil::b64_decode;
use nesticles_core::{AssetKind, Layer, Project};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = nesticlesPlayWav)]
    fn nesticles_play_wav(b64: &str, looped: bool);
}

thread_local! {
    static GAME: RefCell<Option<Game>> = const { RefCell::new(None) };
}

struct Sprite {
    canvas: HtmlCanvasElement,
}

struct Game {
    project: Project,
    sprites: HashMap<String, Sprite>,
    ctx: CanvasRenderingContext2d,
    cleared: HashSet<String>,
    mode: Mode,
    sim: Option<Sim>,
    level_id: String,
    message: String,
    select: usize,
    music_on: bool,
    pointer: Option<(f32, f32)>,
    prev_left: bool,
    prev_right: bool,
    run_held: bool,
    up_held: bool,
    down_held: bool,
    accum: f32,
}

enum Mode {
    Map,
    Level,
}

#[wasm_bindgen]
pub fn start_game(project_json: &str) {
    let document = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };
    let canvas = match document
        .get_element_by_id("screen")
        .and_then(|el| el.dyn_into::<HtmlCanvasElement>().ok())
    {
        Some(c) => c,
        None => return,
    };
    let ctx = match canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|o| o.dyn_into::<CanvasRenderingContext2d>().ok())
    {
        Some(c) => c,
        None => return,
    };
    ctx.set_image_smoothing_enabled(false);
    let project = match serde_json::from_str::<Project>(project_json) {
        Ok(p) => p,
        Err(err) => {
            paint_message(&ctx, &format!("Could not read project: {err}"));
            return;
        }
    };
    let mut sprites = HashMap::new();
    for asset in &project.assets {
        if asset.rgba_base64.is_empty() || asset.w == 0 || asset.h == 0 {
            continue;
        }
        if let Ok(rgba) = b64_decode(&asset.rgba_base64) {
            if let Ok(canvas) = sprite_canvas(&rgba, asset.w, asset.h) {
                sprites.insert(asset.id.clone(), Sprite { canvas });
            }
        }
    }
    GAME.with(|slot| {
        *slot.borrow_mut() = Some(Game {
            project,
            sprites,
            ctx,
            cleared: HashSet::new(),
            mode: Mode::Map,
            sim: None,
            level_id: String::new(),
            message: "Choose a path".into(),
            select: 0,
            music_on: false,
            pointer: None,
            prev_left: false,
            prev_right: false,
            run_held: false,
            up_held: false,
            down_held: false,
            accum: 0.0,
        });
    });
}

#[wasm_bindgen]
pub fn pointer(x: f32, y: f32, down: bool) {
    if !down {
        return;
    }
    GAME.with(|slot| {
        if let Some(game) = slot.borrow_mut().as_mut() {
            game.pointer = Some((x, y));
        }
    });
}

#[wasm_bindgen]
pub fn set_run(held: bool) {
    GAME.with(|slot| {
        if let Some(game) = slot.borrow_mut().as_mut() {
            game.run_held = held;
        }
    });
}

#[wasm_bindgen]
pub fn set_vertical(up: bool, down: bool) {
    GAME.with(|slot| {
        if let Some(game) = slot.borrow_mut().as_mut() {
            game.up_held = up;
            game.down_held = down;
        }
    });
}

#[wasm_bindgen]
pub fn frame(left: bool, right: bool, jump_down: bool, jump_pressed: bool, dt: f32) {
    GAME.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let Some(game) = borrow.as_mut() else {
            return;
        };
        game.tick(left, right, jump_down, jump_pressed, dt);
    });
}

impl Game {
    fn tick(&mut self, left: bool, right: bool, jump_down: bool, jump_pressed: bool, dt: f32) {
        let pointer = self.pointer.take();
        match self.mode {
            Mode::Map => self.tick_map(left, right, jump_pressed, pointer),
            Mode::Level => self.tick_level(left, right, jump_down, jump_pressed, dt),
        }
    }

    fn tick_map(
        &mut self,
        left: bool,
        right: bool,
        jump_pressed: bool,
        pointer: Option<(f32, f32)>,
    ) {
        let unlocked = unlocked_nodes(&self.project, &self.cleared);
        if unlocked.is_empty() {
            self.draw_map(&unlocked);
            return;
        }
        if left && !self.prev_left {
            self.select = self.select.saturating_sub(1);
        }
        if right && !self.prev_right {
            self.select = (self.select + 1).min(unlocked.len() - 1);
        }
        self.prev_left = left;
        self.prev_right = right;
        self.select = self.select.min(unlocked.len() - 1);
        if let Some((x, y)) = pointer {
            for (i, id) in unlocked.iter().enumerate() {
                if let Some(node) = self.project.nodes.iter().find(|n| &n.id == id) {
                    let (nx, ny) = map_point(node.x, node.y);
                    let dx = nx - x as f64;
                    let dy = ny - y as f64;
                    if dx * dx + dy * dy < 28.0 * 28.0 {
                        self.select = i;
                        self.enter(&node.level_id.clone());
                        return;
                    }
                }
            }
        }
        if jump_pressed {
            if let Some(id) = unlocked.get(self.select) {
                if let Some(node) = self.project.nodes.iter().find(|n| &n.id == id) {
                    let level = node.level_id.clone();
                    self.enter(&level);
                    return;
                }
            }
        }
        self.draw_map(&unlocked);
    }

    fn enter(&mut self, level_id: &str) {
        let Some(level) = self
            .project
            .levels
            .iter()
            .find(|l| l.id == level_id)
            .cloned()
        else {
            self.message = "That path has no course".into();
            return;
        };
        self.level_id = level_id.to_string();
        self.sim = Some(Sim::from_level(&self.project, &level));
        self.mode = Mode::Level;
        self.message.clear();
        self.music_on = false;
        if let Some(id) = &level.music_id {
            if let Some(asset) = self.project.assets.iter().find(|a| &a.id == id) {
                if !asset.wav_base64.is_empty() {
                    nesticles_play_wav(&asset.wav_base64, true);
                    self.music_on = true;
                }
            }
        }
    }

    fn tick_level(
        &mut self,
        left: bool,
        right: bool,
        jump_down: bool,
        jump_pressed: bool,
        dt: f32,
    ) {
        let preset = self.project.character_physics;
        let power = self.project.power_up_style;
        if let Some(sim) = self.sim.as_mut() {
            let scale = play_surface_scale(480.0, 270.0, sim.level_w, sim.level_h);
            sim.update_view(480.0 / scale, 270.0 / scale);
        }
        let run_held = self.run_held;
        let up = self.up_held;
        let down = self.down_held;
        let frame = 1.0 / 60.0;
        self.accum += dt.clamp(0.0, 0.05);
        let mut ticks = 0;
        let mut armed = jump_pressed;
        while self.accum >= frame && ticks < 8 {
            self.accum -= frame;
            ticks += 1;
            let input = Input {
                left,
                right,
                jump_held: jump_down,
                jump_pressed: armed,
                run_held,
                up,
                down,
            };
            armed = false;
            let events = if let Some(sim) = self.sim.as_mut() {
                sim.tick_play(preset, power, &input);
                sim.events
            } else {
                nesticles_core::physics::Events::default()
            };
            self.play_events(events);
        }
        if self.accum > frame {
            self.accum %= frame;
        }
        let warp = self.sim.as_mut().and_then(|sim| sim.take_warp());
        if let Some((level, x, y)) = warp {
            let same = level == self.level_id;
            let known = self.project.levels.iter().any(|l| l.id == level);
            if !same && known {
                self.enter(&level);
            }
            if same || known {
                if let Some(sim) = self.sim.as_mut() {
                    sim.place_at_cell(x, y);
                }
            }
        }
        let won = self.sim.as_ref().map(|s| s.won).unwrap_or(false);
        if won {
            self.cleared.insert(self.level_id.clone());
            let name = self
                .project
                .levels
                .iter()
                .find(|l| l.id == self.level_id)
                .map(|l| l.name.clone())
                .unwrap_or_else(|| "Course".into());
            self.message = format!("{name} cleared. Linked paths are open.");
            self.mode = Mode::Map;
            self.sim = None;
            self.draw_map(&unlocked_nodes(&self.project, &self.cleared));
            return;
        }
        self.draw_level();
    }

    fn play_events(&self, events: nesticles_core::physics::Events) {
        if events.jump {
            self.cue("jump", false);
        }
        if events.stomp {
            self.cue("stomp", false);
        }
        if events.hurt {
            self.cue("hurt", false);
        }
        if events.goal {
            self.cue("goal", false);
        }
    }

    fn cue(&self, name: &str, looped: bool) {
        if let Some(asset) = self.project.assets.iter().find(|a| a.cue == name) {
            if !asset.wav_base64.is_empty() {
                nesticles_play_wav(&asset.wav_base64, looped);
            }
        }
    }

    fn draw_map(&self, unlocked: &[String]) {
        let ctx = &self.ctx;
        ctx.set_fill_style_str("#1c1614");
        ctx.fill_rect(0.0, 0.0, 480.0, 270.0);
        ctx.set_stroke_style_str("#8a5a3a");
        ctx.set_line_width(3.0);
        for node in &self.project.nodes {
            let (x, y) = map_point(node.x, node.y);
            for link in &node.links {
                if let Some(other) = self.project.nodes.iter().find(|n| &n.id == link) {
                    let (ox, oy) = map_point(other.x, other.y);
                    ctx.begin_path();
                    ctx.move_to(x, y);
                    ctx.line_to(ox, oy);
                    let _ = ctx.stroke();
                }
            }
        }
        for (i, node) in self.project.nodes.iter().enumerate() {
            let open = unlocked.iter().any(|id| id == &node.id);
            let selected = unlocked
                .get(self.select)
                .map(|id| id == &node.id)
                .unwrap_or(false);
            let (x, y) = map_point(node.x, node.y);
            let color = if !open {
                "#4a403c"
            } else if selected {
                "#e07a3d"
            } else {
                "#f3ead7"
            };
            ctx.set_fill_style_str(color);
            ctx.begin_path();
            let _ = ctx.arc(
                x,
                y,
                if selected { 16.0 } else { 12.0 },
                0.0,
                std::f64::consts::TAU,
            );
            ctx.fill();
            ctx.set_fill_style_str(if open { "#f4ece0" } else { "#8a8078" });
            ctx.set_font("16px serif");
            let _ = ctx.fill_text(&node.name, x + 18.0, y + 4.0);
            let _ = i;
        }
        ctx.set_fill_style_str("#f4ece0");
        ctx.set_font("18px serif");
        let _ = ctx.fill_text(&self.project.name, 16.0, 28.0);
        ctx.set_font("14px serif");
        let _ = ctx.fill_text(&self.message, 16.0, 250.0);
        let _ = ctx.fill_text("Jump enters the lit path", 16.0, 268.0);
    }

    fn draw_level(&self) {
        let Some(sim) = self.sim.as_ref() else {
            return;
        };
        let Some(level) = self.project.levels.iter().find(|l| l.id == self.level_id) else {
            return;
        };
        let ctx = &self.ctx;
        ctx.set_fill_style_str(&level.sky);
        ctx.fill_rect(0.0, 0.0, 480.0, 270.0);
        let scale = f64::from(play_surface_scale(480.0, 270.0, sim.level_w, sim.level_h));
        let cam_x = sim.view.x as f64;
        let cam_y = sim.view.y as f64;
        let mut ordered = level.cells.clone();
        ordered.sort_by_key(|c| match c.layer {
            Layer::Back => 0,
            Layer::Main => 1,
            Layer::Front => 2,
        });
        for cell in &ordered {
            let Some(asset) = self.project.assets.iter().find(|a| a.id == cell.asset_id) else {
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
                &asset.id,
                (cell.x as f64 * TILE as f64 - cam_x) * scale,
                ((cell.y as f32 * TILE - lift) as f64 - cam_y) * scale,
                TILE as f64 * scale,
                TILE as f64 * scale,
                false,
            );
        }
        for enemy in sim.enemies.iter().filter(|e| e.alive) {
            self.blit(
                &enemy.asset_id,
                (enemy.x as f64 - 1.0 - cam_x) * scale,
                (enemy.y as f64 - 2.0 - cam_y) * scale,
                TILE as f64 * scale,
                TILE as f64 * scale,
                enemy.dir < 0,
            );
        }
        for marker in &sim.markers {
            let hex = format!(
                "#{:02x}{:02x}{:02x}",
                marker.rgb[0], marker.rgb[1], marker.rgb[2]
            );
            ctx.set_fill_style_str(&hex);
            ctx.fill_rect(
                (marker.x as f64 - cam_x) * scale,
                (marker.y as f64 - cam_y) * scale,
                marker.w.max(4.0) as f64 * scale,
                marker.h.max(4.0) as f64 * scale,
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
                    &player.id,
                    (sim.player.x as f64 - 2.0 - cam_x) * scale,
                    (sim.player.y as f64 - 2.0 - cam_y) * scale,
                    TILE as f64 * scale,
                    TILE as f64 * scale,
                    sim.player.facing < 0,
                );
            }
        }
        ctx.set_fill_style_str("#f4ece0");
        ctx.set_font("16px serif");
        let _ = ctx.fill_text(&level.name, 12.0, 22.0);
    }

    fn blit(&self, id: &str, x: f64, y: f64, w: f64, h: f64, flip: bool) {
        let Some(sprite) = self.sprites.get(id) else {
            return;
        };
        let ctx = &self.ctx;
        if flip {
            let _ = ctx.save();
            let _ = ctx.translate(x + w, y);
            let _ = ctx.scale(-1.0, 1.0);
            let _ = ctx.draw_image_with_html_canvas_element_and_dw_and_dh(
                &sprite.canvas,
                0.0,
                0.0,
                w,
                h,
            );
            let _ = ctx.restore();
        } else {
            let _ =
                ctx.draw_image_with_html_canvas_element_and_dw_and_dh(&sprite.canvas, x, y, w, h);
        }
    }
}

fn map_point(x: f32, y: f32) -> (f64, f64) {
    (x as f64 * 480.0 / 640.0, y as f64 * 270.0 / 360.0)
}

fn unlocked_nodes(project: &Project, cleared: &HashSet<String>) -> Vec<String> {
    let mut open = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = vec![project.start_node_id.clone()];
    while let Some(id) = queue.pop() {
        if id.is_empty() || !seen.insert(id.clone()) {
            continue;
        }
        let Some(node) = project.nodes.iter().find(|n| n.id == id) else {
            continue;
        };
        open.push(id);
        if cleared.contains(&node.level_id) {
            queue.extend(node.links.iter().cloned());
        }
    }
    open
}

fn sprite_canvas(rgba: &[u8], w: u32, h: u32) -> Result<HtmlCanvasElement, JsValue> {
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas: HtmlCanvasElement = document.create_element("canvas")?.dyn_into()?;
    canvas.set_width(w);
    canvas.set_height(h);
    let ctx = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no 2d"))?
        .dyn_into::<CanvasRenderingContext2d>()?;
    let array = js_sys::Uint8ClampedArray::new_with_length(w.saturating_mul(h).saturating_mul(4));
    let copy = if rgba.len() as u32 >= w.saturating_mul(h).saturating_mul(4) {
        &rgba[..(w * h * 4) as usize]
    } else {
        rgba
    };
    array.copy_from(copy);
    let image = ImageData::new_with_js_u8_clamped_array_and_sh(&array, w, h)?;
    ctx.put_image_data(&image, 0.0, 0.0)?;
    Ok(canvas)
}

fn paint_message(ctx: &CanvasRenderingContext2d, text: &str) {
    ctx.set_fill_style_str("#1c1614");
    ctx.fill_rect(0.0, 0.0, 480.0, 270.0);
    ctx.set_fill_style_str("#f4ece0");
    ctx.set_font("16px serif");
    let _ = ctx.fill_text(text, 16.0, 40.0);
}
