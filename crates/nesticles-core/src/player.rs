//! A Player Blueprint names who is playing and how they move.
//!
//! The picture stays on the visual asset. The physics preset is chosen here.
//! An asset name does not make anyone the player. Courses with no Player
//! Blueprint keep the older rule: the first character picture, and the
//! project's physics preset. Opening such a project does not invent a blueprint.

use crate::engine::res_1080p::character_physics::Power;
use crate::engine::res_1080p::World;
use crate::physics::CharacterPhysics;
use crate::project::{nid, AssetKind, Level, Project};
use serde::{Deserialize, Serialize};

/// Authored player for one or more courses. Pixels stay on [`crate::project::Asset`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerBlueprint {
    pub id: String,
    pub name: String,
    /// [`crate::project::Asset::id`]. Not a copy of the pixels.
    pub visual_id: String,
    pub physics: CharacterPhysics,
    /// 1 faces right. -1 faces left. Any other stored value loads as right.
    #[serde(default = "default_facing")]
    pub facing: i32,
    /// `small`, `super`, or `fire`. Anything else loads as small.
    #[serde(default = "default_power")]
    pub power: String,
    #[serde(default)]
    pub notes: String,
}

fn default_facing() -> i32 {
    1
}

fn default_power() -> String {
    "small".into()
}

/// What Play and the Godot proof should use for one course.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedPlayer {
    /// Set when the course points at a blueprint that exists.
    pub blueprint_id: Option<String>,
    pub name: String,
    pub visual_id: String,
    pub physics: CharacterPhysics,
    pub facing: i32,
    pub power: Power,
    /// The course names a blueprint that is not in the project.
    /// The picture is left blank. Motion uses the project preset so the
    /// course can still run, and the fault is reported.
    pub missing_blueprint: bool,
    /// The blueprint's picture is not in the project.
    pub missing_visual: bool,
}

pub fn new_player_blueprint(
    name: &str,
    visual_id: &str,
    physics: CharacterPhysics,
) -> PlayerBlueprint {
    let name = name.trim();
    PlayerBlueprint {
        id: nid("player-"),
        name: if name.is_empty() {
            "Player".into()
        } else {
            name.into()
        },
        visual_id: visual_id.into(),
        physics,
        facing: 1,
        power: default_power(),
        notes: String::new(),
    }
}

pub fn power_from_label(text: &str) -> Power {
    match text.trim().to_ascii_lowercase().as_str() {
        "super" => Power::Super,
        "fire" => Power::Fire,
        _ => Power::Small,
    }
}

pub fn power_label(power: Power) -> &'static str {
    match power {
        Power::Small => "Small",
        Power::Super => "Super",
        Power::Fire => "Fire",
    }
}

pub fn facing_label(facing: i32) -> &'static str {
    if facing < 0 {
        "Left"
    } else {
        "Right"
    }
}

/// Resolve the player for `level`.
///
/// A missing blueprint does not borrow another picture. A course with no
/// blueprint id keeps the first character picture and the project preset.
pub fn resolve_player(project: &Project, level: &Level) -> ResolvedPlayer {
    if let Some(id) = level.player_blueprint_id.as_ref() {
        let Some(blueprint) = project
            .player_blueprints
            .iter()
            .find(|blueprint| &blueprint.id == id)
        else {
            return ResolvedPlayer {
                blueprint_id: None,
                name: String::new(),
                visual_id: String::new(),
                physics: project.character_physics,
                facing: 1,
                power: Power::Small,
                missing_blueprint: true,
                missing_visual: false,
            };
        };
        let visual_ok = project.assets.iter().any(|asset| {
            asset.id == blueprint.visual_id && !asset.kind.is_audio() && has_pixels(asset)
        });
        return ResolvedPlayer {
            blueprint_id: Some(blueprint.id.clone()),
            name: blueprint.name.clone(),
            visual_id: if visual_ok {
                blueprint.visual_id.clone()
            } else {
                String::new()
            },
            physics: blueprint.physics,
            facing: if blueprint.facing < 0 { -1 } else { 1 },
            power: power_from_label(&blueprint.power),
            missing_blueprint: false,
            missing_visual: !visual_ok,
        };
    }
    let visual_id = project
        .assets
        .iter()
        .find(|asset| asset.kind == AssetKind::Player && has_pixels(asset))
        .map(|asset| asset.id.clone())
        .unwrap_or_default();
    ResolvedPlayer {
        blueprint_id: None,
        name: String::new(),
        visual_id,
        physics: project.character_physics,
        facing: 1,
        power: Power::Small,
        missing_blueprint: false,
        missing_visual: false,
    }
}

fn has_pixels(asset: &crate::project::Asset) -> bool {
    !asset.rgba_base64.trim().is_empty() || !asset.png_base64.trim().is_empty()
}

/// Apply facing and starting power. The production hitbox is unchanged here;
/// the next play tick sizes a Super or Fire body the way a power-up does.
pub fn apply_resolved(world: &mut World, resolved: &ResolvedPlayer) {
    if resolved.missing_blueprint {
        world.blueprint_faults.push(
            "This course names a Player Blueprint that is not in the project. No other picture was substituted."
                .into(),
        );
    }
    if resolved.missing_visual {
        world.blueprint_faults.push(
            "The Player Blueprint's picture is missing. Play has no player sprite until you choose one."
                .into(),
        );
    }
    if resolved.blueprint_id.is_none() {
        return;
    }
    world.body.facing = resolved.facing;
    world.body.power = resolved.power;
}

pub fn player_source_note(resolved: &ResolvedPlayer) -> String {
    if let Some(id) = &resolved.blueprint_id {
        format!(
            "Player Blueprint {id} ({}) supplies the picture and the {} preset. The step is the supported 1080p SMB1-style subset.",
            if resolved.name.is_empty() {
                "player"
            } else {
                resolved.name.as_str()
            },
            resolved.physics.label()
        )
    } else if resolved.missing_blueprint {
        "This course names a Player Blueprint that is missing. No other picture was substituted. The project physics preset is used so the course can still run.".into()
    } else {
        "No Player Blueprint is assigned. The picture is the first character asset and the preset is the project default. This fallback is not a Player Blueprint. The step is the supported 1080p SMB1-style subset.".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::res_1080p::character_physics::{params_for, Power};
    use crate::physics::{map_character, Sim};
    use crate::project::{Asset, AssetKind, Level, Project, Spawn};

    fn picture(id: &str, kind: AssetKind) -> Asset {
        Asset {
            id: id.into(),
            name: id.into(),
            kind,
            style_id: "style".into(),
            w: 1,
            h: 1,
            rgba_base64: "AAAA".into(),
            png_base64: String::new(),
            wav_base64: String::new(),
            solid: kind.default_solid(),
            cue: String::new(),
            item_tag: Default::default(),
            block_tag: Default::default(),
            hazard_tag: Default::default(),
            warp: None,
        }
    }

    fn course(blueprint: Option<&str>) -> Level {
        Level {
            id: "course".into(),
            name: "Course".into(),
            style_id: "style".into(),
            w: 16,
            h: 12,
            sky: "#24343c".into(),
            music_id: None,
            player: Spawn { x: 1.0, y: 1.0 },
            cells: Vec::new(),
            enemy_ai: Default::default(),
            entrances: Vec::new(),
            player_blueprint_id: blueprint.map(str::to_string),
        }
    }

    #[test]
    fn a_course_without_a_blueprint_keeps_the_old_player() {
        let mut project = Project::blank("Old");
        project.character_physics = CharacterPhysics::Smb1Toad;
        project.assets.push(picture("pip", AssetKind::Player));
        project.assets.push(picture("other", AssetKind::Enemy));
        let resolved = resolve_player(&project, &course(None));
        assert!(resolved.blueprint_id.is_none());
        assert_eq!(resolved.visual_id, "pip");
        assert_eq!(resolved.physics, CharacterPhysics::Smb1Toad);
        assert!(!resolved.missing_blueprint);
        let text = serde_json::to_string(&project).unwrap();
        assert!(!text.contains("player_blueprints"));
        let loaded: Project = serde_json::from_str(&text).unwrap();
        assert!(loaded.player_blueprints.is_empty());
        assert_eq!(resolve_player(&loaded, &course(None)).visual_id, "pip");
    }

    #[test]
    fn the_blueprint_picks_the_picture_and_the_preset() {
        let mut project = Project::blank("Made");
        project.character_physics = CharacterPhysics::Smb1Mario;
        project.assets.push(picture("pip", AssetKind::Player));
        project.assets.push(picture("dino", AssetKind::Enemy));
        let mut blueprint = new_player_blueprint("Dinosaur", "dino", CharacterPhysics::Smb1Luigi);
        blueprint.id = "player-dino".into();
        blueprint.facing = -1;
        blueprint.power = "super".into();
        project.player_blueprints.push(blueprint);
        let mut level = course(Some("player-dino"));
        let resolved = resolve_player(&project, &level);
        assert_eq!(resolved.visual_id, "dino");
        assert_eq!(resolved.physics, CharacterPhysics::Smb1Luigi);
        assert_eq!(resolved.facing, -1);
        assert_eq!(resolved.power, Power::Super);
        assert_eq!(resolved.name, "Dinosaur");
        blueprint_physics_stays_when_the_picture_changes(&mut project);
        level.player_blueprint_id = Some("player-dino".into());
        let again = resolve_player(&project, &level);
        assert_eq!(again.physics, CharacterPhysics::Smb1Luigi);
        assert_eq!(again.visual_id, "pebble");
    }

    fn blueprint_physics_stays_when_the_picture_changes(project: &mut Project) {
        project.player_blueprints[0].visual_id = "pebble".into();
        project.assets.push(picture("pebble", AssetKind::Terrain));
    }

    #[test]
    fn changing_the_preset_changes_the_jump() {
        let mut project = Project::blank("Jump");
        project.assets.push(picture("hero", AssetKind::Background));
        let blueprint = new_player_blueprint("Hero", "hero", CharacterPhysics::Smb1Luigi);
        let id = blueprint.id.clone();
        project.player_blueprints.push(blueprint);
        let mut level = course(Some(&id));
        level.cells.clear();
        project.levels.push(level.clone());
        let sim = Sim::from_level(&project, &level);
        let world = sim.play_world().unwrap();
        assert_eq!(world.character, map_character(CharacterPhysics::Smb1Luigi));
        let mut jumping =
            World::from_level(&project, &level, map_character(CharacterPhysics::Smb1Luigi));
        jumping.body.on_ground = true;
        jumping.tick(&crate::engine::Pad {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        });
        let luigi = params_for(map_character(CharacterPhysics::Smb1Luigi));
        let mario = params_for(map_character(CharacterPhysics::Smb1Mario));
        assert_eq!(jumping.body.vy, luigi.jump_slow + luigi.gravity_hold);
        assert_ne!(jumping.body.vy, mario.jump_slow + mario.gravity_hold);
    }

    #[test]
    fn save_and_load_keep_the_assignment() {
        let mut project = Project::blank("Save");
        project.assets.push(picture("hero", AssetKind::Player));
        let blueprint = new_player_blueprint("Hero", "hero", CharacterPhysics::Smb1Peach);
        let id = blueprint.id.clone();
        project.player_blueprints.push(blueprint);
        let mut level = course(Some(&id));
        level.id = "course".into();
        project.levels.push(level);
        let text = serde_json::to_string(&project).unwrap();
        let loaded: Project = serde_json::from_str(&text).unwrap();
        let resolved = resolve_player(&loaded, &loaded.levels[0]);
        assert_eq!(resolved.blueprint_id.as_deref(), Some(id.as_str()));
        assert_eq!(resolved.visual_id, "hero");
        assert_eq!(resolved.physics, CharacterPhysics::Smb1Peach);
        let again: Project = serde_json::from_str(&text).unwrap();
        assert_eq!(again.player_blueprints, loaded.player_blueprints);
    }

    #[test]
    fn a_missing_blueprint_does_not_borrow_another_picture() {
        let mut project = Project::blank("Gap");
        project.assets.push(picture("pip", AssetKind::Player));
        let resolved = resolve_player(&project, &course(Some("player-gone")));
        assert!(resolved.visual_id.is_empty());
        assert!(resolved.missing_blueprint);
        assert_eq!(resolved.physics, project.character_physics);
    }

    #[test]
    fn a_legacy_file_without_the_field_still_loads() {
        let raw = r#"{"name":"Old","levels":[{"id":"c","name":"C","w":8,"h":8,"player":{"x":1.0,"y":2.0}}]}"#;
        let project: Project = serde_json::from_str(raw).unwrap();
        assert!(project.player_blueprints.is_empty());
        assert!(project.levels[0].player_blueprint_id.is_none());
        assert_eq!(project.character_physics, CharacterPhysics::Smb1Mario);
    }

    #[test]
    fn apply_resolved_sets_facing_and_reports_a_missing_picture() {
        let mut project = Project::blank("Face");
        let mut blueprint = new_player_blueprint("Hero", "gone", CharacterPhysics::Smb1Mario);
        blueprint.id = "player-hero".into();
        blueprint.facing = -1;
        project.player_blueprints.push(blueprint);
        let resolved = resolve_player(&project, &course(Some("player-hero")));
        assert!(resolved.missing_visual);
        let mut world = World::from_level(
            &project,
            &course(Some("player-hero")),
            map_character(resolved.physics),
        );
        apply_resolved(&mut world, &resolved);
        assert_eq!(world.body.facing, -1);
        assert!(world
            .blueprint_faults
            .iter()
            .any(|fault| fault.contains("picture is missing")));
    }
}
