//! Godot output for generated courses.
//!
//! Nesticles project data stays in Rust. This module writes a Godot project
//! from that data and can launch the pinned Godot 4.7.2 standard editor.
//! The generated game runs in Godot. The Rust course simulator is not attached
//! and is not replaced.
//!
//! The player picture and physics preset come from a Player Blueprint when the
//! course has one. Otherwise the first character picture and the project preset
//! are used. The Godot step follows the 1080p SMB1-style rules for the
//! supported subset. It is not a second set of speeds, and it is not full
//! Rust Play parity.

mod generate;
mod parity;
mod toolchain;

pub use generate::{generate_proof, preview_dir, ProofInfo, CELL_PX};
pub use toolchain::{
    discover, export_native, import_project, launch_preview, linux_template_dir, run_exported,
    run_parity, run_proof, GodotOutput, GodotTool, PINNED_VERSION,
};
