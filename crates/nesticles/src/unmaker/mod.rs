//! UnMaker Lab studio.
//!
//! The extraction session stays in `ExtractUi` until R2 writes a `SpriteDef`.
//! Saving the project does not store that session or the ROM. A review status
//! is not a published asset.

mod model;
mod view;

pub use model::Stage;
pub use view::StudioUi;
