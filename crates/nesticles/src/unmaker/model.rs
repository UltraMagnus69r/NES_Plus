//! Decisions for the UnMaker studio. Drawing lives in `view.rs`.
//!
//! A documented tile list is not a reconstructed sprite. R1 never reports a
//! completed animation from a `Candidate`.

use nesticles_core::codec::indices_to_rgba;
use nesticles_core::pipeline::{Candidate, CoverageRow, ReviewStatus};

/// Display colors for source-tile review. None of these is an original palette.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ReviewPalette {
    GrayRamp,
    #[default]
    SpriteA,
    SpriteB,
    Terrain,
}

impl ReviewPalette {
    pub const ALL: [ReviewPalette; 4] = [
        ReviewPalette::GrayRamp,
        ReviewPalette::SpriteA,
        ReviewPalette::SpriteB,
        ReviewPalette::Terrain,
    ];

    pub const NOTE: &'static str = "Review colors only — original palette not yet verified.";

    pub fn label(self) -> &'static str {
        match self {
            Self::GrayRamp => "Gray ramp",
            Self::SpriteA => "Sprite review A",
            Self::SpriteB => "Sprite review B",
            Self::Terrain => "Terrain review",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "gray" | "grey" | "ramp" => Some(Self::GrayRamp),
            "a" | "sprite-a" | "warm" => Some(Self::SpriteA),
            "b" | "sprite-b" | "contrast" => Some(Self::SpriteB),
            "terrain" | "ground" => Some(Self::Terrain),
            _ => None,
        }
    }

    /// Four display colors. Index 0 stays transparent in the preview.
    pub fn colors(self) -> [[u8; 3]; 4] {
        match self {
            Self::GrayRamp => [[0, 0, 0], [96, 96, 96], [168, 168, 168], [248, 248, 248]],
            // Warm browns so separate character pieces stay distinguishable.
            Self::SpriteA => [[0, 0, 0], [80, 40, 16], [216, 112, 48], [248, 224, 176]],
            // Stronger contrast than palette A. Still not a verified game palette.
            Self::SpriteB => [[0, 0, 0], [32, 32, 120], [220, 40, 40], [248, 232, 64]],
            Self::Terrain => [[0, 0, 0], [48, 96, 32], [144, 88, 40], [200, 216, 112]],
        }
    }
}

/// One NES tile in the selected review colors. The tile count is 1, so the picture is 8×8.
pub fn review_tile_rgba(indices: &[u8], palette: ReviewPalette) -> (u32, u32, Vec<u8>) {
    indices_to_rgba(indices, 1, 1, &palette.colors(), true)
}

/// One NES tile as gray RGBA. The tile count is 1, so the picture is 8×8.
pub fn gray_tile_rgba(indices: &[u8]) -> (u32, u32, Vec<u8>) {
    review_tile_rgba(indices, ReviewPalette::GrayRamp)
}

pub const NOT_RECONSTRUCTED: &str =
    "UnMaker has identified this character but has not reconstructed its artwork yet.";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Home,
    Import,
    Investigate,
    Discover,
    Review,
    Create,
    Save,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Import => "Import",
            Self::Investigate => "Investigate",
            Self::Discover => "Discover",
            Self::Review => "Review",
            Self::Create => "Create",
            Self::Save => "Save",
        }
    }

    pub fn journey() -> [Stage; 6] {
        [
            Self::Import,
            Self::Investigate,
            Self::Discover,
            Self::Review,
            Self::Create,
            Self::Save,
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Players,
    Enemies,
    Bosses,
    Items,
    Blocks,
    Terrain,
    Backgrounds,
    Other,
    Unidentified,
    Ignored,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Self::Players => "Players",
            Self::Enemies => "Enemies",
            Self::Bosses => "Bosses",
            Self::Items => "Items",
            Self::Blocks => "Blocks",
            Self::Terrain => "Terrain",
            Self::Backgrounds => "Backgrounds",
            Self::Other => "Other",
            Self::Unidentified => "Unidentified",
            Self::Ignored => "Ignored",
        }
    }

    pub fn all() -> [Group; 10] {
        [
            Self::Players,
            Self::Enemies,
            Self::Bosses,
            Self::Items,
            Self::Blocks,
            Self::Terrain,
            Self::Backgrounds,
            Self::Other,
            Self::Unidentified,
            Self::Ignored,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct StudioState {
    pub stage: Stage,
    pub filter: Option<Group>,
    pub selected: Option<String>,
    pub issue_index: usize,
    pub notice: String,
    pub zoom: f32,
    pub evidence_index: usize,
    pub show_technical: bool,
    pub show_manual: bool,
    pub create_animate: bool,
    pub stage_log: Vec<String>,
    pub investigated: bool,
    /// Previous tile, audio, library, and export screens. Not the studio path.
    pub legacy_tools: bool,
    /// Display colors for CHR evidence. Not stored in the project.
    pub review_palette: ReviewPalette,
}

impl Default for StudioState {
    fn default() -> Self {
        Self {
            stage: Stage::Home,
            filter: None,
            selected: None,
            issue_index: 0,
            notice: String::new(),
            zoom: 8.0,
            evidence_index: 0,
            show_technical: false,
            show_manual: false,
            create_animate: false,
            stage_log: Vec::new(),
            investigated: false,
            legacy_tools: false,
            review_palette: ReviewPalette::SpriteA,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryItem {
    pub id: String,
    pub name: String,
    pub group: Group,
    pub status: String,
    pub form: String,
    pub frame_labels: Vec<String>,
    pub tile_ids: Vec<u32>,
    pub needs_user: bool,
    pub reconstructed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueCard {
    pub candidate_id: Option<String>,
    pub name: String,
    pub uncertainty: String,
    pub evidence: String,
    pub attempted: String,
    pub next_action: String,
    pub suggestion: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionFeedback {
    pub message: String,
    pub published: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPolicy {
    pub can_play: bool,
    pub message: String,
    pub timing_label: String,
    pub palette_label: String,
}

pub fn continue_available(rom_open: bool, session: bool, job: bool, sprites: bool) -> bool {
    rom_open || session || job || sprites
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn nav_fits(width: f32, height: f32) -> bool {
    let _ = width;
    let row = 44.0;
    let chrome = 56.0 + 36.0;
    height - chrome >= Stage::journey().len() as f32 * row
}

/// `FrameDraft` values are proposals. R2 is what turns them into sprite frames.
pub fn reconstructed_frame_count(_candidate: &Candidate) -> usize {
    0
}

pub fn preview_policy(reconstructed_frames: usize, timing_verified: bool) -> PreviewPolicy {
    if reconstructed_frames == 0 {
        return PreviewPolicy {
            can_play: false,
            message: NOT_RECONSTRUCTED.into(),
            timing_label: "No reconstructed frames, so there is no timing to play.".into(),
            palette_label: ReviewPalette::NOTE.into(),
        };
    }
    PreviewPolicy {
        can_play: reconstructed_frames > 1,
        message: String::new(),
        timing_label: if timing_verified {
            "Frame durations come from the asset.".into()
        } else {
            "Timing is provisional. It is not original NES timing.".into()
        },
        palette_label: "Palette provenance is stored on the asset.".into(),
    }
}

pub fn gallery_items(candidates: &[Candidate], coverage: &[CoverageRow]) -> Vec<GalleryItem> {
    let mut items = Vec::new();
    let mut used = Vec::new();
    for row in coverage {
        let found = candidates
            .iter()
            .find(|candidate| candidate.name == row.name);
        if let Some(candidate) = found {
            used.push(candidate.id.clone());
        }
        items.push(item_from(row.name.clone(), row.category.as_str(), found));
    }
    for candidate in candidates {
        if used.iter().any(|id| id == &candidate.id) {
            continue;
        }
        items.push(item_from(
            candidate.name.clone(),
            candidate.category.as_str(),
            Some(candidate),
        ));
    }
    items
}

fn item_from(name: String, category: &str, candidate: Option<&Candidate>) -> GalleryItem {
    let ignored = candidate.is_some_and(|candidate| candidate.status == ReviewStatus::Ignored);
    let group = if ignored {
        Group::Ignored
    } else {
        group_for(category)
    };
    let (id, status, form, frame_labels, tile_ids, needs_user) = if let Some(candidate) = candidate
    {
        (
            candidate.id.clone(),
            candidate.status.label().to_string(),
            if candidate.form.is_empty() {
                "one proposed form".into()
            } else {
                candidate.form.clone()
            },
            candidate
                .frames
                .iter()
                .map(|frame| frame.state.clone())
                .collect(),
            candidate.tile_ids(),
            candidate.status != ReviewStatus::Approved,
        )
    } else {
        (
            format!("row:{name}"),
            "No tile mapping".into(),
            "no form".into(),
            Vec::new(),
            Vec::new(),
            true,
        )
    };
    GalleryItem {
        id,
        name,
        group,
        status,
        form,
        frame_labels,
        tile_ids,
        needs_user,
        reconstructed: false,
    }
}

fn group_for(category: &str) -> Group {
    match category {
        "player" => Group::Players,
        "enemy" => Group::Enemies,
        "boss" => Group::Bosses,
        "item" | "power-up" | "projectile" => Group::Items,
        "block" => Group::Blocks,
        "terrain" | "platform" => Group::Terrain,
        "decoration" | "background" => Group::Backgrounds,
        "other" | "hud" | "interactive" => Group::Other,
        _ => Group::Unidentified,
    }
}

pub fn issue_queue(candidates: &[Candidate], coverage: &[CoverageRow]) -> Vec<IssueCard> {
    let mut cards = Vec::new();
    for item in gallery_items(candidates, coverage) {
        let candidate = candidates.iter().find(|candidate| candidate.id == item.id);
        let unresolved = item.tile_ids.is_empty();
        cards.push(IssueCard {
            candidate_id: candidate.map(|candidate| candidate.id.clone()),
            name: item.name.clone(),
            uncertainty: if unresolved {
                format!(
                    "{} has no documented tile mapping in this profile.",
                    item.name
                )
            } else {
                format!(
                    "{} has proposed tile indexes only. Those indexes are not an assembled sprite.",
                    item.name
                )
            },
            evidence: if item.tile_ids.is_empty() {
                "No source tiles are attached. The name comes from the coverage list.".into()
            } else {
                format!(
                    "Proposed tiles: {}. Confidence: {}. Palette is unverified.",
                    item.tile_ids
                        .iter()
                        .map(|tile| format!("0x{tile:02X}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                    candidate
                        .map(|candidate| candidate.confidence.as_str())
                        .unwrap_or("none")
                )
            },
            attempted: candidate
                .map(|candidate| {
                    if candidate.attempts.is_empty() {
                        "The profile proposed this grouping. No later pass has changed it.".into()
                    } else {
                        format!(
                            "{} recorded attempt(s). The latest note: {}",
                            candidate.attempts.len(),
                            candidate
                                .attempts
                                .last()
                                .map(|attempt| attempt.note.as_str())
                                .unwrap_or("none")
                        )
                    }
                })
                .unwrap_or_else(|| "No extraction candidate was created.".into()),
            next_action: if unresolved {
                "There is no artwork to fix. Skip this, or leave it unresolved.".into()
            } else {
                "You can mark the candidate reviewed or place tiles by hand. Neither one publishes an animation.".into()
            },
            suggestion: None,
        });
    }
    cards.sort_by_key(|card| !card.evidence.starts_with("No source"));
    cards
}

pub fn approve_feedback(name: &str, result: Result<(), &str>) -> ActionFeedback {
    let message = match result {
        Ok(()) => format!(
            "Review status for {name} is now Approved. This did not create a sprite, an animation, or a library package."
        ),
        Err(err) => format!("Could not mark {name} reviewed. {err}"),
    };
    ActionFeedback {
        message,
        published: false,
    }
}

pub fn ignore_feedback(name: &str) -> ActionFeedback {
    ActionFeedback {
        message: format!("Moved {name} to Ignored. It was not deleted."),
        published: false,
    }
}

pub fn skip_feedback(name: &str) -> ActionFeedback {
    ActionFeedback {
        message: format!("Skipped {name}. Nothing was changed."),
        published: false,
    }
}

pub fn restore_feedback(name: &str) -> ActionFeedback {
    ActionFeedback {
        message: format!("{name} is back in the gallery. It was not published."),
        published: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nesticles_core::pipeline::{FrameDraft, TileSpot};

    fn goomba() -> Candidate {
        Candidate {
            id: "cand-goomba".into(),
            name: "Goomba".into(),
            category: "enemy".into(),
            target_style: String::new(),
            collection: String::new(),
            character_id: "goomba".into(),
            form: "enemy".into(),
            frames: vec![FrameDraft {
                state: "walk".into(),
                origin: "documented".into(),
                spots: vec![TileSpot {
                    tile: 0x70,
                    x: 0,
                    y: 0,
                    flip_x: false,
                    flip_y: false,
                }],
            }],
            palette: 0,
            status: ReviewStatus::Awaiting,
            fail_reason: String::new(),
            confidence: "documented profile".into(),
            evidence: "EnemyGraphicsTable".into(),
            attempts: Vec::new(),
            rejected_tiles: Vec::new(),
        }
    }

    #[test]
    fn a_fresh_launch_has_no_continue_button() {
        assert!(!continue_available(false, false, false, false));
        assert!(continue_available(true, false, false, false));
    }

    #[test]
    fn the_stage_list_fits_the_target_screen() {
        assert!(nav_fits(1366.0, 768.0));
    }

    #[test]
    fn a_named_goomba_is_not_a_reconstructed_sprite() {
        let candidate = goomba();
        assert_eq!(reconstructed_frame_count(&candidate), 0);
        let items = gallery_items(
            &[candidate],
            &[CoverageRow {
                name: "Goomba".into(),
                category: "enemy".into(),
                raw: "identified".into(),
                assembly: "proposed".into(),
                animation: "proposed".into(),
                palette: "unverified".into(),
                review: "awaiting".into(),
            }],
        );
        assert_eq!(items.len(), 1);
        assert!(!items[0].reconstructed);
        assert_eq!(items[0].group, Group::Enemies);
        assert_eq!(items[0].tile_ids, vec![0x70]);
    }

    #[test]
    fn mario_without_tiles_stays_unresolved() {
        let items = gallery_items(
            &[],
            &[CoverageRow {
                name: "Mario".into(),
                category: "player".into(),
                raw: "identified".into(),
                assembly: "unresolved".into(),
                animation: "unresolved".into(),
                palette: "unverified".into(),
                review: "unresolved".into(),
            }],
        );
        assert_eq!(items[0].group, Group::Players);
        assert!(items[0].tile_ids.is_empty());
        assert!(items[0].needs_user);
        let issues = issue_queue(&[], &items_to_coverage(&items));
        assert!(issues[0].suggestion.is_none());
        assert!(issues[0].uncertainty.contains("no documented tile mapping"));
    }

    fn items_to_coverage(items: &[GalleryItem]) -> Vec<CoverageRow> {
        items
            .iter()
            .map(|item| CoverageRow {
                name: item.name.clone(),
                category: "player".into(),
                raw: String::new(),
                assembly: "unresolved".into(),
                animation: "unresolved".into(),
                palette: "unverified".into(),
                review: "unresolved".into(),
            })
            .collect()
    }

    #[test]
    fn a_source_tile_is_eight_by_eight() {
        let (w, h, rgba) = gray_tile_rgba(&[0u8; 64]);
        assert_eq!((w, h, rgba.len()), (8, 8, 256));
    }

    #[test]
    fn review_palettes_change_the_visible_color() {
        let indices = [1u8; 64];
        let gray = review_tile_rgba(&indices, ReviewPalette::GrayRamp).2;
        let warm = review_tile_rgba(&indices, ReviewPalette::SpriteA).2;
        let contrast = review_tile_rgba(&indices, ReviewPalette::SpriteB).2;
        let terrain = review_tile_rgba(&indices, ReviewPalette::Terrain).2;
        assert_ne!(gray, warm);
        assert_ne!(warm, contrast);
        assert_ne!(contrast, terrain);
        assert_eq!(&warm[0..3], &[80, 40, 16]);
        assert!(ReviewPalette::NOTE.contains("Review colors only"));
        assert!(ReviewPalette::NOTE.contains("not yet verified"));
        assert_eq!(ReviewPalette::parse("b"), Some(ReviewPalette::SpriteB));
    }

    #[test]
    fn an_empty_preview_does_not_play() {
        let policy = preview_policy(0, false);
        assert!(!policy.can_play);
        assert_eq!(policy.message, NOT_RECONSTRUCTED);
        assert!(policy.timing_label.contains("No reconstructed frames"));
    }

    #[test]
    fn approval_is_not_described_as_a_library_save() {
        let feedback = approve_feedback("Goomba", Ok(()));
        assert!(!feedback.published);
        let lower = feedback.message.to_ascii_lowercase();
        assert!(!lower.contains("saved to library"));
        assert!(lower.contains("did not create a sprite"));
        let failed = approve_feedback("Mario", Err("There is nothing to approve."));
        assert!(!failed.published);
        assert!(failed.message.contains("nothing to approve"));
    }

    #[test]
    fn ignore_and_skip_say_what_changed() {
        assert!(ignore_feedback("Goomba").message.contains("Ignored"));
        assert!(skip_feedback("Luigi")
            .message
            .contains("Nothing was changed"));
        assert!(!restore_feedback("Goomba").published);
    }
}
