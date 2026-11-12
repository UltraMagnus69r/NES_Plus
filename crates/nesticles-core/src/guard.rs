//! Whole-word and phrase blocklist for the image maker.
//! The workshop assistant never calls the image model.

const TERMS: &[&str] = &[
    "mario",
    "luigi",
    "princess peach",
    "bowser",
    "yoshi",
    "koopa",
    "goomba",
    "toad",
    "zelda",
    "ganondorf",
    "ganon",
    "kirby",
    "samus",
    "pikachu",
    "pokemon",
    "pokémon",
    "metroid",
    "donkey kong",
    "super mario",
    "nintendo",
    "link",
];

/// Returns the blocked term when the prompt names a commercial character.
/// Matching is case-insensitive and requires a whole word or phrase, so "blink" is allowed.
pub fn image_prompt_blocked(prompt: &str) -> Option<&'static str> {
    let text = normalize(prompt);
    let chars: Vec<char> = text.chars().collect();
    for term in TERMS {
        if contains_term(&chars, term) {
            return Some(*term);
        }
    }
    None
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}'))
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

fn contains_term(text: &[char], term: &str) -> bool {
    let term: Vec<char> = term.chars().collect();
    if term.is_empty() || text.len() < term.len() {
        return false;
    }
    let mut i = 0;
    while i + term.len() <= text.len() {
        if text[i..i + term.len()] == term[..] {
            let before_ok = i == 0 || !is_word(text[i - 1]);
            let after = i + term.len();
            let after_ok = after == text.len() || !is_word(text[after]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Remove secret strings from an error before it is shown or stored.
pub fn redact_secrets(text: &str, secrets: &[&str]) -> String {
    let mut out = text.to_string();
    for secret in secrets {
        if secret.len() >= 4 {
            out = out.replace(secret, "[redacted]");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_mario_and_allows_seed_spirit() {
        assert!(image_prompt_blocked("please draw mario jumping").is_some());
        assert!(image_prompt_blocked("Mario").is_some());
        assert!(image_prompt_blocked("an original seed spirit with a paper lantern").is_none());
        assert!(image_prompt_blocked("a blink of lantern light").is_none());
        assert!(image_prompt_blocked("draw link with a sword").is_some());
        assert!(image_prompt_blocked("marine seed").is_none());
    }
}
