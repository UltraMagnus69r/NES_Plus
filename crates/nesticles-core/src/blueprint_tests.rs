use super::*;
use crate::demo::demo_project;
use crate::enemy_ai::{EnemyAiStyle, EnemyArchetype};
use crate::engine::res_1080p::rules::smb1::collision::BlockKind;
use crate::engine::res_1080p::rules::smb1::enemies::Kind;
use crate::engine::res_1080p::rules::smb1::hazards::HazardKind;
use crate::engine::res_1080p::{CharacterId, World};
use crate::physics::{CharacterPhysics, Input, Sim};
use crate::project::{
    Asset, AssetKind, BlockTag, Cell, HazardTag, ItemTag, Layer, Level, PowerUpStyle, Project,
    Spawn, WarpTarget,
};

fn asset(id: &str, name: &str, kind: AssetKind) -> Asset {
    Asset {
        id: id.into(),
        name: name.into(),
        kind,
        style_id: String::new(),
        w: 16,
        h: 16,
        rgba_base64: format!("rgba-{id}-frame"),
        png_base64: String::new(),
        wav_base64: String::new(),
        solid: kind == AssetKind::Terrain,
        cue: String::new(),
        item_tag: ItemTag::Auto,
        block_tag: BlockTag::Auto,
        hazard_tag: HazardTag::Auto,
        warp: None,
    }
}

fn cell(id: &str, x: i32, y: i32) -> Cell {
    Cell {
        x,
        y,
        asset_id: id.into(),
        ..Cell::default()
    }
}

fn course(cells: Vec<Cell>) -> Level {
    Level {
        id: "course".into(),
        name: "Blueprint".into(),
        style_id: String::new(),
        w: 24,
        h: 15,
        sky: "#24343c".into(),
        music_id: None,
        player: Spawn { x: 2.0, y: 12.0 },
        cells,
        enemy_ai: EnemyAiStyle::Smb1Nes,
    }
}

fn project_with(assets: Vec<Asset>, cells: Vec<Cell>) -> Project {
    let mut project = Project::blank("blueprints");
    project.assets = assets;
    project.levels = vec![course(cells)];
    project
}

fn strip_blueprint_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("format");
            map.remove("blueprints");
            map.remove("blueprint_id");
            for child in map.values_mut() {
                strip_blueprint_fields(child);
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                strip_blueprint_fields(child);
            }
        }
        _ => {}
    }
}

#[test]
fn legacy_project_json_loads_without_blueprint_fields() {
    let original = demo_project();
    let mut value = serde_json::to_value(&original).unwrap();
    strip_blueprint_fields(&mut value);
    assert!(value.get("format").is_none());
    assert!(value.get("blueprints").is_none());
    let loaded: Project = serde_json::from_value(value).unwrap();
    assert_eq!(loaded.format, 1);
    assert!(loaded.blueprints.is_empty());
    assert!(loaded
        .levels
        .iter()
        .flat_map(|level| &level.cells)
        .all(|cell| cell.blueprint_id.is_none()));
    assert_eq!(loaded.name, original.name);
    assert_eq!(loaded.styles.len(), original.styles.len());
    for (loaded_style, original_style) in loaded.styles.iter().zip(&original.styles) {
        assert_eq!(loaded_style.id, original_style.id);
        assert_eq!(loaded_style.name, original_style.name);
        assert_eq!(loaded_style.source, original_style.source);
    }
    assert_eq!(loaded.assets.len(), original.assets.len());
    for (loaded_asset, original_asset) in loaded.assets.iter().zip(&original.assets) {
        assert_eq!(loaded_asset.id, original_asset.id);
        assert_eq!(loaded_asset.name, original_asset.name);
        assert_eq!(loaded_asset.kind, original_asset.kind);
        assert_eq!(loaded_asset.rgba_base64, original_asset.rgba_base64);
        assert_eq!(loaded_asset.item_tag, original_asset.item_tag);
        assert_eq!(loaded_asset.block_tag, original_asset.block_tag);
        assert_eq!(loaded_asset.hazard_tag, original_asset.hazard_tag);
        assert_eq!(loaded_asset.solid, original_asset.solid);
    }
    assert_eq!(loaded.levels.len(), original.levels.len());
    for (loaded_level, original_level) in loaded.levels.iter().zip(&original.levels) {
        assert_eq!(loaded_level.id, original_level.id);
        assert_eq!(loaded_level.name, original_level.name);
        assert_eq!(loaded_level.w, original_level.w);
        assert_eq!(loaded_level.h, original_level.h);
        assert_eq!(loaded_level.cells.len(), original_level.cells.len());
        assert_eq!(loaded_level.player.x, original_level.player.x);
        assert_eq!(loaded_level.player.y, original_level.player.y);
        assert_eq!(loaded_level.enemy_ai, original_level.enemy_ai);
        assert_eq!(loaded_level.music_id, original_level.music_id);
    }
    assert_eq!(loaded.nodes.len(), original.nodes.len());
    assert_eq!(loaded.start_node_id, original.start_node_id);
    assert_eq!(loaded.character_physics, original.character_physics);
    assert_eq!(loaded.power_up_style, original.power_up_style);
    assert_eq!(loaded.enemy_ai, original.enemy_ai);
    assert_eq!(loaded.character_physics, CharacterPhysics::Smb1Mario);
    assert_eq!(loaded.power_up_style, PowerUpStyle::Smb1);
}

#[test]
fn migration_assigns_refs_matching_current_inference() {
    let mut loose = asset("loose", "Loose Sod", AssetKind::Terrain);
    loose.solid = false;
    let mut tagged = asset("tagged", "Question Block", AssetKind::Terrain);
    tagged.block_tag = BlockTag::Brick;
    let mut flower = asset("flower", "Mushroom", AssetKind::Item);
    flower.item_tag = ItemTag::Flower;
    let mut long_bar = asset("long-bar", "Fire Bar", AssetKind::Hazard);
    long_bar.hazard_tag = HazardTag::FireBarLong;
    let mut pipe = asset("pipe", "Pipe", AssetKind::Hazard);
    pipe.warp = Some(WarpTarget {
        level_id: "lantern".into(),
        x: 3,
        y: 4,
    });
    let assets = vec![
        asset("brick", "Brick", AssetKind::Terrain),
        asset("question", "Question Block", AssetKind::Terrain),
        asset("coin", "Coin Brick", AssetKind::Terrain),
        tagged,
        asset("pebble", "Pebble", AssetKind::Enemy),
        asset("bowser", "Goomba", AssetKind::Enemy),
        asset("thorn", "Thorn", AssetKind::Hazard),
        pipe,
        asset("pipe-right", "Pipe Right", AssetKind::Hazard),
        asset("shroom", "Mushroom", AssetKind::Item),
        flower,
        asset("seed", "Seed", AssetKind::Item),
        asset("nest", "Nest", AssetKind::Goal),
        asset("hill", "Hill", AssetKind::Background),
        loose,
        asset("bar", "Fire Bar", AssetKind::Terrain),
        long_bar,
        asset("lava", "Podoboo", AssetKind::Hazard),
        asset("vine", "Vine", AssetKind::Foreground),
        asset("jump", "Hop", AssetKind::Sfx),
    ];
    let mut bowser = cell("bowser", 6, 1);
    bowser.archetype = EnemyArchetype::Bowser;
    let mut back = cell("loose", 1, 3);
    back.layer = Layer::Back;
    let mut main_loose = cell("loose", 2, 3);
    main_loose.layer = Layer::Main;
    let mut pipe_cell = cell("pipe", 8, 4);
    pipe_cell.warp = Some(WarpTarget {
        level_id: "other".into(),
        x: 9,
        y: 1,
    });
    let cells = vec![
        cell("brick", 0, 0),
        cell("question", 1, 0),
        cell("coin", 2, 0),
        cell("tagged", 3, 0),
        cell("pebble", 4, 0),
        bowser,
        cell("thorn", 5, 1),
        pipe_cell,
        cell("pipe-right", 9, 4),
        cell("shroom", 0, 2),
        cell("flower", 1, 2),
        cell("seed", 2, 2),
        cell("nest", 3, 2),
        cell("hill", 4, 2),
        back,
        main_loose,
        cell("bar", 6, 2),
        cell("long-bar", 7, 2),
        cell("lava", 8, 2),
        cell("vine", 9, 2),
        cell("jump", 0, 4),
        cell("missing", 1, 4),
    ];
    let mut project = project_with(assets, cells);
    ensure_legacy_blueprints(&mut project);

    let missing = project.levels[0]
        .cells
        .iter()
        .find(|cell| cell.asset_id == "missing")
        .unwrap();
    assert!(missing.blueprint_id.is_none());

    for cell in &project.levels[0].cells {
        if cell.asset_id == "missing" {
            continue;
        }
        let id = cell.blueprint_id.as_deref().expect("placement got a ref");
        let blueprint = project
            .blueprints
            .iter()
            .find(|blueprint| blueprint.id == id)
            .unwrap();
        let asset = project.asset(&cell.asset_id).unwrap();
        assert_eq!(blueprint.visual_id, asset.id);
        assert!(project.asset(&blueprint.visual_id).is_some());
        assert_eq!(blueprint.origin, BlueprintOrigin::LegacyInfer);
        assert_eq!(
            blueprint.organizational_label,
            OrganizationalLabel::from_kind(asset.kind)
        );
        assert_eq!(blueprint.legacy_role, infer_legacy(asset, cell));
        assert!(!blueprint_json(blueprint).contains(&asset.rgba_base64));
    }

    let role = |id: &str| {
        let cell = project.levels[0]
            .cells
            .iter()
            .find(|cell| cell.asset_id == id)
            .unwrap();
        let blueprint_id = cell.blueprint_id.clone().unwrap();
        project
            .blueprints
            .iter()
            .find(|blueprint| blueprint.id == blueprint_id)
            .unwrap()
            .legacy_role
            .clone()
    };
    assert_eq!(role("brick").block(), Some(LegacyBlock::Brick));
    assert_eq!(role("question").block(), Some(LegacyBlock::Question));
    assert_eq!(role("coin").block(), Some(LegacyBlock::Coin));
    assert_eq!(
        role("tagged").block(),
        Some(LegacyBlock::Brick),
        "explicit brick tag wins over the question name"
    );
    assert_eq!(role("pebble").enemy_kind(), Some(LegacyEnemy::Goomba));
    assert_eq!(role("bowser").enemy_kind(), Some(LegacyEnemy::Bowser));
    assert!(matches!(
        role("thorn"),
        LegacyRole::Fixture {
            block: None,
            item: None,
            hazard: Some(LegacyHazard::Contact),
            warp: None,
        }
    ));
    assert!(matches!(
        role("pipe"),
        LegacyRole::Fixture {
            block: Some(LegacyBlock::Solid),
            hazard: Some(LegacyHazard::PipeDown),
            warp: Some(LegacyWarp { ref level_id, x: 9, y: 1 }),
            ..
        } if level_id == "other"
    ));
    assert!(matches!(
        role("pipe-right"),
        LegacyRole::Fixture {
            block: Some(LegacyBlock::Solid),
            hazard: Some(LegacyHazard::PipeRight),
            ..
        }
    ));
    assert!(matches!(
        role("shroom"),
        LegacyRole::Fixture {
            item: Some(LegacyItem::Mushroom),
            ..
        }
    ));
    assert!(matches!(
        role("flower"),
        LegacyRole::Fixture {
            item: Some(LegacyItem::Flower),
            ..
        }
    ));
    assert!(matches!(
        role("seed"),
        LegacyRole::Fixture {
            item: Some(LegacyItem::Plain),
            hazard: None,
            ..
        }
    ));
    assert!(matches!(role("nest"), LegacyRole::Goal { warp: None }));
    assert_eq!(role("hill"), LegacyRole::Inert);
    assert_eq!(role("jump"), LegacyRole::Inert);
    assert!(matches!(
        role("lava"),
        LegacyRole::Enemy {
            kind: LegacyEnemy::Podoboo,
            ..
        }
    ));
    assert!(matches!(
        role("vine"),
        LegacyRole::Fixture {
            hazard: Some(LegacyHazard::Vine),
            block: None,
            ..
        }
    ));
    assert!(matches!(
        role("bar"),
        LegacyRole::Fixture {
            block: Some(LegacyBlock::Solid),
            hazard: Some(LegacyHazard::FireBar { balls: 6 }),
            ..
        }
    ));
    assert!(matches!(
        role("long-bar"),
        LegacyRole::Fixture {
            hazard: Some(LegacyHazard::FireBar { balls: 12 }),
            block: None,
            ..
        }
    ));

    let world = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    let solid = |x: i32, y: i32| {
        world
            .solids
            .iter()
            .find(|solid| solid.cell_x == x && solid.cell_y == y)
            .map(|solid| solid.block)
    };
    assert_eq!(solid(0, 0), Some(BlockKind::Brick));
    assert_eq!(solid(1, 0), Some(BlockKind::Question));
    assert_eq!(solid(2, 0), Some(BlockKind::Coin));
    assert_eq!(solid(3, 0), Some(BlockKind::Brick));
    assert_eq!(
        solid(1, 3),
        None,
        "back layer of a non-solid terrain is empty"
    );
    assert_eq!(solid(2, 3), Some(BlockKind::Solid));
    assert_eq!(solid(8, 4), Some(BlockKind::Solid));
    assert!(world
        .enemies
        .iter()
        .any(|enemy| enemy.asset_id == "pebble" && enemy.kind == Kind::Goomba));
    assert!(world
        .enemies
        .iter()
        .any(|enemy| enemy.asset_id == "bowser" && enemy.kind == Kind::Bowser));
    assert!(world
        .enemies
        .iter()
        .any(|enemy| enemy.asset_id == "lava" && enemy.kind == Kind::Podoboo));
    assert!(world.hazards.iter().any(|hazard| {
        hazard.cell_x == 5 && hazard.cell_y == 1 && hazard.kind == HazardKind::Contact
    }));
    assert!(world.hazards.iter().any(|hazard| {
        hazard.cell_x == 8
            && hazard.cell_y == 4
            && hazard.kind == HazardKind::PipeDown
            && hazard.warp_level == "other"
    }));
    assert_eq!(
        role("loose"),
        LegacyRole::Inert,
        "the back-layer loose cell is found first and is inert"
    );
    let main = project.levels[0]
        .cells
        .iter()
        .find(|cell| cell.asset_id == "loose" && cell.layer == Layer::Main)
        .unwrap();
    let main_role = project
        .blueprints
        .iter()
        .find(|blueprint| Some(&blueprint.id) == main.blueprint_id.as_ref())
        .unwrap()
        .legacy_role
        .clone();
    assert_eq!(main_role.block(), Some(LegacyBlock::Solid));
    assert_ne!(main.blueprint_id, {
        project.levels[0]
            .cells
            .iter()
            .find(|cell| cell.asset_id == "loose" && cell.layer == Layer::Back)
            .unwrap()
            .blueprint_id
            .clone()
    });
}

#[test]
fn seed_spirit_pebbles_record_goomba() {
    let mut project = demo_project();
    ensure_legacy_blueprints(&mut project);
    let mut pebble_ids = Vec::new();
    for level in &project.levels {
        for cell in &level.cells {
            if cell.asset_id != "pebble" {
                continue;
            }
            let id = cell.blueprint_id.clone().unwrap();
            let blueprint = project
                .blueprints
                .iter()
                .find(|blueprint| blueprint.id == id)
                .unwrap();
            assert_eq!(blueprint.visual_id, "pebble");
            assert_eq!(
                blueprint.legacy_role.enemy_kind(),
                Some(LegacyEnemy::Goomba)
            );
            assert_eq!(blueprint.organizational_label, OrganizationalLabel::Enemy);
            pebble_ids.push(id);
        }
    }
    assert_eq!(pebble_ids.len(), 3, "moss has two pebbles and lantern one");
    assert!(pebble_ids.iter().all(|id| id == &pebble_ids[0]));
    assert_eq!(
        project
            .assets
            .iter()
            .find(|asset| asset.id == "pebble")
            .unwrap()
            .name,
        "Pebble"
    );
}

#[test]
fn brick_role_survives_asset_rename() {
    let mut project = project_with(
        vec![asset("brick", "Brick", AssetKind::Terrain)],
        vec![cell("brick", 1, 1)],
    );
    ensure_legacy_blueprints(&mut project);
    let id = project.levels[0].cells[0].blueprint_id.clone().unwrap();
    assert_eq!(
        project.blueprints[0].legacy_role.block(),
        Some(LegacyBlock::Brick)
    );
    let before = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert_eq!(before.solids[0].block, BlockKind::Brick);

    project.assets[0].name = "Stone".into();
    let blueprint_role = project.blueprints[0].legacy_role.clone();
    assert_eq!(blueprint_role.block(), Some(LegacyBlock::Brick));
    assert_eq!(
        project.levels[0].cells[0].blueprint_id.as_deref(),
        Some(id.as_str())
    );

    // The live reader still uses the name. The stored blueprint does not follow it.
    let after = World::from_level(&project, &project.levels[0], CharacterId::Mario);
    assert_eq!(after.solids[0].block, BlockKind::Solid);
    assert_eq!(
        infer_legacy(&project.assets[0], &project.levels[0].cells[0]).block(),
        Some(LegacyBlock::Solid)
    );
    assert_eq!(
        project.blueprints[0].legacy_role.block(),
        Some(LegacyBlock::Brick)
    );
}

#[test]
fn two_authored_blueprints_share_a_visual_without_copying_pixels() {
    let art = asset("art", "Shared art", AssetKind::Terrain);
    let pixels = art.rgba_base64.clone();
    let mut project = Project::blank("authored");
    project.assets.push(art);
    project.blueprints.push(Blueprint {
        id: "pipe-red".into(),
        visual_id: "art".into(),
        origin: BlueprintOrigin::Authored,
        organizational_label: OrganizationalLabel::Terrain,
        legacy_role: LegacyRole::Fixture {
            block: Some(LegacyBlock::Brick),
            item: None,
            hazard: None,
            warp: None,
        },
        notes: "breakable".into(),
    });
    project.blueprints.push(Blueprint {
        id: "pipe-blue".into(),
        visual_id: "art".into(),
        origin: BlueprintOrigin::Authored,
        organizational_label: OrganizationalLabel::Terrain,
        legacy_role: LegacyRole::Fixture {
            block: Some(LegacyBlock::Question),
            item: None,
            hazard: None,
            warp: None,
        },
        notes: "prize".into(),
    });
    assert_ne!(project.blueprints[0].id, project.blueprints[1].id);
    assert_eq!(
        project.blueprints[0].visual_id,
        project.blueprints[1].visual_id
    );
    let saved = serde_json::to_string_pretty(&project).unwrap();
    assert!(!saved.contains(&pixels) || saved.matches(&pixels).count() == 1);
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.blueprints.len(), 2);
    assert_eq!(loaded.blueprints[0].id, "pipe-red");
    assert_eq!(loaded.blueprints[1].id, "pipe-blue");
    assert_eq!(loaded.blueprints[0].visual_id, "art");
    assert_eq!(loaded.blueprints[1].visual_id, "art");
    assert_eq!(loaded.blueprints[0].origin, BlueprintOrigin::Authored);
    assert_eq!(loaded.blueprints[1].origin, BlueprintOrigin::Authored);
    assert_ne!(
        loaded.blueprints[0].legacy_role,
        loaded.blueprints[1].legacy_role
    );
    assert_eq!(loaded.assets[0].rgba_base64, pixels);
    assert!(!blueprint_json(&loaded.blueprints[0]).contains(&pixels));
    assert!(!blueprint_json(&loaded.blueprints[1]).contains(&pixels));
}

#[test]
fn shared_blueprint_survives_save_and_reload() {
    let mut project = project_with(
        vec![asset("sod", "Sod", AssetKind::Terrain)],
        vec![cell("sod", 0, 12), cell("sod", 1, 12), cell("sod", 2, 12)],
    );
    ensure_legacy_blueprints(&mut project);
    assert_eq!(project.blueprints.len(), 1);
    let id = project.blueprints[0].id.clone();
    assert!(project.levels[0]
        .cells
        .iter()
        .all(|cell| cell.blueprint_id.as_deref() == Some(id.as_str())));
    let saved = serde_json::to_string_pretty(&project).unwrap();
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.blueprints.len(), 1);
    assert_eq!(loaded.blueprints[0].id, id);
    assert_eq!(loaded.blueprints[0].visual_id, "sod");
    assert!(loaded.levels[0]
        .cells
        .iter()
        .all(|cell| cell.blueprint_id.as_deref() == Some(id.as_str())));
}

#[test]
fn same_art_with_different_behavior_is_not_merged() {
    let mut scout = asset("scout", "Pebble", AssetKind::Enemy);
    scout.solid = false;
    let mut project = project_with(
        vec![scout],
        vec![cell("scout", 2, 11), cell("scout", 8, 11)],
    );
    project.levels[0].cells[1].archetype = EnemyArchetype::Bowser;
    ensure_legacy_blueprints(&mut project);
    assert_eq!(project.blueprints.len(), 2);
    assert_eq!(project.blueprints[0].visual_id, "scout");
    assert_eq!(project.blueprints[1].visual_id, "scout");
    assert_ne!(project.blueprints[0].id, project.blueprints[1].id);
    assert_ne!(
        project.levels[0].cells[0].blueprint_id,
        project.levels[0].cells[1].blueprint_id
    );
    let kinds: Vec<_> = project
        .blueprints
        .iter()
        .filter_map(|blueprint| blueprint.legacy_role.enemy_kind())
        .collect();
    assert!(kinds.contains(&LegacyEnemy::Goomba));
    assert!(kinds.contains(&LegacyEnemy::Bowser));

    let mut loose = asset("loose", "Loose", AssetKind::Terrain);
    loose.solid = false;
    let mut back = cell("loose", 0, 1);
    back.layer = Layer::Back;
    let mut project = project_with(vec![loose], vec![cell("loose", 0, 2), back]);
    ensure_legacy_blueprints(&mut project);
    assert_eq!(project.blueprints.len(), 2);
    assert_ne!(
        project.levels[0].cells[0].blueprint_id,
        project.levels[0].cells[1].blueprint_id
    );

    let mut pipe = asset("pipe", "Pipe", AssetKind::Hazard);
    pipe.solid = false;
    let mut other = cell("pipe", 4, 4);
    other.warp = Some(WarpTarget {
        level_id: "lantern".into(),
        x: 2,
        y: 2,
    });
    let mut project = project_with(vec![pipe], vec![cell("pipe", 3, 4), other]);
    ensure_legacy_blueprints(&mut project);
    assert_eq!(
        project.blueprints.len(),
        2,
        "different pipe exits must not share a blueprint"
    );
}

#[test]
fn migration_twice_keeps_ids_and_refs() {
    let mut first = demo_project();
    let mut second = demo_project();
    ensure_legacy_blueprints(&mut first);
    ensure_legacy_blueprints(&mut second);
    assert_eq!(first.blueprints.len(), second.blueprints.len());
    assert_eq!(blueprint_ids(&first), blueprint_ids(&second));
    assert_eq!(cell_refs(&first), cell_refs(&second));

    let count = first.blueprints.len();
    let ids = blueprint_ids(&first);
    let refs = cell_refs(&first);
    ensure_legacy_blueprints(&mut first);
    assert_eq!(first.blueprints.len(), count);
    assert_eq!(blueprint_ids(&first), ids);
    assert_eq!(cell_refs(&first), refs);
    assert!(count > 1);
}

#[test]
fn migrated_project_round_trips() {
    let mut project = demo_project();
    ensure_legacy_blueprints(&mut project);
    let saved = serde_json::to_string_pretty(&project).unwrap();
    let loaded: Project = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded.format, project.format);
    assert_eq!(loaded.blueprints, project.blueprints);
    assert_eq!(cell_refs(&loaded), cell_refs(&project));
    assert_eq!(loaded.assets.len(), project.assets.len());
    assert_eq!(loaded.levels[0].cells.len(), project.levels[0].cells.len());
    assert_eq!(loaded.character_physics, project.character_physics);
    assert_eq!(serde_json::to_string_pretty(&loaded).unwrap(), saved);
}

#[test]
fn migration_does_not_change_tick_play() {
    let mut cells = Vec::new();
    for x in 0..24 {
        cells.push(cell("ground", x, 13));
    }
    cells.push(cell("goomba", 8, 12));
    cells.push(cell("ground", 6, 12));
    let bare = project_with(
        vec![
            asset("ground", "Ground", AssetKind::Terrain),
            asset("goomba", "Goomba", AssetKind::Enemy),
        ],
        cells,
    );
    let mut migrated = bare.clone();
    ensure_legacy_blueprints(&mut migrated);
    assert!(migrated.levels[0]
        .cells
        .iter()
        .all(|cell| cell.blueprint_id.is_some()));
    let mut left = Sim::from_level(&bare, &bare.levels[0]);
    let mut right = Sim::from_level(&migrated, &migrated.levels[0]);
    let run = Input {
        right: true,
        run_held: true,
        ..Input::default()
    };
    for _ in 0..45 {
        left.tick_play(CharacterPhysics::Smb1Mario, PowerUpStyle::Smb1, &run);
        right.tick_play(CharacterPhysics::Smb1Mario, PowerUpStyle::Smb1, &run);
        assert_eq!(left.player.x.to_bits(), right.player.x.to_bits());
        assert_eq!(left.player.y.to_bits(), right.player.y.to_bits());
        assert_eq!(left.player.vx.to_bits(), right.player.vx.to_bits());
        assert_eq!(left.player.vy.to_bits(), right.player.vy.to_bits());
        assert_eq!(left.player.h.to_bits(), right.player.h.to_bits());
        assert_eq!(left.player.on_ground, right.player.on_ground);
        assert_eq!(left.won, right.won);
        assert_eq!(left.enemies.len(), right.enemies.len());
        for (before, after) in left.enemies.iter().zip(&right.enemies) {
            assert_eq!(before.x.to_bits(), after.x.to_bits());
            assert_eq!(before.y.to_bits(), after.y.to_bits());
            assert_eq!(before.dir, after.dir);
            assert_eq!(before.alive, after.alive);
        }
    }
}

#[test]
fn authored_refs_and_future_fields_stay_put() {
    let mut project = project_with(
        vec![asset("brick", "Brick", AssetKind::Terrain)],
        vec![cell("brick", 1, 1), cell("brick", 2, 1)],
    );
    project.levels[0].cells[0].blueprint_id = Some("hand-brick".into());
    project.blueprints.push(Blueprint {
        id: "hand-brick".into(),
        visual_id: "brick".into(),
        origin: BlueprintOrigin::Authored,
        organizational_label: OrganizationalLabel::Terrain,
        legacy_role: LegacyRole::Inert,
        notes: "kept".into(),
    });
    ensure_legacy_blueprints(&mut project);
    assert_eq!(
        project.levels[0].cells[0].blueprint_id.as_deref(),
        Some("hand-brick")
    );
    let authored = project
        .blueprints
        .iter()
        .find(|blueprint| blueprint.id == "hand-brick")
        .unwrap();
    assert_eq!(authored.origin, BlueprintOrigin::Authored);
    assert_eq!(authored.legacy_role, LegacyRole::Inert);
    assert_eq!(authored.notes, "kept");
    let generated = project.levels[0].cells[1].blueprint_id.clone().unwrap();
    assert_ne!(generated, "hand-brick");
    assert_eq!(
        project
            .blueprints
            .iter()
            .find(|blueprint| blueprint.id == generated)
            .unwrap()
            .legacy_role
            .block(),
        Some(LegacyBlock::Brick)
    );

    let partial: Blueprint = serde_json::from_str(
        r#"{"id":"later","visual_id":"art","future_component":{"kind":"mover"}}"#,
    )
    .unwrap();
    assert_eq!(partial.id, "later");
    assert_eq!(partial.visual_id, "art");
    assert_eq!(partial.origin, BlueprintOrigin::LegacyInfer);
    assert_eq!(
        partial.organizational_label,
        OrganizationalLabel::Unclassified
    );
    assert_eq!(partial.legacy_role, LegacyRole::Inert);
    assert_eq!(partial.notes, "");
    let with_format: Project =
        serde_json::from_str(r#"{"name":"old","unrelated_future":true}"#).unwrap();
    assert_eq!(with_format.format, 1);
    assert!(with_format.blueprints.is_empty());
}

fn blueprint_json(blueprint: &Blueprint) -> String {
    serde_json::to_string(blueprint).unwrap()
}

fn blueprint_ids(project: &Project) -> Vec<String> {
    project
        .blueprints
        .iter()
        .map(|blueprint| blueprint.id.clone())
        .collect()
}

fn cell_refs(project: &Project) -> Vec<Option<String>> {
    project
        .levels
        .iter()
        .flat_map(|level| level.cells.iter().map(|cell| cell.blueprint_id.clone()))
        .collect()
}
