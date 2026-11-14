//! Shared Nesticles workshop: project files, tile codecs, audio, physics, and tools.
//! The native editor and the browser player both call into this crate.

#[cfg(test)]
mod play_tests;

pub mod anim;
pub mod audio;
pub mod behavior;
pub mod pipeline;
pub mod blueprint;
pub mod classify;
pub mod codec;
pub mod demo;
pub mod detect;
pub mod disasm;
pub mod enemy_ai;
pub mod engine;
pub mod godot;
pub mod guard;
pub mod physics;
pub mod player;
pub mod pngutil;
pub mod project;
pub mod terrain;
pub mod tools;
pub mod transition;
pub mod wav;

pub use anim::{
    add_clip, add_part, append_frame_to_clip, apply_standard_fallbacks, asset_image,
    bind_blueprint, bind_player, check_canvas, clip_finished, composite, create_set, create_sprite,
    delete_clip_frame, delete_frame, duplicate_frame, duplicate_part, enemy_pose, grid_parts,
    missing_states, move_part, player_pose, present, rename_clip, reorder_clip, resolve_clip,
    sample, select_frame, set_clip_facing, set_clip_ticks, set_for_blueprint, set_for_player,
    set_part_layer, set_playback, slice_grid, write_project, Anchor, AnimHistory, AnimSet, Clip,
    Part, Playback, PoseClock, Present, Slice, SpriteDef, SpriteFrame, SCHEMA,
};
pub use audio::{decode_brr, decode_dpcm_levels, decode_gb_wave, AudioHit};
pub use blueprint::{
    assign_cell, compose_fixture, enemy_from_archetype, enemy_role, ensure_legacy_blueprints,
    gameplay_label, gameplay_options, gameplay_summary, infer_legacy, load_project_json,
    placed_fixture, placement_label, resolve_warp_arrival, set_placement_warp, Blueprint,
    BlueprintOrigin, FixtureParts, LegacyBlock, LegacyHazard, LegacyItem, LegacyRole,
};
pub use classify::{suggest_kind, Suggestion};
pub use codec::{decode_tile, encode_tile, tile_bytes, Codec};
pub use demo::{demo_project, demo_rom};
pub use detect::{detect_rom, find_strings, RomFormat, RomInfo};
pub use disasm::{disassemble6502, AsmLine};
pub use enemy_ai::{archetype_label, override_label, EnemyAiStyle, EnemyArchetype};
pub use guard::image_prompt_blocked;
pub use physics::{
    play_surface_scale, scale_virtual_px, Aabb, CharacterPhysics, Input, Sim, WarpArrival, TILE,
};
pub use player::{
    apply_resolved, facing_label, new_player_blueprint, power_from_label, power_label,
    resolve_player, PlayerBlueprint, ResolvedPlayer,
};
pub use project::PlacementWarp;
pub use project::{
    Asset, AssetKind, BlockTag, HazardTag, ItemTag, Layer, Level, PowerUpStyle, Project, Style,
    WarpTarget,
};
pub use tools::execute_tool_json;
pub use transition::{
    add_entrance, add_return, clear_exit, clear_named_exit, ensure_legacy_transitions, named_exit,
    remove_entrance, remove_return, set_exit, update_entrance, BakedExit, NamedExit,
};
