//! Decisions for the UnMaker studio. Drawing lives in `view.rs`.
//!
//! A documented tile list is not a reconstructed sprite. Discover may say
//! "Reconstructed" only after `reconstruct::assemble` accepts the candidate.

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

/// One NES tile as gray RGBA. The tile count is 1, so the picture is 8×8.
pub fn gray_tile_rgba(indices: &[u8]) -> (u32, u32, Vec<u8>) {
    indices_to_rgba(indices, 1, 1, &ReviewPalette::GrayRamp.colors(), true)
}

/// One NES tile in the selected review colors. The tile count is 1, so the picture is 8×8.
pub fn review_tile_rgba(indices: &[u8], palette: ReviewPalette) -> (u32, u32, Vec<u8>) {
    if palette == ReviewPalette::GrayRamp {
        return gray_tile_rgba(indices);
    }
    indices_to_rgba(indices, 1, 1, &palette.colors(), true)
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

/// An older utility, opened from the stage that owns that job.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StudioTool {
    Tiles,
    Audio,
    Library,
    Export,
    Notes,
    Overworld,
    Providers,
    Behavior,
    Guide,
}

impl StudioTool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tiles => "Source tiles",
            Self::Audio => "Audio",
            Self::Library => "Library",
            Self::Export => "Export package",
            Self::Notes => "Notes",
            Self::Overworld => "Overworld",
            Self::Providers => "AI providers",
            Self::Behavior => "Behavior graph",
            Self::Guide => "Guide",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            Self::Tiles => "Raw cartridge tiles. Looking at them does not assemble a character.",
            Self::Audio => {
                "Sounds stored in the project. Playback here is inspection, not a new soundtrack."
            }
            Self::Library => {
                "Local alpha packages and recipes. Opening one does not reconstruct a sprite."
            }
            Self::Export => {
                "Package export still paints the first frame only. It is not a finished character."
            }
            Self::Notes => "Notes stay in the project. They are not sent anywhere.",
            Self::Overworld => {
                "The existing overworld list. It does not build a new map by itself."
            }
            Self::Providers => {
                "Provider settings. This screen does not call a model unless you send a request."
            }
            Self::Behavior => {
                "Existing behavior graph. Patrol and jump labels are not a second physics model."
            }
            Self::Guide => "Short explanations of the workshop. They do not change the cartridge.",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "tiles" | "source" => Some(Self::Tiles),
            "audio" => Some(Self::Audio),
            "library" => Some(Self::Library),
            "export" => Some(Self::Export),
            "notes" => Some(Self::Notes),
            "overworld" => Some(Self::Overworld),
            "providers" | "ai" => Some(Self::Providers),
            "behavior" => Some(Self::Behavior),
            "guide" => Some(Self::Guide),
            "assemble" | "animate" => None,
            _ => None,
        }
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
    /// Display colors for CHR evidence. Not stored in the project.
    pub review_palette: ReviewPalette,
    /// An existing utility opened inside the current stage. Not a second application.
    pub tool: Option<StudioTool>,
    /// Walk or defeat. Playback reads the assembled frames, not a separate animation.
    pub preview_pose: String,
    pub preview_playing: bool,
    pub preview_elapsed: u32,
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
            review_palette: ReviewPalette::SpriteA,
            tool: None,
            preview_pose: "walk".into(),
            preview_playing: false,
            preview_elapsed: 0,
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

/// Discover's view of one investigation. This is not a second sprite model.
/// `Ok` means `assemble` produced the canonical Goomba. `Err` is the reason it
/// refused. `None` means the session has no Goomba candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoombaDiscovery {
    pub candidate_id: String,
    pub walk_frames: usize,
    pub defeat_frames: usize,
    pub can_play: bool,
    pub width: u32,
    pub height: u32,
}

pub fn discover_goomba(
    rom: &[u8],
    payload_sha256: &str,
    candidates: &[Candidate],
    palette: &[[u8; 3]; 4],
) -> Option<Result<GoombaDiscovery, String>> {
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.character_id == "goomba")?;
    Some(
        match nesticles_core::reconstruct::assemble(rom, payload_sha256, candidate, palette) {
            Ok(assembly) => {
                let walk_frames = assembly
                    .frames
                    .iter()
                    .filter(|frame| frame.state == "walk")
                    .count();
                let defeat_frames = assembly
                    .frames
                    .iter()
                    .filter(|frame| frame.state == "defeat")
                    .count();
                let (width, height) = assembly
                    .frames
                    .first()
                    .map(|frame| (frame.w, frame.h))
                    .unwrap_or((0, 0));
                Ok(GoombaDiscovery {
                    candidate_id: candidate.id.clone(),
                    walk_frames,
                    defeat_frames,
                    can_play: walk_frames > 1,
                    width,
                    height,
                })
            }
            Err(err) => Err(err),
        },
    )
}

/// One animation state inside a form. The `id` matches the clip and pose name
/// already stored on the sprite. The label is only display text.
///
/// A later separate object, such as a moving shell, would be another state or
/// a linked id. R2.2 does not store that link and does not add gameplay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterStateView {
    pub id: String,
    pub label: String,
    pub frames: usize,
    pub looping: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterFormView {
    pub id: String,
    pub label: String,
    pub states: Vec<CharacterStateView>,
}

/// How Review presents one character. This is not a second sprite or animation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterSheet {
    pub candidate_id: String,
    pub name: String,
    pub status: String,
    pub saved: bool,
    pub forms: Vec<CharacterFormView>,
}

impl CharacterSheet {
    pub fn form_count(&self) -> usize {
        self.forms.len()
    }

    pub fn state_count(&self) -> usize {
        self.forms.iter().map(|form| form.states.len()).sum()
    }
}

/// Display name for a pose id. Unknown ids stay as stored. This does not
/// invent graphics for Mario forms or Koopa shell states.
pub fn label_for_state(id: &str) -> String {
    match id {
        "walk" => "Walk".into(),
        "defeat" => "Defeat / Stomped".into(),
        other => other.to_string(),
    }
}

fn goomba_states(found: &GoombaDiscovery) -> Vec<CharacterStateView> {
    let mut states = vec![CharacterStateView {
        id: "walk".into(),
        label: label_for_state("walk"),
        frames: found.walk_frames,
        looping: true,
    }];
    if found.defeat_frames > 0 {
        states.push(CharacterStateView {
            id: "defeat".into(),
            label: label_for_state("defeat"),
            frames: found.defeat_frames,
            looping: false,
        });
    }
    states
}

pub fn form_label(id: &str) -> String {
    if id.is_empty() || id == "default" {
        "Default".into()
    } else {
        id.to_string()
    }
}

/// The sheet exists only after assemble accepts this candidate.
pub fn character_sheet(
    candidate: &Candidate,
    discovery: Option<&GoombaDiscovery>,
    saved: bool,
) -> Option<CharacterSheet> {
    let found = discovery?;
    if candidate.id != found.candidate_id || candidate.character_id != "goomba" {
        return None;
    }
    if found.walk_frames < 2 {
        return None;
    }
    Some(CharacterSheet {
        candidate_id: candidate.id.clone(),
        name: candidate.name.clone(),
        status: "Reconstructed".into(),
        saved,
        forms: vec![CharacterFormView {
            id: candidate.form.clone(),
            label: form_label(&candidate.form),
            states: goomba_states(found),
        }],
    })
}

pub fn approve_label(name: &str) -> String {
    format!("Approve & Save {name}")
}

pub fn save_status_line(saved: bool) -> &'static str {
    if saved {
        "Saved in the project. Reopen does not need the cartridge."
    } else {
        "Not saved yet."
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryCard {
    pub title: String,
    pub status: String,
    pub summary: String,
    pub needs_person: bool,
    pub open_label: String,
}

/// Compact Discover text. A reconstructed Goomba is one card, not a workspace.
pub fn gallery_card(
    item: &GalleryItem,
    is_goomba: bool,
    discovery: Option<&Result<GoombaDiscovery, String>>,
) -> GalleryCard {
    if let Some(Ok(found)) = discovery {
        if item.id == found.candidate_id {
            let states = goomba_states(found);
            let forms = 1;
            let state_count = states.len();
            let saved = !item.needs_user;
            return GalleryCard {
                title: item.name.clone(),
                status: if saved {
                    "Reconstructed · Saved".into()
                } else {
                    "Reconstructed".into()
                },
                summary: format!("{forms} form · {state_count} states"),
                needs_person: false,
                open_label: format!("Open {} in Review", item.name),
            };
        }
    }
    if is_goomba {
        if let Some(Err(err)) = discovery {
            return GalleryCard {
                title: item.name.clone(),
                status: err.clone(),
                summary: "Reconstruction failed".into(),
                needs_person: true,
                open_label: format!("Open {} in Review", item.name),
            };
        }
    }
    let states = item.frame_labels.len();
    let forms = usize::from(states > 0);
    GalleryCard {
        title: item.name.clone(),
        status: if item.needs_user {
            "Artwork not reconstructed · Needs a person".into()
        } else if item.reconstructed {
            "Saved in the project".into()
        } else {
            "Reviewed, not published".into()
        },
        summary: if forms == 0 {
            "No form yet".into()
        } else {
            format!("{forms} form · {states} states")
        },
        needs_person: item.needs_user,
        open_label: format!("Open {} in Review", item.name),
    }
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
            } else if candidate.is_some_and(|candidate| candidate.character_id == "goomba") {
                "Approve and save writes the assembled Goomba when this is the verified SMB1 payload. A review flag by itself does not save it.".into()
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

    #[test]
    fn investigation_exposes_the_goomba_to_discover_without_a_launch_hook() {
        let bytes = std::fs::read("/home/g/Desktop/Roms/smb1.nes").expect("owner ROM");
        let session = nesticles_core::pipeline::propose_session(&bytes, "cartridge.nes", &[], &[])
            .expect("the normal investigation session");
        let palette = ReviewPalette::SpriteA.colors();
        let discovery = discover_goomba(
            &bytes,
            &session.report.payload_sha256,
            &session.candidates,
            &palette,
        )
        .expect("the SMB1 session has a Goomba candidate")
        .expect("assemble accepts that candidate");
        assert!(discovery.can_play);
        assert_eq!(discovery.walk_frames, 2);
        assert_eq!(discovery.defeat_frames, 1);
        assert_eq!((discovery.width, discovery.height), (16, 24));
        let items = gallery_items(&session.candidates, &session.coverage);
        let goomba_item = items.iter().find(|item| item.name == "Goomba").unwrap();
        let card = gallery_card(goomba_item, true, Some(&Ok(discovery.clone())));
        assert_eq!(card.title, "Goomba");
        assert_eq!(card.status, "Reconstructed");
        assert_eq!(card.summary, "1 form · 2 states");
        assert!(!card.needs_person);
        assert_eq!(card.open_label, "Open Goomba in Review");
        assert!(!card.status.contains("Artwork not reconstructed"));
        let candidate = session
            .candidates
            .iter()
            .find(|candidate| candidate.id == discovery.candidate_id)
            .unwrap();
        let sheet = character_sheet(candidate, Some(&discovery), false).unwrap();
        assert_eq!(sheet.form_count(), 1);
        assert_eq!(sheet.state_count(), 2);
        assert_eq!(sheet.forms[0].label, "Default");
        assert_eq!(sheet.forms[0].states[0].label, "Walk");
        assert_eq!(sheet.forms[0].states[0].frames, 2);
        assert!(sheet.forms[0].states[0].looping);
        assert_eq!(sheet.forms[0].states[1].label, "Defeat / Stomped");
        assert_eq!(sheet.forms[0].states[1].frames, 1);
        assert!(!sheet.forms[0].states[1].looping);
        assert_eq!(approve_label(&sheet.name), "Approve & Save Goomba");
        assert_eq!(save_status_line(false), "Not saved yet.");
        let saved = character_sheet(candidate, Some(&discovery), true).unwrap();
        assert!(saved.saved);
        assert_eq!(
            save_status_line(true),
            "Saved in the project. Reopen does not need the cartridge."
        );
        let mario = items.iter().find(|item| item.name == "Mario").unwrap();
        let mario_card = gallery_card(mario, false, Some(&Ok(discovery)));
        assert!(mario_card.status.contains("Artwork not reconstructed"));
        assert!(mario_card.needs_person);
        assert_eq!(mario_card.open_label, "Open Mario in Review");
    }

    #[test]
    fn a_koopa_proposal_does_not_invent_shell_states() {
        let koopa = nesticles_core::pipeline::smb1_profile()
            .candidates
            .into_iter()
            .find(|candidate| candidate.character_id == "koopa")
            .unwrap();
        assert!(character_sheet(&koopa, None, false).is_none());
        assert!(koopa.frames.iter().all(|frame| frame.state != "shell"));
        assert_eq!(label_for_state("walk-b"), "walk-b");
    }

    #[test]
    fn an_unmatched_cartridge_does_not_invent_a_goomba() {
        let rom = nesticles_core::pipeline::synthetic_fixture();
        let session =
            nesticles_core::pipeline::propose_session(&rom, "synthetic-fixture.nes", &[], &[])
                .unwrap();
        let palette = ReviewPalette::SpriteA.colors();
        assert!(discover_goomba(
            &rom,
            &session.report.payload_sha256,
            &session.candidates,
            &palette,
        )
        .is_none());
        let goomba = nesticles_core::pipeline::smb1_profile()
            .candidates
            .into_iter()
            .find(|candidate| candidate.character_id == "goomba")
            .unwrap();
        let item = gallery_items(
            std::slice::from_ref(&goomba),
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
        let failed = discover_goomba(
            &rom,
            &session.report.payload_sha256,
            std::slice::from_ref(&goomba),
            &palette,
        )
        .unwrap()
        .unwrap_err();
        let card = gallery_card(&item[0], true, Some(&Err(failed.clone())));
        assert_eq!(card.status, failed);
        assert!(!card.status.contains("Reconstructed"));
        assert_ne!(card.open_label, approve_label("Goomba"));
    }
}
