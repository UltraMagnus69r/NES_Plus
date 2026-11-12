//! Shared Nesticles workshop: project files, tile codecs, audio, physics, and tools.
//! The native editor and the browser player both call into this crate.

#[cfg(test)]
mod play_tests;

pub mod audio;
pub mod blueprint;
pub mod classify;
pub mod codec;
pub mod demo;
pub mod detect;
pub mod disasm;
pub mod enemy_ai;
pub mod engine;
pub mod guard;
pub mod physics;
pub mod pngutil;
pub mod project;
pub mod tools;
pub mod wav;

pub use audio::{decode_brr, decode_dpcm_levels, decode_gb_wave, AudioHit};
pub use blueprint::{
    assign_cell, enemy_from_archetype, enemy_role, ensure_legacy_blueprints, gameplay_label,
    gameplay_options, infer_legacy, load_project_json, placement_label, Blueprint, BlueprintOrigin,
    LegacyRole,
};
pub use classify::{suggest_kind, Suggestion};
pub use codec::{decode_tile, encode_tile, tile_bytes, Codec};
pub use demo::{demo_project, demo_rom};
pub use detect::{detect_rom, find_strings, RomFormat, RomInfo};
pub use disasm::{disassemble6502, AsmLine};
pub use enemy_ai::{archetype_label, override_label, EnemyAiStyle, EnemyArchetype};
pub use guard::image_prompt_blocked;
pub use physics::{play_surface_scale, scale_virtual_px, Aabb, CharacterPhysics, Input, Sim, TILE};
pub use project::{
    Asset, AssetKind, BlockTag, HazardTag, ItemTag, Layer, Level, PowerUpStyle, Project, Style,
    WarpTarget,
};
pub use tools::execute_tool_json;
