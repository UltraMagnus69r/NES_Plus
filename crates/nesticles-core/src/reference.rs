//! Untrusted reference pages. HTML is scanned as text. Scripts are not run.

use crate::pipeline::{allow_reference_url, Candidate, FrameDraft, ReviewStatus};
use crate::project::nid;

pub const PAGE_BYTE_LIMIT: usize = 1_000_000;
pub const IMAGE_BYTE_LIMIT: usize = 4_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefClass {
    /// A game sprite published on the page. Still not pixels from the user's ROM.
    Original,
    /// Artwork the page itself marks as custom.
    Custom,
    /// Logos, preloaders, and unrelated thumbnails.
    SiteChrome,
    /// A file the user picked on this machine.
    Local,
}

impl RefClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Original => "Original-game reference",
            Self::Custom => "Custom artwork",
            Self::SiteChrome => "Site chrome",
            Self::Local => "Local file",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefImage {
    pub title: String,
    pub category: String,
    pub class: RefClass,
    pub page_url: String,
    pub image_url: String,
}

pub fn bounded_body(len: usize, limit: usize) -> Result<(), String> {
    if len > limit {
        Err(format!(
            "The download is {len} bytes, above the {limit} byte limit. It was not kept."
        ))
    } else {
        Ok(())
    }
}

pub fn http_status_error(stage: &str, status: u16) -> String {
    format!("Failed during {stage}. The server returned HTTP {status}.")
}

pub fn scan_reference_html(page_url: &str, html: &str) -> Vec<RefImage> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < html.len() && found.len() < 500 {
        let Some(rel) = html[i..].find('<') else {
            break;
        };
        i += rel;
        let rest = &html[i..];
        let head = rest.get(..8).unwrap_or(rest).to_ascii_lowercase();
        if head.starts_with("<script") {
            i = skip_until(html, i, "</script>");
            continue;
        }
        if head.starts_with("<style") {
            i = skip_until(html, i, "</style>");
            continue;
        }
        if head.starts_with("<img") {
            let tag_end = rest.find('>').unwrap_or(rest.len().saturating_sub(1));
            let tag = &rest[..=tag_end.min(rest.len().saturating_sub(1))];
            let raw = attr(tag, &["src", "data-src", "data-lazy-src", "data-original"])
                .or_else(|| srcset_first(attr(tag, &["srcset"]).as_deref()));
            let title = attr(tag, &["alt"]).unwrap_or_default();
            if let Some(raw) = raw {
                push_image(&mut found, page_url, &raw, &title);
            }
            i += tag_end.saturating_add(1);
            continue;
        }
        if is_anchor(rest) {
            let tag_end = rest.find('>').unwrap_or(rest.len());
            let tag = &rest[..tag_end];
            let after = i + tag_end;
            let text_end = html[after.min(html.len())..]
                .to_ascii_lowercase()
                .find("</a>")
                .map(|n| after + n)
                .unwrap_or(html.len());
            let title = strip_tags(&html[after.min(html.len())..text_end]);
            if let Some(href) = attr(tag, &["href"]) {
                push_image(&mut found, page_url, &href, &title);
            }
            i += tag_end.saturating_add(1);
            continue;
        }
        i += 1;
    }
    found
}

pub fn attribution(item: &RefImage) -> String {
    match item.class {
        RefClass::Custom => format!(
            "{} is custom artwork from {}. It was not extracted from the cartridge.",
            item.title, item.page_url
        ),
        RefClass::Original => format!(
            "{} is a website reference ({}) from {}. These pixels are not the ROM's CHR.",
            item.title, item.image_url, item.page_url
        ),
        RefClass::SiteChrome => format!(
            "{} is site artwork from {}. It is not a cartridge sprite.",
            item.title, item.page_url
        ),
        RefClass::Local => format!(
            "{} is a local image. It was not extracted from the cartridge.",
            item.title
        ),
    }
}

/// A review candidate the user can assemble by hand. It carries no tile guess.
pub fn reference_candidate(item: &RefImage) -> Candidate {
    Candidate {
        id: nid("ref"),
        name: item.title.clone(),
        category: item.category.clone(),
        target_style: "reference".into(),
        collection: "reference".into(),
        character_id: item.category.to_ascii_lowercase(),
        form: String::new(),
        frames: vec![FrameDraft {
            state: "reference".into(),
            origin: "user authored".into(),
            spots: Vec::new(),
        }],
        palette: 0,
        status: ReviewStatus::Awaiting,
        fail_reason: String::new(),
        confidence: "human reference, not matched".into(),
        evidence: attribution(item),
        attempts: Vec::new(),
        rejected_tiles: Vec::new(),
    }
}

pub fn local_reference(title: &str, path: &str) -> RefImage {
    RefImage {
        title: if title.is_empty() {
            "Local image".into()
        } else {
            title.into()
        },
        category: category_of(title, path),
        class: RefClass::Local,
        page_url: String::new(),
        image_url: path.into(),
    }
}

fn push_image(found: &mut Vec<RefImage>, page_url: &str, raw: &str, title: &str) {
    let Ok(image_url) = resolve_reference_url(page_url, raw) else {
        return;
    };
    if !is_image_url(&image_url) {
        return;
    }
    if found.iter().any(|item| item.image_url == image_url) {
        return;
    }
    let file = image_url
        .rsplit('/')
        .next()
        .unwrap_or("image")
        .split('?')
        .next()
        .unwrap_or("image");
    let title = {
        let clean = title.trim();
        if clean.is_empty() {
            file.rsplit_once('.')
                .map(|(stem, _)| stem.to_string())
                .unwrap_or_else(|| file.to_string())
        } else {
            clean.to_string()
        }
    };
    found.push(RefImage {
        category: category_of(&title, &image_url),
        class: class_of(&image_url),
        page_url: page_url.to_string(),
        image_url,
        title,
    });
}

pub fn resolve_reference_url(page: &str, raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.starts_with('#')
        || raw.to_ascii_lowercase().starts_with("javascript:")
        || raw.to_ascii_lowercase().starts_with("data:")
    {
        return Err("That link is not an image URL.".into());
    }
    let absolute = if raw.starts_with("https://") {
        raw.to_string()
    } else if let Some(rest) = raw.strip_prefix("//") {
        format!("https://{rest}")
    } else if raw.to_ascii_lowercase().starts_with("http://") {
        return Err("Reference images must use HTTPS.".into());
    } else if raw.starts_with('/') {
        format!("https://{}{raw}", host_of(page)?)
    } else {
        format!("{}/{raw}", directory_of(page)?)
    };
    allow_reference_url(&absolute)?;
    Ok(absolute)
}

fn directory_of(page: &str) -> Result<String, String> {
    allow_reference_url(page)?;
    let page = page.split(['?', '#']).next().unwrap_or(page);
    if page.ends_with('/') {
        Ok(page.trim_end_matches('/').to_string())
    } else {
        Ok(page
            .rsplit_once('/')
            .map(|(dir, _)| dir.to_string())
            .unwrap_or_else(|| page.to_string()))
    }
}

fn host_of(page: &str) -> Result<String, String> {
    allow_reference_url(page)?;
    let rest = page.trim_start_matches("https://");
    let host = rest.split('/').next().unwrap_or("").trim();
    if host.is_empty() {
        Err("That URL has no host.".into())
    } else {
        Ok(host.to_string())
    }
}

fn class_of(url: &str) -> RefClass {
    let lower = url.to_ascii_lowercase();
    if lower.contains("custom") {
        RefClass::Custom
    } else if lower.contains("/wp-content/uploads/")
        || lower.contains("preloader")
        || lower.contains("/logo.")
    {
        RefClass::SiteChrome
    } else {
        RefClass::Original
    }
}

fn category_of(title: &str, url: &str) -> String {
    let file = url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let blob = format!("{} {file}", title.to_ascii_lowercase());
    if file.starts_with("bg-") || blob.contains("background") {
        "Backgrounds".into()
    } else if blob.contains("enem") {
        "Enemies".into()
    } else if blob.contains("item") {
        "Items".into()
    } else if blob.contains("block") {
        "Blocks".into()
    } else if blob.contains("tile") {
        "Tiles".into()
    } else if blob.contains("mario") && blob.contains("luigi") {
        "Characters".into()
    } else if blob.contains("mario") {
        "Mario".into()
    } else if blob.contains("luigi") {
        "Luigi".into()
    } else {
        "Other".into()
    }
}

fn is_image_url(url: &str) -> bool {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .to_ascii_lowercase();
    [".png", ".gif", ".jpg", ".jpeg", ".webp"]
        .iter()
        .any(|ext| path.ends_with(ext))
}

fn is_anchor(rest: &str) -> bool {
    let head = rest.get(..3).unwrap_or(rest).to_ascii_lowercase();
    if !head.starts_with("<a") {
        return false;
    }
    rest.len() < 3
        || rest[2..].starts_with(|c: char| c.is_ascii_whitespace() || c == '>')
        || rest.len() == 2
}

fn attr(tag: &str, names: &[&str]) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    for name in names {
        let key = format!("{name}=");
        let Some(at) = lower.find(&key) else {
            continue;
        };
        let value = &tag[at + key.len()..];
        let value = value.trim_start();
        if let Some(rest) = value.strip_prefix('"') {
            return Some(rest.split('"').next().unwrap_or("").to_string());
        }
        if let Some(rest) = value.strip_prefix('\'') {
            return Some(rest.split('\'').next().unwrap_or("").to_string());
        }
        return Some(
            value
                .split(|c: char| c.is_ascii_whitespace() || c == '>')
                .next()
                .unwrap_or("")
                .to_string(),
        );
    }
    None
}

fn srcset_first(srcset: Option<&str>) -> Option<String> {
    let srcset = srcset?;
    let first = srcset.split(',').next()?.trim();
    let url = first.split_whitespace().next()?.trim();
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

fn strip_tags(text: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for ch in text.chars() {
        if ch == '<' {
            skipping = true;
            continue;
        }
        if ch == '>' {
            skipping = false;
            continue;
        }
        if !skipping {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn skip_until(html: &str, from: usize, close: &str) -> usize {
    let lower = html[from..].to_ascii_lowercase();
    match lower.find(close) {
        Some(at) => from + at + close.len(),
        None => html.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "https://cdn.example/sprites/nes/smb/";

    fn fixture() -> String {
        format!(
            r#"<html><script>var x = "src=\"https://evil.example/injected.png\"";</script>
<img src="https://cdn.example/logo.png" alt="Logo">
<a href="blocks.png">Blocks</a>
<a href="https://cdn.example/sprites/nes/smb/enemies.png">Enemies</a>
<a href="mario-custom.png">Mario</a>
<img data-src="/sprites/items.gif" alt="Items">
<img src="tiles.png" alt="Tiles">
<img srcset="bg-1-1.png 1x, bg-big.png 2x" alt="1-1">
<a href="notes.html">Notes</a>
<a href="luigi.png">Luigi</a>
<p>No extra pictures</p></html>"#
        )
    }

    #[test]
    fn a_public_page_lists_sprite_links_and_skips_scripts() {
        let found = scan_reference_html(PAGE, &fixture());
        let urls: Vec<_> = found.iter().map(|item| item.image_url.as_str()).collect();
        assert!(urls.iter().all(|url| !url.contains("injected")));
        assert!(urls.contains(&"https://cdn.example/sprites/nes/smb/blocks.png"));
        assert!(urls.contains(&"https://cdn.example/sprites/nes/smb/enemies.png"));
        assert!(urls.contains(&"https://cdn.example/sprites/items.gif"));
        assert!(urls.iter().all(|url| !url.ends_with("notes.html")));
        assert!(found.iter().any(|item| item.category == "Mario"
            && item.class == RefClass::Custom));
        assert!(found
            .iter()
            .any(|item| item.category == "Enemies" && item.class == RefClass::Original));
        assert!(found
            .iter()
            .any(|item| item.category == "Luigi" && item.class == RefClass::Original));
        assert!(found
            .iter()
            .any(|item| item.category == "Items" && item.class == RefClass::Original));
        assert!(found
            .iter()
            .any(|item| item.category == "Blocks" && item.class == RefClass::Original));
        assert!(found
            .iter()
            .any(|item| item.category == "Tiles" && item.class == RefClass::Original));
        assert!(found
            .iter()
            .any(|item| item.category == "Backgrounds" && item.class == RefClass::Original));
        assert!(found
            .iter()
            .any(|item| item.class == RefClass::SiteChrome));
    }

    #[test]
    fn malformed_and_empty_pages_do_not_invent_images() {
        let broken = scan_reference_html(PAGE, "<img src=\"https://cdn.example/a.png\"");
        assert!(broken.is_empty() || broken.iter().all(|item| item.image_url.starts_with("https://")));
        assert!(scan_reference_html(PAGE, "<p>No pictures here</p>").is_empty());
        assert!(scan_reference_html(PAGE, "<a href=\"relative-page\">Mario</a>").is_empty());
    }

    #[test]
    fn relative_links_stay_on_https() {
        let url = resolve_reference_url(PAGE, "mario.png").unwrap();
        assert_eq!(url, "https://cdn.example/sprites/nes/smb/mario.png");
        let rooted = resolve_reference_url(PAGE, "/sprites/luigi.png").unwrap();
        assert_eq!(rooted, "https://cdn.example/sprites/luigi.png");
        assert!(resolve_reference_url(PAGE, "http://cdn.example/mario.png").is_err());
        assert!(resolve_reference_url(PAGE, "https://127.0.0.1/mario.png").is_err());
    }

    #[test]
    fn oversized_downloads_are_refused() {
        assert!(bounded_body(IMAGE_BYTE_LIMIT, IMAGE_BYTE_LIMIT).is_ok());
        assert!(bounded_body(IMAGE_BYTE_LIMIT + 1, IMAGE_BYTE_LIMIT).is_err());
    }

    #[test]
    fn http_failures_name_the_status() {
        let text = http_status_error("Page retrieval", 404);
        assert!(text.contains("404"));
        assert!(text.contains("Page retrieval"));
    }

    #[test]
    fn a_reference_handoff_does_not_invent_rom_tiles() {
        let item = RefImage {
            title: "Mario".into(),
            category: "Mario".into(),
            class: RefClass::Original,
            page_url: PAGE.into(),
            image_url: "https://cdn.example/sprites/nes/smb/mario.png".into(),
        };
        let candidate = reference_candidate(&item);
        assert!(candidate.tile_ids().is_empty());
        assert_eq!(candidate.confidence, "human reference, not matched");
        assert!(candidate.evidence.contains("not the ROM"));
        let local = local_reference("Goomba", "/tmp/goomba.png");
        assert_eq!(local.class, RefClass::Local);
        assert!(attribution(&local).contains("not extracted"));
    }

    #[test]
    fn custom_art_is_not_labeled_as_a_cartridge_extract() {
        let item = RefImage {
            title: "Yoshi".into(),
            category: "Other".into(),
            class: RefClass::Custom,
            page_url: PAGE.into(),
            image_url: "https://cdn.example/yoshi-custom.png".into(),
        };
        let text = attribution(&item);
        assert!(text.contains("custom artwork"));
        assert!(text.contains("not extracted"));
    }

    #[cfg(feature = "png")]
    #[test]
    fn a_broken_image_and_an_oversized_buffer_are_rejected() {
        assert!(crate::pngutil::reference_preview(b"not-an-image", 64).is_err());
        let huge = vec![0u8; IMAGE_BYTE_LIMIT + 1];
        let err = crate::pngutil::reference_preview(&huge, 64).unwrap_err();
        assert!(err.contains("4 MB"));
    }
}
