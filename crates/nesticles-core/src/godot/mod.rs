//! Godot output for one playable proof scene.
//!
//! Nesticles project data stays in Rust. This module writes a Godot project
//! from that data and can launch the pinned Godot 4.7.2 standard editor.
//! The generated game runs in Godot. The Rust course simulator is not attached
//! and is not replaced.
//!
//! The player in this proof is a temporary adapter over the course spawn and
//! the project character preset. It is not a Player Blueprint. Full player
//! authoring remains a later requirement.

mod generate;
mod toolchain;

pub use generate::{generate_proof, preview_dir, ProofInfo, CELL_PX};
pub use toolchain::{
    discover, export_native, import_project, launch_preview, linux_template_dir, run_exported,
    run_proof, GodotOutput, GodotTool, PINNED_VERSION,
};
