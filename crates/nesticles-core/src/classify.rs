//! Guess whether an uncompressed tile looks empty, like a sprite, scenery, text, or noise.
//! This is a hint. A cartridge does not label the player.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Suggestion {
    Empty,
    Sprite,
    Scenery,
    Text,
    Noise,
}

impl Suggestion {
    pub fn label(self) -> &'static str {
        match self {
            Suggestion::Empty => "empty",
            Suggestion::Sprite => "sprite-like",
            Suggestion::Scenery => "scenery-like",
            Suggestion::Text => "text-like",
            Suggestion::Noise => "noise",
        }
    }
}

pub fn suggest_kind(indices: &[u8], w: usize, h: usize) -> Suggestion {
    if w == 0 || h == 0 {
        return Suggestion::Empty;
    }
    let n = w * h;
    let mut filled = 0usize;
    let mut border_clear = 0usize;
    let mut border = 0usize;
    let mut changes = 0usize;
    let mut used = [false; 256];
    let mut used_count = 0usize;
    for y in 0..h {
        for x in 0..w {
            let v = indices.get(y * w + x).copied().unwrap_or(0);
            if v != 0 {
                filled += 1;
            }
            if !used[v as usize] {
                used[v as usize] = true;
                used_count += 1;
            }
            let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            if edge {
                border += 1;
                if v == 0 {
                    border_clear += 1;
                }
            }
            if x > 0 && indices.get(y * w + x - 1).copied().unwrap_or(0) != v {
                changes += 1;
            }
        }
    }
    let fill = filled as f32 / n as f32;
    if fill < 0.04 {
        return Suggestion::Empty;
    }
    let clear_ratio = if border == 0 {
        0.0
    } else {
        border_clear as f32 / border as f32
    };
    let churn = changes as f32 / n as f32;
    if churn > 0.72 && used_count > 6 {
        return Suggestion::Noise;
    }
    if used_count <= 2 && fill > 0.25 && fill < 0.85 && churn > 0.18 {
        return Suggestion::Text;
    }
    if clear_ratio > 0.55 && fill < 0.72 && fill > 0.08 {
        return Suggestion::Sprite;
    }
    Suggestion::Scenery
}
