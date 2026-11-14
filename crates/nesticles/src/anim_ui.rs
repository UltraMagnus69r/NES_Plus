//! UnMaker Lab sprite assembly and animation, plus the Play-tab picture lookup.
//! The decisions live in `nesticles_core::anim`. This file only draws them.

use super::{serif, NestApp, PlayState};
use crate::studio::{self, UnmakerView};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, TextureOptions, Vec2};
use nesticles_core::anim::{Playback, PoseClock};
use nesticles_core::codec::indices_to_rgba;
use nesticles_core::project::nid;
use nesticles_core::{
    add_clip, add_part, append_frame_to_clip, apply_standard_fallbacks, bind_blueprint,
    bind_player, composite, create_set, create_sprite, delete_clip_frame, duplicate_frame,
    duplicate_part, move_part, present, reorder_clip, select_frame, set_clip_ticks, set_part_layer,
    slice_grid, AnimSet, SpriteDef, TILE,
};

pub fn advance_anim(play: &mut PlayState) {
    let snap = {
        let Some(sim) = play.sim.as_ref() else {
            return;
        };
        (
            sim.events.hurt,
            sim.player.on_ground,
            sim.player.vx,
            sim.player.vy,
            sim.enemies
                .iter()
                .map(|enemy| enemy.alive)
                .collect::<Vec<_>>(),
        )
    };
    if snap.0 {
        play.hurt_latch = 12;
    } else if play.hurt_latch > 0 {
        play.hurt_latch -= 1;
    }
    let pose = nesticles_core::player_pose(snap.1, snap.2, snap.3, play.hurt_latch > 0);
    play.player_clock.step(pose);
    if play.enemy_clocks.len() != snap.4.len() {
        play.enemy_clocks.resize(snap.4.len(), PoseClock::default());
    }
    for (clock, alive) in play.enemy_clocks.iter_mut().zip(snap.4) {
        clock.step(nesticles_core::enemy_pose(alive));
    }
}

impl NestApp {
    pub(super) fn warm_anim_textures(&mut self, ctx: &egui::Context) {
        let mut jobs = Vec::new();
        for sprite in &self.project.sprites {
            for frame in &sprite.frames {
                if self.anim_textures.contains_key(&frame.id) {
                    continue;
                }
                if let Ok(image) = composite(sprite, frame, &self.project) {
                    jobs.push((frame.id.clone(), image.w, image.h, image.rgba));
                }
            }
        }
        for (id, w, h, rgba) in jobs {
            if w == 0 || h == 0 || rgba.len() < (w * h * 4) as usize {
                continue;
            }
            let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            let tex = ctx.load_texture(format!("anim-{id}"), image, TextureOptions::NEAREST);
            self.anim_textures.insert(id, tex);
        }
    }

    pub(super) fn blit_anim_frame(
        &self,
        painter: &egui::Painter,
        origin: Pos2,
        cam_x: f32,
        cam_y: f32,
        scale: f32,
        set_id: &str,
        pose: &str,
        facing: i32,
        elapsed: u32,
        x: f32,
        y: f32,
    ) -> bool {
        let Ok(pick) = select_frame(&self.project, set_id, pose, facing, elapsed) else {
            return false;
        };
        let Some(tex) = self.anim_textures.get(&pick.frame_id) else {
            return false;
        };
        let tex_px = tex.size_vec2();
        let size = tex_px * scale;
        let p = origin
            + Vec2::new((x - cam_x) * scale, (y - cam_y) * scale)
            + Vec2::new(
                (TILE - tex_px.x) * scale * 0.5,
                (TILE - tex_px.y) * scale * 0.5,
            );
        let rect = Rect::from_min_size(p, size);
        let uv = if pick.mirror {
            Rect::from_min_max(Pos2::new(1.0, 0.0), Pos2::new(0.0, 1.0))
        } else {
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
        };
        painter.image(tex.id(), rect, uv, Color32::WHITE);
        let _ = TILE;
        true
    }

    fn touch_anim(&mut self) {
        self.anim_textures.clear();
        self.dirty = true;
    }

    fn remember_anim(&mut self) {
        self.anim_history.capture(&self.project);
    }

    pub(super) fn undo_anim(&mut self) {
        if self.anim_history.undo(&mut self.project) {
            self.touch_anim();
            self.status = "Undid the sprite edit.".into();
        }
    }

    pub(super) fn redo_anim(&mut self) {
        if self.anim_history.redo(&mut self.project) {
            self.touch_anim();
            self.status = "Redid the sprite edit.".into();
        }
    }

    fn ensure_sprite(&mut self) -> Option<String> {
        if let Some(id) = self.sprite_id.clone() {
            if self.project.sprites.iter().any(|sprite| sprite.id == id) {
                return Some(id);
            }
        }
        let id = create_sprite(&mut self.project, "Sprite", 16, 16).ok()?;
        self.sprite_id = Some(id.clone());
        self.frame_id = self
            .project
            .sprites
            .last()
            .map(|sprite| sprite.frames[0].id.clone());
        self.touch_anim();
        Some(id)
    }

    pub(super) fn assemble_selected_tiles(&mut self, page_origin: usize) {
        let Some(selected) = self.selected else {
            self.status = "Select a tile first. Nothing was assembled.".into();
            return;
        };
        let stamp = self.stamp.max(1);
        let mut ids = Vec::new();
        for ty in 0..stamp {
            for tx in 0..stamp {
                let index = selected + ty * self.columns.max(1) + tx;
                let Some(indices) = self.tile_indices(page_origin, index) else {
                    self.status = "That tile is past the end of the cartridge.".into();
                    return;
                };
                let (w, h, rgba) =
                    indices_to_rgba(&indices, 1, 1, &self.palette, self.transparent_zero);
                let png = nesticles_core::pngutil::rgba_to_png(&rgba, w, h).unwrap_or_default();
                let name = format!("Tile {tx},{ty}");
                self.add_rgba_asset(&name, self.tag, w, h, rgba, png);
                if let Some(id) = self.paint_asset.clone() {
                    ids.push(id);
                }
            }
        }
        let side = stamp as u32 * 8;
        let Ok(sprite) = create_sprite(&mut self.project, "Assembled sprite", side, side) else {
            self.status = "The sprite could not be created.".into();
            return;
        };
        let frame = self
            .project
            .sprites
            .iter()
            .find(|item| item.id == sprite)
            .map(|item| item.frames[0].id.clone())
            .unwrap_or_default();
        for (index, id) in ids.iter().enumerate() {
            let x = (index % stamp) as i32 * 8;
            let y = (index / stamp) as i32 * 8;
            if add_part(&mut self.project, &sprite, &frame, id, x, y).is_err() {
                self.status = "A tile could not be placed. The sprite was kept.".into();
                return;
            }
        }
        self.sprite_id = Some(sprite);
        self.frame_id = Some(frame);
        self.unmaker = UnmakerView::Assemble;
        self.touch_anim();
        self.status = format!(
            "Assembled {stamp}×{stamp} tiles into a {side}×{side} sprite. The cartridge file is not in the project."
        );
    }

    pub(super) fn assemble_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("New sprite").clicked() {
                self.remember_anim();
                if let Ok(id) = create_sprite(&mut self.project, "Sprite", 16, 16) {
                    self.sprite_id = Some(id);
                    self.frame_id = self
                        .project
                        .sprites
                        .last()
                        .map(|sprite| sprite.frames[0].id.clone());
                    self.touch_anim();
                    self.status = "New 16×16 sprite. Add pictures from the asset list.".into();
                }
            }
            if ui
                .button("Add selected picture")
                .on_hover_text(
                    "Place the selected asset on this frame. It does not copy the pixels.",
                )
                .clicked()
            {
                self.place_selected_asset();
            }
            if ui.button("Zoom in").clicked() {
                self.anim_zoom = (self.anim_zoom + 1.0).min(12.0);
            }
            if ui.button("Zoom out").clicked() {
                self.anim_zoom = (self.anim_zoom - 1.0).max(2.0);
            }
            if ui.button("Undo").clicked() {
                self.undo_anim();
            }
            if ui.button("Redo").clicked() {
                self.redo_anim();
            }
        });
        self.sprite_canvas(ui, ctx);
    }

    fn place_selected_asset(&mut self) {
        let Some(asset) = self.lib_sel.clone().or_else(|| self.paint_asset.clone()) else {
            self.status = "Select a picture in the asset list first.".into();
            return;
        };
        let Some(sprite) = self.ensure_sprite() else {
            return;
        };
        let frame = self.current_frame(&sprite);
        self.remember_anim();
        match add_part(&mut self.project, &sprite, &frame, &asset, 0, 0) {
            Ok(part) => {
                self.part_id = Some(part);
                self.frame_id = Some(frame);
                self.touch_anim();
                self.status = "Placed the picture on this frame.".into();
            }
            Err(err) => self.status = err,
        }
    }

    fn current_frame(&mut self, sprite_id: &str) -> String {
        if let Some(id) = self.frame_id.clone() {
            if self.project.sprites.iter().any(|sprite| {
                sprite.id == sprite_id && sprite.frames.iter().any(|frame| frame.id == id)
            }) {
                return id;
            }
        }
        let id = self
            .project
            .sprites
            .iter()
            .find(|sprite| sprite.id == sprite_id)
            .and_then(|sprite| sprite.frames.first().map(|frame| frame.id.clone()))
            .unwrap_or_default();
        self.frame_id = Some(id.clone());
        id
    }

    fn sprite_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(sprite_id) = self.sprite_id.clone() else {
            ui.label("Create a sprite, or assemble tiles, to open the canvas.");
            return;
        };
        let Some(sprite) = self
            .project
            .sprites
            .iter()
            .find(|sprite| sprite.id == sprite_id)
            .cloned()
        else {
            ui.label("That sprite is no longer in the project.");
            return;
        };
        let frame_id = self.current_frame(&sprite_id);
        let frame = sprite
            .frames
            .iter()
            .find(|frame| frame.id == frame_id)
            .cloned();
        let Some(frame) = frame else {
            ui.label("This sprite has no frames.");
            return;
        };
        if let Ok(image) = composite(&sprite, &frame, &self.project) {
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [image.w as usize, image.h as usize],
                &image.rgba,
            );
            let tex = ctx.load_texture("sprite-canvas", color, TextureOptions::NEAREST);
            let zoom = self.anim_zoom;
            let size = Vec2::new(sprite.w as f32 * zoom, sprite.h as f32 * zoom);
            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
            checker(ui.painter(), rect, zoom);
            ui.painter().image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            if zoom >= 4.0 {
                let grid = Color32::from_rgba_unmultiplied(27, 58, 92, 70);
                for col in 0..=sprite.w {
                    let x = rect.min.x + col as f32 * zoom;
                    ui.painter().line_segment(
                        [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                        egui::Stroke::new(1.0_f32, grid),
                    );
                }
                for row in 0..=sprite.h {
                    let y = rect.min.y + row as f32 * zoom;
                    ui.painter().line_segment(
                        [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                        egui::Stroke::new(1.0_f32, grid),
                    );
                }
            }
            if let Some(part_id) = self.part_id.as_ref() {
                if let Some(part) = frame.parts.iter().find(|part| &part.id == part_id) {
                    let w = if part.src_w == 0 { 16 } else { part.src_w } as f32 * zoom;
                    let h = if part.src_h == 0 { 16 } else { part.src_h } as f32 * zoom;
                    let piece = Rect::from_min_size(
                        rect.min + Vec2::new(part.x as f32 * zoom, part.y as f32 * zoom),
                        Vec2::new(w, h),
                    );
                    ui.painter().rect_stroke(
                        piece,
                        0.0,
                        egui::Stroke::new(2.0_f32, studio::amber()),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            let origin =
                rect.min + Vec2::new(sprite.origin_x as f32 * zoom, sprite.origin_y as f32 * zoom);
            ui.painter()
                .circle_stroke(origin, 4.0, egui::Stroke::new(2.0_f32, studio::clay()));
            if response.clicked() || response.drag_started() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let local = (pos - rect.min) / zoom;
                    self.part_id = hit_part(&frame, local.x, local.y);
                }
            }
            if response.drag_started() {
                self.remember_anim();
                self.anim_dragging = true;
            }
            if response.dragged() {
                if let Some(part_id) = self.part_id.clone() {
                    let delta = response.drag_delta() / zoom;
                    let current = self
                        .project
                        .sprites
                        .iter()
                        .find(|item| item.id == sprite_id)
                        .and_then(|item| item.frames.iter().find(|item| item.id == frame_id))
                        .and_then(|item| item.parts.iter().find(|item| item.id == part_id))
                        .map(|item| (item.x, item.y));
                    if let Some((x, y)) = current {
                        let _ = move_part(
                            &mut self.project,
                            &sprite_id,
                            &frame_id,
                            &part_id,
                            x + delta.x.round() as i32,
                            y + delta.y.round() as i32,
                        );
                        self.touch_anim();
                    }
                }
            }
            if response.drag_stopped() {
                self.anim_dragging = false;
            }
        } else {
            ui.label("This frame has a missing picture. The other pieces were kept.");
        }
        ui.label(format!(
            "{}   {}×{}   zoom {}×",
            sprite.name, sprite.w, sprite.h, self.anim_zoom as i32
        ));
    }

    pub(super) fn animate_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.advance_preview(ctx);
        ui.horizontal_wrapped(|ui| {
            if ui.button("New animation").clicked() {
                self.new_animation();
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.clip_name)
                    .hint_text("State name")
                    .desired_width(140.0),
            );
            if ui
                .button("New state")
                .on_hover_text("Add a named clip such as idle, walk, or jump.")
                .clicked()
            {
                self.new_clip();
            }
            if ui.button("Play").clicked() {
                self.anim_playing = true;
            }
            if ui.button("Pause").clicked() {
                self.anim_playing = false;
            }
            if ui.button("Restart").clicked() {
                self.preview_clock.elapsed = 0;
                self.preview_accum = 0.0;
            }
            if ui.button("Step back").clicked() {
                self.anim_playing = false;
                self.preview_clock.elapsed = self.preview_clock.elapsed.saturating_sub(1);
            }
            if ui.button("Step forward").clicked() {
                self.anim_playing = false;
                self.preview_clock.elapsed = self.preview_clock.elapsed.saturating_add(1);
            }
            if ui.button("Half speed").clicked() {
                self.anim_speed = 0.5;
            }
            if ui.button("Normal speed").clicked() {
                self.anim_speed = 1.0;
            }
            if ui.button("Double speed").clicked() {
                self.anim_speed = 2.0;
            }
            if ui
                .button("Open in Maker Engine")
                .on_hover_text("The animation stays in this project. Assign it to a Blueprint there.")
                .clicked()
            {
                self.workspace = crate::studio::Workspace::Maker;
                self.tab = super::Tab::Courses;
                self.status = "The animation is already in this project. Choose it on the Player Blueprint, or select a placed part.".into();
            }
        });
        ui.label("Preview speed does not change the stored tick durations.");
        self.preview_canvas(ui, ctx);
    }

    fn advance_preview(&mut self, ctx: &egui::Context) {
        if !self.anim_playing {
            return;
        }
        let dt = ctx.input(|input| input.stable_dt);
        self.preview_accum += dt * self.anim_speed;
        let step = 1.0 / 60.0;
        let mut ticks = 0;
        while self.preview_accum >= step && ticks < 8 {
            self.preview_accum -= step;
            self.preview_clock.elapsed = self.preview_clock.elapsed.saturating_add(1);
            ticks += 1;
        }
        ctx.request_repaint();
    }

    fn preview_canvas(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(set_id) = self.set_id.clone() else {
            ui.label("Create an animation to preview it.");
            return;
        };
        let clip_name = self
            .project
            .anim_sets
            .iter()
            .find(|set| set.id == set_id)
            .and_then(|set| {
                set.clips
                    .iter()
                    .find(|clip| Some(&clip.id) == self.clip_id.as_ref())
            })
            .map(|clip| clip.name.clone())
            .unwrap_or_else(|| self.clip_name.clone());
        let shown = present(
            &self.project,
            &set_id,
            &clip_name,
            1,
            self.preview_clock.elapsed,
        );
        if shown.rgba.is_empty() {
            ui.label(if shown.note.is_empty() {
                "This state has no frames yet.".to_string()
            } else {
                shown.note.clone()
            });
            return;
        }
        let color = egui::ColorImage::from_rgba_unmultiplied(
            [shown.w as usize, shown.h as usize],
            &shown.rgba,
        );
        let tex = ctx.load_texture("anim-preview", color, TextureOptions::NEAREST);
        let zoom = self.anim_zoom;
        let size = Vec2::new(shown.w as f32 * zoom, shown.h as f32 * zoom);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        checker(ui.painter(), rect, zoom);
        ui.painter().image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.label(format!(
            "{}   frame {}   tick {}   {}× speed",
            shown.clip_name,
            shown.frame_index + 1,
            self.preview_clock.elapsed,
            self.anim_speed
        ));
    }

    fn new_animation(&mut self) {
        let Some(sprite) = self.sprite_id.clone().or_else(|| self.ensure_sprite()) else {
            return;
        };
        self.remember_anim();
        match create_set(&mut self.project, "Animation", &sprite) {
            Ok(id) => {
                self.set_id = Some(id);
                self.unmaker = UnmakerView::Animate;
                self.touch_anim();
                self.status = "New animation. Add a state, then add frames.".into();
            }
            Err(err) => self.status = err,
        }
    }

    fn new_clip(&mut self) {
        let Some(set_id) = self.set_id.clone() else {
            self.status = "Create an animation first.".into();
            return;
        };
        self.remember_anim();
        match add_clip(&mut self.project, &set_id, &self.clip_name) {
            Ok(id) => {
                self.clip_id = Some(id);
                self.preview_clock.elapsed = 0;
                self.touch_anim();
                self.status = format!("Added the {} state.", self.clip_name.trim());
            }
            Err(err) => self.status = err,
        }
    }

    pub(super) fn anim_timeline(&mut self, ui: &mut egui::Ui) {
        let Some(set_id) = self.set_id.clone() else {
            ui.label("The timeline appears while an animation is open.");
            return;
        };
        let Some(clip_id) = self.clip_id.clone() else {
            ui.label("Add a named state to edit its frames.");
            return;
        };
        let frames: Vec<(usize, String, u32)> = self
            .project
            .anim_sets
            .iter()
            .find(|set| set.id == set_id)
            .and_then(|set| set.clips.iter().find(|clip| clip.id == clip_id))
            .map(|clip| {
                clip.frame_ids
                    .iter()
                    .enumerate()
                    .map(|(index, id)| {
                        let ticks = clip.ticks.get(index).copied().unwrap_or(6);
                        (index, id.clone(), ticks)
                    })
                    .collect()
            })
            .unwrap_or_default();
        ui.horizontal(|ui| {
            if ui.button("Add frame").clicked() {
                self.timeline_add();
            }
            if ui.button("Duplicate frame").clicked() {
                self.timeline_duplicate();
            }
            if ui.button("Delete frame").clicked() {
                self.timeline_delete();
            }
            if ui.button("Move earlier").clicked() {
                self.timeline_move(-1);
            }
            if ui.button("Move later").clicked() {
                self.timeline_move(1);
            }
        });
        let mut cursor = 0u32;
        let playing = frames.iter().position(|(_, _, ticks)| {
            let start = cursor;
            cursor += *ticks;
            self.preview_clock.elapsed >= start && self.preview_clock.elapsed < start + *ticks
        });
        ui.horizontal(|ui| {
            for (index, _id, ticks) in &frames {
                let selected = self.frame_id.as_ref().is_some_and(|id| {
                    self.project
                        .anim_sets
                        .iter()
                        .find(|set| set.id == set_id)
                        .and_then(|set| set.clips.iter().find(|clip| clip.id == clip_id))
                        .and_then(|clip| clip.frame_ids.get(*index))
                        == Some(id)
                });
                let mark = if playing == Some(*index) { "▶ " } else { "" };
                let label = format!("{mark}Frame {} · {ticks} ticks", index + 1);
                let mut button = egui::Button::new(label);
                if selected {
                    button = button.fill(studio::amber());
                }
                if ui.add(button).clicked() {
                    if let Some(id) = frames.get(*index).map(|item| item.1.clone()) {
                        self.frame_id = Some(id);
                        self.preview_clock.elapsed =
                            frames.iter().take(*index).map(|item| item.2).sum();
                    }
                }
            }
        });
    }

    fn timeline_add(&mut self) {
        let (Some(set_id), Some(sprite_id)) = (self.set_id.clone(), self.sprite_id.clone()) else {
            self.status = "Open a sprite and an animation first.".into();
            return;
        };
        let Some(clip_id) = self.clip_id.clone() else {
            self.status = "Add a named state first.".into();
            return;
        };
        let frame = self.current_frame(&sprite_id);
        self.remember_anim();
        if let Err(err) = append_frame_to_clip(&mut self.project, &set_id, &clip_id, &frame, 8) {
            self.status = err;
            return;
        }
        self.touch_anim();
        self.status = "Added this frame to the state. Duration is 8 ticks.".into();
    }

    fn timeline_duplicate(&mut self) {
        let Some(sprite_id) = self.sprite_id.clone() else {
            return;
        };
        let frame = self.current_frame(&sprite_id);
        self.remember_anim();
        match duplicate_frame(&mut self.project, &sprite_id, &frame) {
            Ok(copy) => {
                self.frame_id = Some(copy.clone());
                if let (Some(set_id), Some(clip_id)) = (self.set_id.clone(), self.clip_id.clone()) {
                    let _ = append_frame_to_clip(&mut self.project, &set_id, &clip_id, &copy, 8);
                }
                self.touch_anim();
                self.status = "Duplicated the frame. The new placement is independent.".into();
            }
            Err(err) => self.status = err,
        }
    }

    fn timeline_delete(&mut self) {
        let (Some(set_id), Some(clip_id)) = (self.set_id.clone(), self.clip_id.clone()) else {
            return;
        };
        let index = self.clip_index(&set_id, &clip_id);
        self.remember_anim();
        if let Err(err) = delete_clip_frame(&mut self.project, &set_id, &clip_id, index) {
            self.status = err;
            return;
        }
        self.touch_anim();
        self.status =
            "Removed that frame from the state. The sprite frame is still available.".into();
    }

    fn timeline_move(&mut self, delta: i32) {
        let (Some(set_id), Some(clip_id)) = (self.set_id.clone(), self.clip_id.clone()) else {
            return;
        };
        let index = self.clip_index(&set_id, &clip_id);
        let target = index as i32 + delta;
        if target < 0 {
            return;
        }
        self.remember_anim();
        if let Err(err) = reorder_clip(&mut self.project, &set_id, &clip_id, index, target as usize)
        {
            self.status = err;
            return;
        }
        self.touch_anim();
        self.status = "Reordered the frames.".into();
    }

    fn clip_index(&self, set_id: &str, clip_id: &str) -> usize {
        let Some(frame_id) = self.frame_id.as_ref() else {
            return 0;
        };
        self.project
            .anim_sets
            .iter()
            .find(|set| set.id == set_id)
            .and_then(|set| set.clips.iter().find(|clip| clip.id == clip_id))
            .and_then(|clip| clip.frame_ids.iter().position(|id| id == frame_id))
            .unwrap_or(0)
    }

    pub(super) fn anim_inspector(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Sprite").font(serif(22.0)));
        let sprite_ids: Vec<(String, String)> = self
            .project
            .sprites
            .iter()
            .map(|sprite| (sprite.id.clone(), sprite.name.clone()))
            .collect();
        let mut sprite_choice = self.sprite_id.clone().unwrap_or_default();
        egui::ComboBox::from_id_salt("sprite-pick")
            .selected_text(sprite_label(&self.project.sprites, &sprite_choice))
            .show_ui(ui, |ui| {
                for (id, name) in &sprite_ids {
                    ui.selectable_value(&mut sprite_choice, id.clone(), name);
                }
            });
        if Some(sprite_choice.clone()) != self.sprite_id && !sprite_choice.is_empty() {
            self.sprite_id = Some(sprite_choice);
            self.frame_id = None;
        }
        if let Some(sprite_id) = self.sprite_id.clone() {
            if let Some(sprite) = self
                .project
                .sprites
                .iter_mut()
                .find(|sprite| sprite.id == sprite_id)
            {
                ui.label("Name");
                let before = sprite.name.clone();
                ui.text_edit_singleline(&mut sprite.name);
                if sprite.name != before {
                    self.dirty = true;
                }
                ui.label(format!("Canvas {}×{}", sprite.w, sprite.h));
            }
            let (origin_x0, origin_y0, feet0) = self
                .project
                .sprites
                .iter()
                .find(|sprite| sprite.id == sprite_id)
                .map(|sprite| {
                    let feet = sprite
                        .anchors
                        .iter()
                        .find(|anchor| anchor.name == "feet")
                        .map(|anchor| (anchor.x, anchor.y));
                    (sprite.origin_x, sprite.origin_y, feet)
                })
                .unwrap_or((0, 0, None));
            let mut origin_x = origin_x0;
            let mut origin_y = origin_y0;
            let mut origin_gesture = false;
            ui.horizontal(|ui| {
                let x = ui.add(egui::DragValue::new(&mut origin_x).prefix("Origin x "));
                let y = ui.add(egui::DragValue::new(&mut origin_y).prefix("Origin y "));
                origin_gesture =
                    x.drag_started() || y.drag_started() || (!x.dragged() && !y.dragged());
            });
            if origin_x != origin_x0 || origin_y != origin_y0 {
                if origin_gesture {
                    self.remember_anim();
                }
                if let Some(sprite) = self
                    .project
                    .sprites
                    .iter_mut()
                    .find(|sprite| sprite.id == sprite_id)
                {
                    sprite.origin_x = origin_x;
                    sprite.origin_y = origin_y;
                }
                self.dirty = true;
            }
            if let Some((feet_x0, feet_y0)) = feet0 {
                ui.label("Feet");
                let mut feet_x = feet_x0;
                let mut feet_y = feet_y0;
                let mut feet_gesture = false;
                ui.horizontal(|ui| {
                    let x = ui.add(egui::DragValue::new(&mut feet_x).prefix("x "));
                    let y = ui.add(egui::DragValue::new(&mut feet_y).prefix("y "));
                    feet_gesture =
                        x.drag_started() || y.drag_started() || (!x.dragged() && !y.dragged());
                });
                if feet_x != feet_x0 || feet_y != feet_y0 {
                    if feet_gesture {
                        self.remember_anim();
                    }
                    if let Some(feet) = self
                        .project
                        .sprites
                        .iter_mut()
                        .find(|sprite| sprite.id == sprite_id)
                        .and_then(|sprite| {
                            sprite
                                .anchors
                                .iter_mut()
                                .find(|anchor| anchor.name == "feet")
                        })
                    {
                        feet.x = feet_x;
                        feet.y = feet_y;
                    }
                    self.dirty = true;
                }
            }
            ui.label("Sheet slice");
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.sheet_w)
                        .range(1..=256)
                        .prefix("Frame w "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.sheet_h)
                        .range(1..=256)
                        .prefix("h "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.sheet_margin)
                        .range(0..=64)
                        .prefix("Margin "),
                );
                ui.add(
                    egui::DragValue::new(&mut self.sheet_spacing)
                        .range(0..=64)
                        .prefix("Gap "),
                );
            });
            ui.label("Palette indices are not reconstructed from baked colors.");
        }
        ui.separator();
        ui.label(RichText::new("Piece").font(serif(18.0)));
        ui.horizontal_wrapped(|ui| {
            if ui.button("Nudge left").clicked() {
                self.nudge(-1, 0);
            }
            if ui.button("Nudge right").clicked() {
                self.nudge(1, 0);
            }
            if ui.button("Nudge up").clicked() {
                self.nudge(0, -1);
            }
            if ui.button("Nudge down").clicked() {
                self.nudge(0, 1);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("Mirror").clicked() {
                self.flip_piece();
            }
            if ui.button("Duplicate piece").clicked() {
                self.copy_piece();
            }
            if ui.button("Remove piece").clicked() {
                self.remove_piece();
            }
            if ui.button("Layer forward").clicked() {
                self.shift_layer(1);
            }
            if ui.button("Layer back").clicked() {
                self.shift_layer(-1);
            }
        });
        ui.separator();
        ui.label(RichText::new("Animation").font(serif(18.0)));
        let sets: Vec<(String, String)> = self
            .project
            .anim_sets
            .iter()
            .map(|set| (set.id.clone(), set.name.clone()))
            .collect();
        let mut set_choice = self.set_id.clone().unwrap_or_default();
        egui::ComboBox::from_id_salt("set-pick")
            .selected_text(set_label(&self.project.anim_sets, &set_choice))
            .show_ui(ui, |ui| {
                for (id, name) in &sets {
                    ui.selectable_value(&mut set_choice, id.clone(), name);
                }
            });
        if Some(set_choice.clone()) != self.set_id && !set_choice.is_empty() {
            self.set_id = Some(set_choice);
            self.clip_id = None;
        }
        if let Some(set_id) = self.set_id.clone() {
            let clips: Vec<(String, String, Playback, i32)> = self
                .project
                .anim_sets
                .iter()
                .find(|set| set.id == set_id)
                .map(|set| {
                    set.clips
                        .iter()
                        .map(|clip| {
                            (
                                clip.id.clone(),
                                clip.name.clone(),
                                clip.playback,
                                clip.facing,
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mut clip_choice = self.clip_id.clone().unwrap_or_default();
            egui::ComboBox::from_id_salt("clip-pick")
                .selected_text(
                    clips
                        .iter()
                        .find(|(id, _, _, _)| id == &clip_choice)
                        .map(|(_, name, _, _)| name.clone())
                        .unwrap_or_else(|| "Choose a state".into()),
                )
                .show_ui(ui, |ui| {
                    for (id, name, _, _) in &clips {
                        ui.selectable_value(&mut clip_choice, id.clone(), name);
                    }
                });
            if Some(clip_choice.clone()) != self.clip_id && !clip_choice.is_empty() {
                self.clip_id = Some(clip_choice);
                self.preview_clock = PoseClock::default();
            }
            if let Some(clip_id) = self.clip_id.clone() {
                if let Some(clip) = self
                    .project
                    .anim_sets
                    .iter_mut()
                    .find(|set| set.id == set_id)
                    .and_then(|set| set.clips.iter_mut().find(|clip| clip.id == clip_id))
                {
                    let mut playback = clip.playback;
                    egui::ComboBox::from_id_salt("playback")
                        .selected_text(playback.label())
                        .show_ui(ui, |ui| {
                            for mode in Playback::ALL {
                                ui.selectable_value(&mut playback, mode, mode.label());
                            }
                        });
                    if playback != clip.playback {
                        clip.playback = playback;
                        self.dirty = true;
                    }
                    let mut facing = clip.facing;
                    ui.horizontal(|ui| {
                        if ui.selectable_label(facing == 0, "Either facing").clicked() {
                            facing = 0;
                        }
                        if ui.selectable_label(facing < 0, "Drawn left").clicked() {
                            facing = -1;
                        }
                        if ui.selectable_label(facing > 0, "Drawn right").clicked() {
                            facing = 1;
                        }
                    });
                    if facing != clip.facing {
                        clip.facing = facing;
                        self.dirty = true;
                    }
                }
                let index = self.clip_index(&set_id, &clip_id);
                let ticks = self
                    .project
                    .anim_sets
                    .iter()
                    .find(|set| set.id == set_id)
                    .and_then(|set| set.clips.iter().find(|clip| clip.id == clip_id))
                    .and_then(|clip| clip.ticks.get(index).copied())
                    .unwrap_or(8);
                let mut edited = ticks;
                let response = ui.add(egui::Slider::new(&mut edited, 1..=60).text("ticks"));
                if edited != ticks {
                    if response.drag_started() || !response.dragged() {
                        self.remember_anim();
                    }
                    if let Err(err) =
                        set_clip_ticks(&mut self.project, &set_id, &clip_id, index, edited)
                    {
                        self.status = err;
                    } else {
                        self.dirty = true;
                    }
                }
                let missing = self
                    .project
                    .anim_sets
                    .iter()
                    .find(|set| set.id == set_id)
                    .map(|set| {
                        nesticles_core::missing_states(
                            set,
                            &["idle", "walk", "jump", "fall", "hurt", "defeat"],
                        )
                    })
                    .unwrap_or_default();
                if missing.is_empty() {
                    ui.label("Idle, walk, jump, fall, hurt, and defeat all resolve.");
                } else {
                    ui.label(format!(
                        "Missing states: {}. The still picture is used for those. Nothing else is substituted.",
                        missing.join(", ")
                    ));
                }
                if ui
                    .button("Use standard fallbacks")
                    .on_hover_text(
                        "Run uses walk, walk uses idle, and so on. Attack is not filled in.",
                    )
                    .clicked()
                {
                    self.remember_anim();
                    if let Err(err) = apply_standard_fallbacks(&mut self.project, &set_id) {
                        self.status = err;
                    } else {
                        self.touch_anim();
                        self.status = "Standard fallbacks are saved on this animation.".into();
                    }
                }
            }
            if ui
                .button("Assign to course player")
                .on_hover_text("The Player Blueprint keeps its physics. Only the picture changes.")
                .clicked()
            {
                self.assign_player_anim();
            }
            if ui.button("Assign to selected blueprint").clicked() {
                self.assign_cell_anim();
            }
            if ui.button("Slice selected sheet").on_hover_text("Cut the selected picture into frames. You can still move the pieces afterward.").clicked() {
                self.slice_selected();
            }
        }
    }

    pub(super) fn cell_animation_picker(&mut self, ui: &mut egui::Ui) {
        let Some((x, y)) = self.placement_pick else {
            return;
        };
        let Some(blueprint_id) = self.project.levels.get(self.level_sel).and_then(|level| {
            level
                .cells
                .iter()
                .find(|cell| cell.x == x && cell.y == y)
                .and_then(|cell| cell.blueprint_id.clone())
        }) else {
            ui.label("This part has no Blueprint. The picture is not given gameplay.");
            return;
        };
        let sets: Vec<(String, String)> = self
            .project
            .anim_sets
            .iter()
            .map(|set| (set.id.clone(), set.name.clone()))
            .collect();
        if sets.is_empty() {
            ui.label("No animation is in this project yet.");
            return;
        }
        let current = self
            .project
            .blueprints
            .iter()
            .find(|blueprint| blueprint.id == blueprint_id)
            .map(|blueprint| blueprint.anim_set_id.clone())
            .unwrap_or_default();
        let mut choice = current.clone();
        ui.label("Animation");
        egui::ComboBox::from_id_salt("cell-anim")
            .selected_text(set_label(&self.project.anim_sets, &choice))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut choice, String::new(), "Still picture");
                for (id, name) in &sets {
                    ui.selectable_value(&mut choice, id.clone(), name);
                }
            });
        if choice != current {
            match bind_blueprint(&mut self.project, &blueprint_id, &choice) {
                Ok(()) => {
                    self.dirty = true;
                    self.status =
                        "The Blueprint animation changed. Its behavior was not changed.".into();
                }
                Err(err) => self.status = err,
            }
        }
    }

    pub(super) fn sprite_browser(&mut self, ui: &mut egui::Ui) {
        if self.project.sprites.is_empty() && self.project.anim_sets.is_empty() {
            return;
        }
        ui.separator();
        ui.label(RichText::new("Sprites").font(serif(18.0)));
        let sprites: Vec<(String, String)> = self
            .project
            .sprites
            .iter()
            .map(|sprite| (sprite.id.clone(), sprite.name.clone()))
            .collect();
        for (id, name) in sprites {
            let on = self.sprite_id.as_deref() == Some(id.as_str());
            if ui.selectable_label(on, name).clicked() {
                self.sprite_id = Some(id);
                self.frame_id = None;
                self.unmaker = UnmakerView::Assemble;
            }
        }
        ui.label(RichText::new("Animations").font(serif(18.0)));
        let sets: Vec<(String, String, String)> = self
            .project
            .anim_sets
            .iter()
            .map(|set| (set.id.clone(), set.name.clone(), set.sprite_id.clone()))
            .collect();
        for (id, name, sprite_id) in sets {
            let on = self.set_id.as_deref() == Some(id.as_str());
            if ui.selectable_label(on, name).clicked() {
                self.set_id = Some(id);
                self.sprite_id = Some(sprite_id);
                self.clip_id = None;
                self.unmaker = UnmakerView::Animate;
            }
        }
    }

    fn nudge(&mut self, dx: i32, dy: i32) {
        let (Some(sprite), Some(part)) = (self.sprite_id.clone(), self.part_id.clone()) else {
            self.status = "Select a piece on the canvas first.".into();
            return;
        };
        let frame = self.current_frame(&sprite);
        let Some((x, y)) = self
            .project
            .sprites
            .iter()
            .find(|item| item.id == sprite)
            .and_then(|item| item.frames.iter().find(|frame_item| frame_item.id == frame))
            .and_then(|frame_item| frame_item.parts.iter().find(|item| item.id == part))
            .map(|item| (item.x, item.y))
        else {
            self.status = "That piece is not on this frame.".into();
            return;
        };
        self.remember_anim();
        if let Err(err) = move_part(&mut self.project, &sprite, &frame, &part, x + dx, y + dy) {
            self.status = err;
        } else {
            self.touch_anim();
        }
    }

    fn flip_piece(&mut self) {
        let (Some(sprite), Some(part_id)) = (self.sprite_id.clone(), self.part_id.clone()) else {
            return;
        };
        let frame = self.current_frame(&sprite);
        self.remember_anim();
        if let Some(part) = self
            .project
            .sprites
            .iter_mut()
            .find(|sprite_item| sprite_item.id == sprite)
            .and_then(|sprite_item| {
                sprite_item
                    .frames
                    .iter_mut()
                    .find(|frame_item| frame_item.id == frame)
            })
            .and_then(|frame_item| frame_item.parts.iter_mut().find(|part| part.id == part_id))
        {
            part.flip_x = !part.flip_x;
            self.touch_anim();
        }
    }

    fn copy_piece(&mut self) {
        let (Some(sprite), Some(part)) = (self.sprite_id.clone(), self.part_id.clone()) else {
            return;
        };
        let frame = self.current_frame(&sprite);
        self.remember_anim();
        match duplicate_part(&mut self.project, &sprite, &frame, &part) {
            Ok(id) => {
                self.part_id = Some(id);
                self.touch_anim();
            }
            Err(err) => self.status = err,
        }
    }

    fn remove_piece(&mut self) {
        let (Some(sprite), Some(part_id)) = (self.sprite_id.clone(), self.part_id.clone()) else {
            return;
        };
        let frame = self.current_frame(&sprite);
        self.remember_anim();
        let removed = self
            .project
            .sprites
            .iter_mut()
            .find(|sprite_item| sprite_item.id == sprite)
            .and_then(|sprite_item| {
                sprite_item
                    .frames
                    .iter_mut()
                    .find(|frame_item| frame_item.id == frame)
            })
            .map(|frame_item| {
                let before = frame_item.parts.len();
                frame_item.parts.retain(|part| part.id != part_id);
                before != frame_item.parts.len()
            })
            .unwrap_or(false);
        if removed {
            self.part_id = None;
            self.touch_anim();
            self.status = "Removed the piece from this frame.".into();
        }
    }

    fn shift_layer(&mut self, delta: i32) {
        let (Some(sprite), Some(part)) = (self.sprite_id.clone(), self.part_id.clone()) else {
            return;
        };
        let frame = self.current_frame(&sprite);
        let Some(z) = self
            .project
            .sprites
            .iter()
            .find(|item| item.id == sprite)
            .and_then(|item| item.frames.iter().find(|frame_item| frame_item.id == frame))
            .and_then(|frame_item| frame_item.parts.iter().find(|item| item.id == part))
            .map(|item| item.z)
        else {
            return;
        };
        self.remember_anim();
        if let Err(err) = set_part_layer(&mut self.project, &sprite, &frame, &part, z + delta) {
            self.status = err;
        } else {
            self.touch_anim();
        }
    }

    fn assign_player_anim(&mut self) {
        let Some(set_id) = self.set_id.clone() else {
            return;
        };
        let Some(player_id) = self
            .project
            .levels
            .get(self.level_sel)
            .and_then(|level| level.player_blueprint_id.clone())
        else {
            self.status = "This course has no Player Blueprint. Create one in More. The picture was not turned into a player.".into();
            return;
        };
        self.remember_anim();
        match bind_player(&mut self.project, &player_id, &set_id) {
            Ok(()) => {
                self.touch_anim();
                self.status =
                    "The Player Blueprint now uses this animation. Its physics were not changed."
                        .into();
            }
            Err(err) => self.status = err,
        }
    }

    fn assign_cell_anim(&mut self) {
        let Some(set_id) = self.set_id.clone() else {
            return;
        };
        let Some((x, y)) = self.placement_pick else {
            self.status = "Select a placed part in Maker Engine first.".into();
            return;
        };
        let Some(blueprint_id) = self.project.levels.get(self.level_sel).and_then(|level| {
            level
                .cells
                .iter()
                .find(|cell| cell.x == x && cell.y == y)
                .and_then(|cell| cell.blueprint_id.clone())
        }) else {
            self.status = "That cell has no Blueprint. The picture was not given gameplay.".into();
            return;
        };
        self.remember_anim();
        match bind_blueprint(&mut self.project, &blueprint_id, &set_id) {
            Ok(()) => {
                self.touch_anim();
                self.status =
                    "The Blueprint uses this animation. Its behavior was not changed.".into();
            }
            Err(err) => self.status = err,
        }
    }

    fn slice_selected(&mut self) {
        let Some(asset_id) = self.lib_sel.clone().or_else(|| self.paint_asset.clone()) else {
            self.status = "Select a picture first.".into();
            return;
        };
        let Some(asset) = self.project.asset(&asset_id).cloned() else {
            return;
        };
        let slices = match slice_grid(
            asset.w,
            asset.h,
            self.sheet_w,
            self.sheet_h,
            self.sheet_margin,
            self.sheet_spacing,
        ) {
            Ok(slices) => slices,
            Err(err) => {
                self.status = err;
                return;
            }
        };
        self.remember_anim();
        let Ok(sprite) = create_sprite(
            &mut self.project,
            &format!("{} frames", asset.name),
            self.sheet_w,
            self.sheet_h,
        ) else {
            return;
        };
        let first = self
            .project
            .sprites
            .iter()
            .find(|item| item.id == sprite)
            .map(|item| item.frames[0].id.clone())
            .unwrap_or_default();
        if let Some(part) = self
            .project
            .sprites
            .iter_mut()
            .find(|item| item.id == sprite)
            .and_then(|item| item.frames.first_mut())
            .and_then(|frame| {
                frame.parts.push(nesticles_core::anim::Part {
                    id: nid("part-"),
                    asset_id: asset_id.clone(),
                    x: 0,
                    y: 0,
                    z: 0,
                    visible: true,
                    flip_x: false,
                    flip_y: false,
                    src_x: slices[0].x,
                    src_y: slices[0].y,
                    src_w: slices[0].w,
                    src_h: slices[0].h,
                });
                Some(())
            })
        {
            let _ = part;
        }
        let mut frame_ids = vec![first];
        for slice in slices.iter().skip(1) {
            let Some(copy) =
                duplicate_frame(&mut self.project, &sprite, frame_ids.last().unwrap()).ok()
            else {
                break;
            };
            if let Some(part) = self
                .project
                .sprites
                .iter_mut()
                .find(|item| item.id == sprite)
                .and_then(|item| item.frames.iter_mut().find(|frame| frame.id == copy))
                .and_then(|frame| frame.parts.first_mut())
            {
                part.src_x = slice.x;
                part.src_y = slice.y;
                part.src_w = slice.w;
                part.src_h = slice.h;
                part.asset_id = asset_id.clone();
            }
            frame_ids.push(copy);
        }
        self.sprite_id = Some(sprite.clone());
        self.frame_id = frame_ids.first().cloned();
        if let Ok(set) = create_set(&mut self.project, &asset.name, &sprite) {
            if let Ok(clip) = add_clip(&mut self.project, &set, "idle") {
                for id in &frame_ids {
                    let _ = append_frame_to_clip(&mut self.project, &set, &clip, id, 8);
                }
                self.set_id = Some(set);
                self.clip_id = Some(clip);
            }
        }
        self.unmaker = UnmakerView::Animate;
        self.touch_anim();
        self.status = format!(
            "Sliced {} into {} frames. A one-frame picture stays a still if you delete the extra frames.",
            asset.name,
            frame_ids.len()
        );
    }

    pub(super) fn player_animation_picker(&mut self, ui: &mut egui::Ui) {
        let Some(player_id) = self
            .project
            .levels
            .get(self.level_sel)
            .and_then(|level| level.player_blueprint_id.clone())
        else {
            return;
        };
        let sets: Vec<(String, String)> = self
            .project
            .anim_sets
            .iter()
            .map(|set| (set.id.clone(), set.name.clone()))
            .collect();
        if sets.is_empty() {
            return;
        }
        let current = self
            .project
            .player_blueprints
            .iter()
            .find(|blueprint| blueprint.id == player_id)
            .map(|blueprint| blueprint.anim_set_id.clone())
            .unwrap_or_default();
        let mut choice = current.clone();
        ui.label("Animation");
        egui::ComboBox::from_id_salt("player-anim")
            .selected_text(set_label(&self.project.anim_sets, &choice))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut choice, String::new(), "Still picture");
                for (id, name) in &sets {
                    ui.selectable_value(&mut choice, id.clone(), name);
                }
            });
        if choice != current {
            match bind_player(&mut self.project, &player_id, &choice) {
                Ok(()) => {
                    self.dirty = true;
                    self.status =
                        "The player animation changed. The physics preset did not.".into();
                }
                Err(err) => self.status = err,
            }
        }
    }
}

fn checker(painter: &egui::Painter, rect: Rect, zoom: f32) {
    let step = (8.0 * zoom).max(8.0);
    let mut y = rect.min.y;
    let mut row = 0;
    while y < rect.max.y {
        let mut x = rect.min.x;
        let mut col = 0;
        while x < rect.max.x {
            let fill = if (row + col) % 2 == 0 {
                Color32::from_rgb(210, 196, 176)
            } else {
                Color32::from_rgb(236, 226, 210)
            };
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(x, y),
                    Vec2::new(step.min(rect.max.x - x), step.min(rect.max.y - y)),
                ),
                0.0,
                fill,
            );
            x += step;
            col += 1;
        }
        y += step;
        row += 1;
    }
}

fn hit_part(frame: &nesticles_core::anim::SpriteFrame, x: f32, y: f32) -> Option<String> {
    let mut parts = frame.parts.clone();
    parts.sort_by(|a, b| b.z.cmp(&a.z));
    for part in parts {
        if !part.visible {
            continue;
        }
        let w = if part.src_w == 0 {
            16
        } else {
            part.src_w as i32
        };
        let h = if part.src_h == 0 {
            16
        } else {
            part.src_h as i32
        };
        if x >= part.x as f32
            && y >= part.y as f32
            && x < (part.x + w) as f32
            && y < (part.y + h) as f32
        {
            return Some(part.id);
        }
    }
    None
}

fn sprite_label(sprites: &[SpriteDef], id: &str) -> String {
    sprites
        .iter()
        .find(|sprite| sprite.id == id)
        .map(|sprite| sprite.name.clone())
        .unwrap_or_else(|| "Choose a sprite".into())
}

fn set_label(sets: &[AnimSet], id: &str) -> String {
    if id.is_empty() {
        return "Still picture".into();
    }
    sets.iter()
        .find(|set| set.id == id)
        .map(|set| set.name.clone())
        .unwrap_or_else(|| "Choose an animation".into())
}
