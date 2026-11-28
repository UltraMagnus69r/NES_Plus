//! R4 reference intelligence: asset-linked references, deterministic compare notes,
//! and AI send gates. References are evidence for the owner. They are not approved
//! reconstructions. AI suggestions never write SpriteDef or AnimSet by themselves.

use crate::pngutil;
use crate::project::nid;
use crate::reference::{http_status_error, local_reference, RefClass, RefImage, IMAGE_BYTE_LIMIT};
use serde::{Deserialize, Serialize};

/// How the reference entered the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSourceKind {
    LocalFile,
    ImageUrl,
    PageUrl,
    SessionOnly,
}

impl ReferenceSourceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::LocalFile => "Local file",
            Self::ImageUrl => "Direct image URL",
            Self::PageUrl => "Webpage URL",
            Self::SessionOnly => "Session only",
        }
    }
}

/// Whether the association claims verified origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceConfidence {
    /// Owner brought the file or URL. Not a verified original-game source.
    UserSupplied,
    /// Independently observed by the app (for example a fetched HTTPS image).
    Observed,
    /// Computed or guessed from other data.
    Inferred,
    /// Explicitly uncertain.
    Uncertain,
}

impl ReferenceConfidence {
    pub fn label(self) -> &'static str {
        match self {
            Self::UserSupplied => "User-supplied",
            Self::Observed => "Independently observed",
            Self::Inferred => "Inferred",
            Self::Uncertain => "Uncertain",
        }
    }
}

/// One reference linked to a stable project asset id (sprite, candidate, or asset).
/// Old projects omit this list and load empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetReference {
    pub id: String,
    /// Stable asset / candidate / sprite identity. Never a gallery index alone.
    pub asset_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub source_kind: ReferenceSourceKind,
    #[serde(default)]
    pub source_url: String,
    /// Basename or relative hint only. Absolute Fedora paths are not required to recover
    /// when `png_base64` is present.
    #[serde(default)]
    pub source_hint: String,
    pub confidence: ReferenceConfidence,
    /// Optional embedded preview/copy inside the project file (not GitHub fixtures).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
    /// When false, reopening needs the external file or a re-fetch.
    #[serde(default = "default_true")]
    pub embedded: bool,
    /// ISO-8601-ish stamp when added. Empty on older records.
    #[serde(default)]
    pub added_at: String,
    /// Active selection for side-by-side compare.
    #[serde(default)]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

/// AI suggestion stored beside a reference. Never auto-applied to reconstruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AiAssistRecord {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub submitted_summary: String,
    #[serde(default)]
    pub response_text: String,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub error: String,
    /// Always true in saved records: output is a suggestion, not verified evidence.
    #[serde(default = "default_true")]
    pub suggestion_only: bool,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationKind {
    Observed,
    Derived,
    Uncertain,
}

impl ObservationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Observed => "Observed",
            Self::Derived => "Derived",
            Self::Uncertain => "Uncertain",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompareObservation {
    pub kind: ObservationKind,
    pub title: String,
    pub detail: String,
}

/// Preconditions for an AI network call. No request is made here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiSendRequest {
    pub has_credential: bool,
    pub user_confirmed: bool,
    pub explicit_send: bool,
    pub image_bytes: usize,
    pub notes: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiSendBlock {
    MissingCredential,
    NotConfirmed,
    NoExplicitSend,
    EmptyPayload,
}

impl AiSendBlock {
    pub fn message(self) -> &'static str {
        match self {
            Self::MissingCredential => {
                "AI is unavailable until a credential is configured locally."
            }
            Self::NotConfirmed => {
                "Confirm what will be sent before analyzing. The ROM is never attached."
            }
            Self::NoExplicitSend => {
                "AI runs only after an explicit Send to AI action. Opening a project does not send."
            }
            Self::EmptyPayload => "There is nothing to send. Import or select a reference first.",
        }
    }
}

pub fn gate_ai_send(req: &AiSendRequest) -> Result<(), AiSendBlock> {
    if !req.has_credential {
        return Err(AiSendBlock::MissingCredential);
    }
    if !req.explicit_send {
        return Err(AiSendBlock::NoExplicitSend);
    }
    if !req.user_confirmed {
        return Err(AiSendBlock::NotConfirmed);
    }
    if req.image_bytes == 0 && req.notes.trim().is_empty() {
        return Err(AiSendBlock::EmptyPayload);
    }
    Ok(())
}

/// Summarize the payload the owner would transmit. Does not include secrets.
pub fn describe_ai_payload(req: &AiSendRequest) -> String {
    let image = if req.image_bytes == 0 {
        "no image".into()
    } else {
        format!("{} image bytes", req.image_bytes)
    };
    let notes = if req.notes.trim().is_empty() {
        "no notes".into()
    } else {
        format!("{} note characters", req.notes.trim().chars().count())
    };
    format!(
        "Provider material: {image}; {notes}; model {}. ROM bytes and unrelated project files are not included.",
        if req.model.trim().is_empty() {
            "(model unset)"
        } else {
            req.model.trim()
        }
    )
}

/// Validate image bytes for a local or downloaded reference.
pub fn validate_reference_bytes(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    if bytes.is_empty() {
        return Err("That file is empty.".into());
    }
    if bytes.len() > IMAGE_BYTE_LIMIT {
        return Err(format!(
            "The image is {} bytes, above the {} byte limit. It was not kept.",
            bytes.len(),
            IMAGE_BYTE_LIMIT
        ));
    }
    #[cfg(feature = "png")]
    {
        let (w, h, rgba, _, _) = pngutil::reference_preview(bytes, 1024)?;
        Ok((w, h, rgba))
    }
    #[cfg(not(feature = "png"))]
    {
        let _ = bytes;
        Err("PNG support is not enabled in this build.".into())
    }
}

/// Reject unsupported extensions before decode when a path is known.
pub fn supported_image_extension(path: &str) -> bool {
    let lower = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

pub fn unsupported_image_message(path: &str) -> String {
    format!(
        "Unsupported image format for `{path}`. Use PNG, JPEG, GIF, or WebP."
    )
}

pub fn missing_source_message(hint: &str) -> String {
    if hint.is_empty() {
        "The reference source is missing. Import the file again or choose another reference."
            .into()
    } else {
        format!(
            "The external reference `{hint}` is missing. The association remains, but the image cannot be shown until the file is restored or re-imported."
        )
    }
}

pub fn url_fetch_error(stage: &str, status: u16) -> String {
    http_status_error(stage, status)
}

/// Build a persisted reference from decoded local image bytes.
pub fn reference_from_local(
    asset_id: &str,
    title: &str,
    path: &str,
    bytes: &[u8],
    embed: bool,
) -> Result<AssetReference, String> {
    if !path.is_empty() && !supported_image_extension(path) {
        return Err(unsupported_image_message(path));
    }
    let (w, h, _rgba) = validate_reference_bytes(bytes)?;
    let hint = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_string();
    let title = if title.trim().is_empty() {
        if hint.is_empty() {
            "Local reference".into()
        } else {
            hint.clone()
        }
    } else {
        title.trim().to_string()
    };
    let png_base64 = if embed {
        pngutil::b64_encode(bytes)
    } else {
        String::new()
    };
    Ok(AssetReference {
        id: nid("aref"),
        asset_id: asset_id.into(),
        title,
        notes: String::new(),
        source_kind: ReferenceSourceKind::LocalFile,
        source_url: String::new(),
        source_hint: hint,
        confidence: ReferenceConfidence::UserSupplied,
        png_base64,
        width: w,
        height: h,
        embedded: embed,
        added_at: now_stamp(),
        active: true,
    })
}

pub fn reference_from_image_url(
    asset_id: &str,
    title: &str,
    url: &str,
    bytes: &[u8],
) -> Result<AssetReference, String> {
    let (w, h, _rgba) = validate_reference_bytes(bytes)?;
    let title = if title.trim().is_empty() {
        "URL reference".into()
    } else {
        title.trim().to_string()
    };
    Ok(AssetReference {
        id: nid("aref"),
        asset_id: asset_id.into(),
        title,
        notes: String::new(),
        source_kind: ReferenceSourceKind::ImageUrl,
        source_url: url.into(),
        source_hint: String::new(),
        confidence: ReferenceConfidence::Observed,
        png_base64: pngutil::b64_encode(bytes),
        width: w,
        height: h,
        embedded: true,
        added_at: now_stamp(),
        active: true,
    })
}

pub fn mark_active(list: &mut [AssetReference], asset_id: &str, reference_id: &str) {
    for item in list.iter_mut().filter(|item| item.asset_id == asset_id) {
        item.active = item.id == reference_id;
    }
}

pub fn remove_association(list: &mut Vec<AssetReference>, reference_id: &str) -> bool {
    let before = list.len();
    list.retain(|item| item.id != reference_id);
    before != list.len()
}

pub fn active_for<'a>(list: &'a [AssetReference], asset_id: &str) -> Option<&'a AssetReference> {
    list.iter()
        .find(|item| item.asset_id == asset_id && item.active)
        .or_else(|| list.iter().find(|item| item.asset_id == asset_id))
}

pub fn refs_for<'a>(list: &'a [AssetReference], asset_id: &str) -> Vec<&'a AssetReference> {
    list.iter()
        .filter(|item| item.asset_id == asset_id)
        .collect()
}

/// Decode embedded PNG/JPEG bytes for display. Missing bytes return a clear error.
pub fn load_reference_rgba(item: &AssetReference) -> Result<(u32, u32, Vec<u8>), String> {
    if item.png_base64.is_empty() {
        return Err(missing_source_message(&item.source_hint));
    }
    let bytes = pngutil::b64_decode(&item.png_base64)?;
    validate_reference_bytes(&bytes)
}

/// Deterministic side-by-side observations. Does not invent NES frames.
pub fn compare_rgba(
    ref_w: u32,
    ref_h: u32,
    ref_rgba: &[u8],
    recon_w: u32,
    recon_h: u32,
    recon_rgba: &[u8],
) -> Vec<CompareObservation> {
    let mut out = Vec::new();
    out.push(CompareObservation {
        kind: ObservationKind::Observed,
        title: "Reference dimensions".into(),
        detail: format!("{ref_w}×{ref_h} pixels"),
    });
    out.push(CompareObservation {
        kind: ObservationKind::Observed,
        title: "Reconstruction dimensions".into(),
        detail: format!("{recon_w}×{recon_h} pixels"),
    });

    if ref_w == 0 || ref_h == 0 || recon_w == 0 || recon_h == 0 {
        out.push(CompareObservation {
            kind: ObservationKind::Uncertain,
            title: "Empty image".into(),
            detail: "One side has a zero dimension, so pixel comparison is not possible.".into(),
        });
        return out;
    }

    if ref_w == recon_w && ref_h == recon_h {
        out.push(CompareObservation {
            kind: ObservationKind::Observed,
            title: "Matching pixel grid".into(),
            detail: "Both images share the same width and height.".into(),
        });
        let (ref_opaque, ref_colors) = sample_stats(ref_w, ref_h, ref_rgba);
        let (recon_opaque, recon_colors) = sample_stats(recon_w, recon_h, recon_rgba);
        out.push(CompareObservation {
            kind: ObservationKind::Derived,
            title: "Opaque pixel counts".into(),
            detail: format!("Reference {ref_opaque}; reconstruction {recon_opaque}"),
        });
        out.push(CompareObservation {
            kind: ObservationKind::Derived,
            title: "Distinct opaque colors (sampled)".into(),
            detail: format!("Reference {ref_colors}; reconstruction {recon_colors}"),
        });
        if let Some(diff) = rough_mismatch_ratio(ref_w, ref_h, ref_rgba, recon_rgba) {
            out.push(CompareObservation {
                kind: ObservationKind::Derived,
                title: "Pixel mismatch ratio".into(),
                detail: format!(
                    "{:.1}% of sampled opaque positions differ after nearest mapping.",
                    diff * 100.0
                ),
            });
        }
    } else {
        out.push(CompareObservation {
            kind: ObservationKind::Uncertain,
            title: "Scale differs".into(),
            detail: "Widths or heights differ, so a direct pixel comparison may be unreliable."
                .into(),
        });
        let ref_aspect = ref_w as f32 / ref_h as f32;
        let recon_aspect = recon_w as f32 / recon_h as f32;
        out.push(CompareObservation {
            kind: ObservationKind::Derived,
            title: "Aspect ratios".into(),
            detail: format!("Reference {ref_aspect:.3}; reconstruction {recon_aspect:.3}"),
        });
    }

    let ref_alpha = alpha_share(ref_w, ref_h, ref_rgba);
    let recon_alpha = alpha_share(recon_w, recon_h, recon_rgba);
    out.push(CompareObservation {
        kind: ObservationKind::Derived,
        title: "Transparency share".into(),
        detail: format!(
            "Reference {:.1}% transparent; reconstruction {:.1}% transparent",
            ref_alpha * 100.0,
            recon_alpha * 100.0
        ),
    });

    out.push(CompareObservation {
        kind: ObservationKind::Uncertain,
        title: "Not an automatic approval".into(),
        detail: "These notes are evidence aids. They do not publish SpriteDef or AnimSet changes."
            .into(),
    });
    out
}

fn sample_stats(w: u32, h: u32, rgba: &[u8]) -> (usize, usize) {
    let mut opaque = 0usize;
    let mut colors = std::collections::BTreeSet::new();
    let need = (w as usize) * (h as usize) * 4;
    if rgba.len() < need || w == 0 || h == 0 {
        return (0, 0);
    }
    let step_x = ((w / 32).max(1)) as usize;
    let step_y = ((h / 32).max(1)) as usize;
    for y in (0..h as usize).step_by(step_y) {
        for x in (0..w as usize).step_by(step_x) {
            let i = (y * w as usize + x) * 4;
            if rgba[i + 3] > 16 {
                opaque += 1;
                colors.insert([rgba[i], rgba[i + 1], rgba[i + 2]]);
            }
        }
    }
    (opaque, colors.len())
}

fn alpha_share(w: u32, h: u32, rgba: &[u8]) -> f32 {
    let total = (w as usize) * (h as usize);
    if total == 0 || rgba.len() < total * 4 {
        return 0.0;
    }
    let mut clear = 0usize;
    for px in 0..total {
        if rgba[px * 4 + 3] <= 16 {
            clear += 1;
        }
    }
    clear as f32 / total as f32
}

fn rough_mismatch_ratio(w: u32, h: u32, a: &[u8], b: &[u8]) -> Option<f32> {
    let total = (w as usize) * (h as usize);
    if total == 0 || a.len() < total * 4 || b.len() < total * 4 {
        return None;
    }
    let step = ((total / 2048).max(1)) as usize;
    let mut compared = 0usize;
    let mut diff = 0usize;
    for px in (0..total).step_by(step) {
        let i = px * 4;
        let aa = a[i + 3] > 16;
        let bb = b[i + 3] > 16;
        if !aa && !bb {
            continue;
        }
        compared += 1;
        if aa != bb
            || (a[i].abs_diff(b[i]) > 24
                || a[i + 1].abs_diff(b[i + 1]) > 24
                || a[i + 2].abs_diff(b[i + 2]) > 24)
        {
            diff += 1;
        }
    }
    if compared == 0 {
        None
    } else {
        Some(diff as f32 / compared as f32)
    }
}

pub fn zoom_clamped(zoom: f32) -> f32 {
    zoom.clamp(1.0, 24.0)
}

/// Visible compare size in screen pixels: `native × zoom`.
///
/// Zoom is magnification (1 = one screen pixel per image pixel). Default UI
/// zoom is 8 for NES-scale inspection. Unlike panel fit helpers, this does
/// **not** re-clamp the result into a max box — callers scroll when the size
/// exceeds the pane so raising zoom always changes the rendered size.
pub fn compare_display_size(native_w: u32, native_h: u32, zoom: f32) -> (f32, f32) {
    let z = zoom_clamped(zoom);
    ((native_w as f32) * z, (native_h as f32) * z)
}

/// True when the zoomed image should sit in a scroll region instead of being
/// silently shrunk to the pane.
pub fn compare_needs_scroll(display_w: f32, display_h: f32, pane_w: f32, pane_h: f32) -> bool {
    display_w > pane_w.max(1.0) + 0.5 || display_h > pane_h.max(1.0) + 0.5
}

pub fn layout_side_by_side(available_width: f32) -> bool {
    available_width >= 720.0
}

/// Move every association from one asset id onto another (for example ephemeral
/// Discover candidate id → durable project sprite id after Approve & Save).
/// Does not duplicate rows. Returns how many rows were rewired.
pub fn rewire_asset_id(list: &mut [AssetReference], from_id: &str, to_id: &str) -> usize {
    if from_id.is_empty() || to_id.is_empty() || from_id == to_id {
        return 0;
    }
    let mut n = 0;
    for item in list.iter_mut() {
        if item.asset_id == from_id {
            item.asset_id = to_id.to_string();
            n += 1;
        }
    }
    n
}

/// After reopen, ephemeral candidate ids no longer exist. Rewire orphaned
/// associations onto `fallback_id` when that id is known and the old id is not.
pub fn recover_orphan_asset_ids(
    list: &mut [AssetReference],
    known_ids: &[String],
    fallback_id: Option<&str>,
) -> usize {
    let Some(fallback) = fallback_id.filter(|id| !id.is_empty()) else {
        return 0;
    };
    if !known_ids.iter().any(|id| id == fallback) {
        return 0;
    }
    let mut n = 0;
    for item in list.iter_mut() {
        if item.asset_id == fallback {
            continue;
        }
        if known_ids.iter().any(|id| id == &item.asset_id) {
            continue;
        }
        item.asset_id = fallback.to_string();
        n += 1;
    }
    n
}

/// Prefer a durable sprite id when associating from a Discover candidate that
/// already has a published reconstruction in the project.
pub fn stable_association_asset_id(
    selected_id: &str,
    published_sprite_id: Option<&str>,
) -> String {
    published_sprite_id
        .filter(|id| !id.is_empty())
        .unwrap_or(selected_id)
        .to_string()
}

/// Bridge an ephemeral RefImage into attribution text without claiming ROM origin.
pub fn session_item_attribution(item: &RefImage) -> String {
    crate::reference::attribution(item)
}

pub fn classify_local_path(title: &str, path: &str) -> RefImage {
    local_reference(title, path)
}

pub fn ref_class_label(class: RefClass) -> &'static str {
    class.label()
}

fn now_stamp() -> String {
    // Deterministic-friendly wall clock string. Empty is allowed for tests that overwrite.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    fn tiny_png() -> Vec<u8> {
        // 2×2 opaque PNG
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        pngutil::rgba_to_png(&rgba, 2, 2).expect("png")
    }

    #[test]
    fn local_import_associates_and_persists() {
        let png = tiny_png();
        let mut project = Project::blank("R4");
        let item = reference_from_local("sprite-goomba", "Goomba ref", "goomba-ref.png", &png, true)
            .expect("import");
        assert_eq!(item.asset_id, "sprite-goomba");
        assert!(item.embedded);
        assert!(!item.png_base64.is_empty());
        assert_eq!(item.confidence, ReferenceConfidence::UserSupplied);
        project.asset_references.push(item.clone());
        let text = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.asset_references.len(), 1);
        assert_eq!(loaded.asset_references[0].id, item.id);
        assert_eq!(loaded.asset_references[0].asset_id, "sprite-goomba");
        let (w, h, _) = load_reference_rgba(&loaded.asset_references[0]).unwrap();
        assert_eq!((w, h), (2, 2));
    }

    #[test]
    fn unsupported_and_corrupt_files_are_rejected() {
        assert!(!supported_image_extension("notes.txt"));
        let err = reference_from_local("a", "x", "notes.txt", b"not-an-image", true).unwrap_err();
        assert!(err.contains("Unsupported"));
        let bad = reference_from_local("a", "x", "broken.png", b"not-an-image", true).unwrap_err();
        assert!(!bad.is_empty());
        let huge = vec![0u8; IMAGE_BYTE_LIMIT + 1];
        let over = validate_reference_bytes(&huge).unwrap_err();
        assert!(over.contains("byte limit"));
    }

    #[test]
    fn active_selection_and_removal_keep_other_assets() {
        let png = tiny_png();
        let a = reference_from_local("asset-a", "A", "a.png", &png, true).unwrap();
        let mut b = reference_from_local("asset-b", "B", "b.png", &png, true).unwrap();
        let c = reference_from_local("asset-a", "A2", "a2.png", &png, true).unwrap();
        let mut list = vec![a.clone(), b.clone(), c.clone()];
        mark_active(&mut list, "asset-a", &c.id);
        assert!(list.iter().find(|i| i.id == c.id).unwrap().active);
        assert!(!list.iter().find(|i| i.id == a.id).unwrap().active);
        assert!(remove_association(&mut list, &c.id));
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|i| i.id == b.id));
        b.active = false;
        list[1].active = false;
        assert_eq!(active_for(&list, "asset-b").unwrap().id, b.id);
    }

    #[test]
    fn missing_embedded_bytes_report_recovery() {
        let mut item = reference_from_local("x", "hint", "gone.png", &tiny_png(), true).unwrap();
        item.png_base64.clear();
        item.embedded = false;
        let err = load_reference_rgba(&item).unwrap_err();
        assert!(err.contains("gone.png"));
    }

    #[test]
    fn compare_notes_mark_scale_mismatch_uncertain() {
        let a = [255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let b = [255, 0, 0, 255];
        let notes = compare_rgba(2, 2, &a, 1, 1, &b);
        assert!(notes
            .iter()
            .any(|n| n.kind == ObservationKind::Uncertain && n.title.contains("Scale")));
        assert!(notes.iter().any(|n| n.title.contains("Not an automatic")));
    }

    #[test]
    fn compare_same_size_reports_observed_grid() {
        let px = [10, 20, 30, 255, 0, 0, 0, 0, 0, 0, 0, 0, 40, 50, 60, 255];
        let notes = compare_rgba(2, 2, &px, 2, 2, &px);
        assert!(notes
            .iter()
            .any(|n| n.kind == ObservationKind::Observed && n.title.contains("Matching")));
    }

    #[test]
    fn ai_gate_blocks_automatic_and_credentialless_sends() {
        let base = AiSendRequest {
            has_credential: false,
            user_confirmed: true,
            explicit_send: true,
            image_bytes: 12,
            notes: String::new(),
            model: "gpt-4.1-mini".into(),
        };
        assert_eq!(
            gate_ai_send(&base).unwrap_err(),
            AiSendBlock::MissingCredential
        );
        let mut ok = base.clone();
        ok.has_credential = true;
        ok.user_confirmed = false;
        assert_eq!(gate_ai_send(&ok).unwrap_err(), AiSendBlock::NotConfirmed);
        ok.user_confirmed = true;
        ok.explicit_send = false;
        assert_eq!(gate_ai_send(&ok).unwrap_err(), AiSendBlock::NoExplicitSend);
        ok.explicit_send = true;
        ok.image_bytes = 0;
        assert_eq!(gate_ai_send(&ok).unwrap_err(), AiSendBlock::EmptyPayload);
        ok.image_bytes = 8;
        assert!(gate_ai_send(&ok).is_ok());
        let summary = describe_ai_payload(&ok);
        assert!(summary.contains("image bytes"));
        assert!(summary.contains("ROM bytes"));
        assert!(!summary.contains("sk-"));
    }

    #[test]
    fn old_projects_without_references_still_open() {
        let bare = Project::blank("Old");
        let mut value = serde_json::to_value(&bare).unwrap();
        value.as_object_mut().unwrap().remove("asset_references");
        value.as_object_mut().unwrap().remove("ai_assist_records");
        let loaded: Project = serde_json::from_value(value).unwrap();
        assert!(loaded.asset_references.is_empty());
        assert!(loaded.ai_assist_records.is_empty());
    }

    #[test]
    fn zoom_and_layout_bounds() {
        assert_eq!(zoom_clamped(0.2), 1.0);
        assert_eq!(zoom_clamped(99.0), 24.0);
        assert!(layout_side_by_side(800.0));
        assert!(!layout_side_by_side(600.0));
    }

    #[test]
    fn reference_zoom_changes_display_size_without_panel_fit() {
        // A 64×64 reference used to appear stuck once preview_fit filled the pane.
        let (w8, h8) = compare_display_size(64, 64, 8.0);
        let (w16, h16) = compare_display_size(64, 64, 16.0);
        let (w24, h24) = compare_display_size(64, 64, 24.0);
        assert_eq!((w8, h8), (512.0, 512.0));
        assert_eq!((w16, h16), (1024.0, 1024.0));
        assert_eq!((w24, h24), (1536.0, 1536.0));
        assert!(w16 > w8 && w24 > w16);
        assert!(compare_needs_scroll(w8, h8, 360.0, 280.0));
        // Tiny reconstruction still scales with zoom and stays independent.
        let (r8, _) = compare_display_size(16, 16, 8.0);
        let (r16, _) = compare_display_size(16, 16, 16.0);
        assert_eq!(r8, 128.0);
        assert_eq!(r16, 256.0);
        assert_ne!(r16, w16);
    }

    #[test]
    fn rewire_and_orphan_recovery_preserve_bytes() {
        let png = tiny_png();
        let mut a = reference_from_local("candidate-ephemeral", "A", "a.png", &png, true).unwrap();
        let b = reference_from_local("sprite-goomba", "B", "b.png", &png, true).unwrap();
        a.active = true;
        let mut list = vec![a.clone(), b.clone()];
        assert_eq!(rewire_asset_id(&mut list, "candidate-ephemeral", "sprite-goomba"), 1);
        assert_eq!(list[0].asset_id, "sprite-goomba");
        assert!(!list[0].png_base64.is_empty());
        // Second save must not invent a duplicate when ids already match.
        assert_eq!(rewire_asset_id(&mut list, "sprite-goomba", "sprite-goomba"), 0);
        assert_eq!(list.len(), 2);

        let mut orphan = reference_from_local("candidate-gone", "Orphan", "o.png", &png, true).unwrap();
        orphan.active = true;
        let mut orphans = vec![orphan, b.clone()];
        let known = vec!["sprite-goomba".into()];
        assert_eq!(
            recover_orphan_asset_ids(&mut orphans, &known, Some("sprite-goomba")),
            1
        );
        assert_eq!(orphans[0].asset_id, "sprite-goomba");
        assert_eq!(stable_association_asset_id("candidate-x", Some("sprite-goomba")), "sprite-goomba");
        assert_eq!(stable_association_asset_id("candidate-x", None), "candidate-x");
    }

    #[test]
    fn save_reopen_keeps_embedded_reference_and_active_flag() {
        let png = tiny_png();
        let mut project = Project::blank("R4.1");
        let mut item =
            reference_from_local("sprite-goomba", "Ref", "ref.png", &png, true).unwrap();
        item.notes = "owner note".into();
        item.active = true;
        project.asset_references.push(item.clone());
        let text = serde_json::to_string_pretty(&project).unwrap();
        let loaded: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.asset_references.len(), 1);
        let restored = active_for(&loaded.asset_references, "sprite-goomba").unwrap();
        assert_eq!(restored.notes, "owner note");
        assert_eq!(restored.png_base64, item.png_base64);
        let (w, h, rgba) = load_reference_rgba(restored).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(rgba.len(), 16);
        // Re-serializing must not duplicate the association.
        let again: Project = serde_json::from_str(&serde_json::to_string(&loaded).unwrap()).unwrap();
        assert_eq!(again.asset_references.len(), 1);
    }

    #[test]
    fn url_errors_preserve_403_honesty() {
        let text = url_fetch_error("Image retrieval", 403);
        assert!(text.contains("403"));
        assert!(text.contains("blocked automated access"));
    }
}
