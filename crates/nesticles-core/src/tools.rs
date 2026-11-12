//! Tool executor for the workshop assistant.
//! The assistant can edit the project. It cannot call the image model.

use crate::project::{audit_course, nid, AssetKind, Cell, Layer, Level, OverNode, Project, Spawn};
use serde_json::Value;

pub fn execute_tool_json(project: &mut Project, json_text: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(json_text).map_err(|e| format!("tool JSON: {e}"))?;
    execute_tool_value(project, &value)
}

pub fn execute_tool_value(project: &mut Project, value: &Value) -> Result<String, String> {
    let name = value
        .get("tool")
        .or_else(|| value.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("tool call needs a tool name")?;
    let args = value
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| value.clone());
    if name == "generate_image" || name == "draw_image" || name == "image" {
        return Err(
            "The workshop assistant cannot call the image model. Open the Image maker and press Generate."
                .into(),
        );
    }
    match name {
        "create_course" => create_course(project, &args),
        "rename_course" => rename_course(project, &args),
        "paint_rect" => paint_rect(project, &args),
        "paint_cell" => {
            let mut one = args.clone();
            if let Some(obj) = one.as_object_mut() {
                obj.insert("w".into(), Value::from(1));
                obj.insert("h".into(), Value::from(1));
            }
            paint_rect(project, &one)
        }
        "set_player_start" => set_player_start(project, &args),
        "rename_asset" => rename_asset(project, &args),
        "retarget_asset" => retarget_asset(project, &args),
        "add_overworld_node" => add_node(project, &args),
        "move_overworld_node" => move_node(project, &args),
        "link_nodes" => link_nodes(project, &args),
        "set_start_node" => set_start_node(project, &args),
        "set_course_style" => set_course_style(project, &args),
        "audit_course" => {
            let id = req_str(&args, &["level_id", "id"])?;
            audit_course(project, &id)
        }
        other => Err(format!("unknown tool {other}")),
    }
}

fn create_course(project: &mut Project, args: &Value) -> Result<String, String> {
    let name = req_str(args, &["name"])?;
    let w = req_i32(args, &["w", "width"]).unwrap_or(32).clamp(8, 128) as u32;
    let h = req_i32(args, &["h", "height"]).unwrap_or(15).clamp(8, 64) as u32;
    let id = nid("course-");
    project.levels.push(Level {
        id: id.clone(),
        name,
        style_id: opt_str(args, &["style_id"]).unwrap_or_default(),
        w,
        h,
        sky: "#24343c".into(),
        music_id: None,
        player: Spawn { x: 2.0, y: 2.0 },
        cells: Vec::new(),
        enemy_ai: crate::enemy_ai::EnemyAiStyle::Smb1Nes,
        entrances: Vec::new(),
        player_blueprint_id: None,
    });
    Ok(format!("created course {id}"))
}

fn rename_course(project: &mut Project, args: &Value) -> Result<String, String> {
    let id = req_str(args, &["level_id", "id"])?;
    let name = req_str(args, &["name"])?;
    let level = project
        .level_mut(&id)
        .ok_or_else(|| format!("no course {id}"))?;
    level.name = name.clone();
    Ok(format!("renamed {id} to {name}"))
}

fn paint_rect(project: &mut Project, args: &Value) -> Result<String, String> {
    let level_id = req_str(args, &["level_id", "course", "id"])?;
    let asset_id = req_str(args, &["asset_id", "asset"])?;
    if project.asset(&asset_id).is_none() {
        return Err(format!("no asset {asset_id}"));
    }
    let x = req_i32(args, &["x"])?;
    let y = req_i32(args, &["y"])?;
    let w = req_i32(args, &["w", "width"]).unwrap_or(1);
    let h = req_i32(args, &["h", "height"]).unwrap_or(1);
    if w <= 0 || h <= 0 || w > 256 || h > 256 {
        return Err("paint size must be between 1 and 256".into());
    }
    let layer = Layer::parse(opt_str(args, &["layer"]).as_deref().unwrap_or("main"));
    let level = project
        .level_mut(&level_id)
        .ok_or_else(|| format!("no course {level_id}"))?;
    let mut painted = 0u32;
    for yy in y..y + h {
        for xx in x..x + w {
            level
                .cells
                .retain(|c| !(c.x == xx && c.y == yy && c.layer == layer));
            level.cells.push(Cell {
                x: xx,
                y: yy,
                asset_id: asset_id.clone(),
                layer,
                ..Cell::default()
            });
            painted += 1;
        }
    }
    Ok(format!(
        "painted {painted} cells on {level_id} with {asset_id}"
    ))
}

fn set_player_start(project: &mut Project, args: &Value) -> Result<String, String> {
    let level_id = req_str(args, &["level_id", "id"])?;
    let x = req_f32(args, &["x"])?;
    let y = req_f32(args, &["y"])?;
    let level = project
        .level_mut(&level_id)
        .ok_or_else(|| format!("no course {level_id}"))?;
    level.player = Spawn { x, y };
    Ok(format!("player start on {level_id} is ({x}, {y})"))
}

fn rename_asset(project: &mut Project, args: &Value) -> Result<String, String> {
    let id = req_str(args, &["asset_id", "id"])?;
    let name = req_str(args, &["name"])?;
    let asset = project
        .asset_mut(&id)
        .ok_or_else(|| format!("no asset {id}"))?;
    asset.name = name.clone();
    Ok(format!("renamed asset {id} to {name}"))
}

fn retarget_asset(project: &mut Project, args: &Value) -> Result<String, String> {
    let id = req_str(args, &["asset_id", "id"])?;
    let kind_text = req_str(args, &["kind"])?;
    let kind = AssetKind::parse(&kind_text).ok_or_else(|| format!("unknown kind {kind_text}"))?;
    let asset = project
        .asset_mut(&id)
        .ok_or_else(|| format!("no asset {id}"))?;
    asset.kind = kind;
    asset.solid = kind.default_solid();
    Ok(format!("asset {id} is now {}", kind.label()))
}

fn add_node(project: &mut Project, args: &Value) -> Result<String, String> {
    let name = req_str(args, &["name"])?;
    let level_id = req_str(args, &["level_id"])?;
    if project.level(&level_id).is_none() {
        return Err(format!("no course {level_id}"));
    }
    let x = req_f32(args, &["x"]).unwrap_or(120.0);
    let y = req_f32(args, &["y"]).unwrap_or(160.0);
    let id = nid("node-");
    project.nodes.push(OverNode {
        id: id.clone(),
        name,
        level_id,
        x,
        y,
        links: Vec::new(),
    });
    if project.start_node_id.is_empty() {
        project.start_node_id = id.clone();
    }
    Ok(format!("added overworld node {id}"))
}

fn move_node(project: &mut Project, args: &Value) -> Result<String, String> {
    let id = req_str(args, &["node_id", "id"])?;
    let x = req_f32(args, &["x"])?;
    let y = req_f32(args, &["y"])?;
    let node = project
        .nodes
        .iter_mut()
        .find(|n| n.id == id)
        .ok_or_else(|| format!("no node {id}"))?;
    node.x = x;
    node.y = y;
    Ok(format!("moved {id} to ({x}, {y})"))
}

fn link_nodes(project: &mut Project, args: &Value) -> Result<String, String> {
    let from = req_str(args, &["from", "from_id"])?;
    let to = req_str(args, &["to", "to_id"])?;
    if !project.nodes.iter().any(|n| n.id == to) {
        return Err(format!("no node {to}"));
    }
    let node = project
        .nodes
        .iter_mut()
        .find(|n| n.id == from)
        .ok_or_else(|| format!("no node {from}"))?;
    if !node.links.iter().any(|id| id == &to) {
        node.links.push(to.clone());
    }
    Ok(format!(
        "linked {from} -> {to}. Clearing the source course unlocks the link."
    ))
}

fn set_course_style(project: &mut Project, args: &Value) -> Result<String, String> {
    let level_id = req_str(args, &["level_id", "id"])?;
    let style_id = req_str(args, &["style_id"])?;
    if !project.styles.iter().any(|style| style.id == style_id) {
        return Err(format!("no style {style_id}"));
    }
    let name = project.style_name(&style_id);
    let level = project
        .level_mut(&level_id)
        .ok_or_else(|| format!("no course {level_id}"))?;
    level.style_id = style_id;
    Ok(format!(
        "course {level_id} home style is {name}. Parts from other styles can still be painted on it."
    ))
}

fn set_start_node(project: &mut Project, args: &Value) -> Result<String, String> {
    let id = req_str(args, &["node_id", "id"])?;
    if !project.nodes.iter().any(|n| n.id == id) {
        return Err(format!("no node {id}"));
    }
    project.start_node_id = id.clone();
    Ok(format!("start node is {id}"))
}

fn opt_str(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(text) = value.get(*key).and_then(|v| v.as_str()) {
            return Some(text.to_string());
        }
    }
    None
}

fn req_str(value: &Value, keys: &[&str]) -> Result<String, String> {
    opt_str(value, keys).ok_or_else(|| format!("missing {}", keys[0]))
}

fn req_i32(value: &Value, keys: &[&str]) -> Result<i32, String> {
    for key in keys {
        if let Some(v) = value.get(*key) {
            if let Some(n) = v.as_i64() {
                return Ok(n as i32);
            }
            if let Some(n) = v.as_f64() {
                return Ok(n as i32);
            }
            if let Some(text) = v.as_str() {
                if let Ok(n) = text.parse::<i32>() {
                    return Ok(n);
                }
            }
        }
    }
    Err(format!("missing {}", keys[0]))
}

fn req_f32(value: &Value, keys: &[&str]) -> Result<f32, String> {
    for key in keys {
        if let Some(v) = value.get(*key) {
            if let Some(n) = v.as_f64() {
                return Ok(n as f32);
            }
            if let Some(n) = v.as_i64() {
                return Ok(n as f32);
            }
        }
    }
    Err(format!("missing {}", keys[0]))
}
