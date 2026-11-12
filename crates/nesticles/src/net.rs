//! Provider calls for the workshop assistant and the separate image maker.
//! Keys are redacted from any error string. The assistant has no image tool.

use crate::keys::KeyFile;
use nesticles_core::guard::redact_secrets;
use nesticles_core::pngutil::sprite_from_image;
use nesticles_core::tools::execute_tool_value;
use nesticles_core::Project;
use serde_json::{json, Value};

const ASSISTANT_RULES: &str = "\
You are the Nesticles workshop assistant. You edit a local original game with tools.\n\
You do not draw pictures and you must not call an image model. If the user asks you to draw, \
paint, illustrate, or generate a sprite, tell them to open the Image maker and press Generate there.\n\
Refuse requests to copy commercial characters. Ask for an original design instead.\n\
A cartridge does not label which sprite is the player. Physics is not extracted from cartridges. \
Scripts are custom per game. Do not claim otherwise.\n\
Coordinates are tile cells, origin at the top left, y growing downward.\n\
Use tools for concrete edits. After tools run, summarize what changed in a sentence or two.";

pub struct AssistDone {
    pub lines: Vec<(String, String)>,
    pub project: Project,
}

pub struct ImageDone {
    pub rgba: Vec<u8>,
    pub png: Vec<u8>,
    pub side: u32,
}

pub fn run_assistant(keys: &KeyFile, history: &[(String, String)], mut project: Project) -> Result<AssistDone, String> {
    if keys.assistant_key.trim().is_empty() {
        return Err("Add a workshop assistant key in API keys. The image maker uses its own key.".into());
    }
    let secrets = [keys.assistant_key.as_str(), keys.image_key.as_str()];
    let model = if keys.assistant_model.trim().is_empty() {
        keys.suggested_assistant_model().to_string()
    } else {
        keys.assistant_model.trim().to_string()
    };
    let system = format!("{ASSISTANT_RULES}\n\nCurrent project:\n{}", project.summary());
    let provider = keys.assistant_provider.as_str();
    let result = if provider == "anthropic" {
        anthropic_loop(keys, &model, &system, history, &mut project)
    } else {
        openai_loop(keys, &model, &system, history, &mut project)
    };
    result.map_err(|e| redact_secrets(&e, &secrets))
}

pub fn run_image(keys: &KeyFile, prompt: &str, side: u32) -> Result<ImageDone, String> {
    if nesticles_core::image_prompt_blocked(prompt).is_some() {
        return Err(
            "That prompt is refused because it names a commercial character. Describe an original design."
                .into(),
        );
    }
    if keys.image_key.trim().is_empty() {
        return Err("Add an image maker key in API keys. It is separate from the workshop assistant key.".into());
    }
    let secrets = [keys.image_key.as_str(), keys.assistant_key.as_str()];
    let model = if keys.image_model.trim().is_empty() {
        "gpt-image-1".to_string()
    } else {
        keys.image_model.trim().to_string()
    };
    fetch_image(keys, &model, prompt)
        .and_then(|bytes| {
            let (w, _h, rgba, png) = sprite_from_image(&bytes, side)?;
            Ok(ImageDone {
                rgba,
                png,
                side: w,
            })
        })
        .map_err(|e| redact_secrets(&e, &secrets))
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .user_agent("nesticles/0.1")
        .build()
        .map_err(|e| e.to_string())
}

fn openai_loop(
    keys: &KeyFile,
    model: &str,
    system: &str,
    history: &[(String, String)],
    project: &mut Project,
) -> Result<AssistDone, String> {
    let url = chat_url(&keys.assistant_base_url);
    let mut messages = vec![json!({"role": "system", "content": system})];
    for (role, text) in history.iter().rev().take(12).collect::<Vec<_>>().into_iter().rev() {
        let role = if role == "assistant" { "assistant" } else { "user" };
        messages.push(json!({"role": role, "content": text}));
    }
    let tools = openai_tools();
    let http = client()?;
    let mut lines = Vec::new();
    for _ in 0..5 {
        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "temperature": 0.3,
        });
        let response = http
            .post(&url)
            .bearer_auth(&keys.assistant_key)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let text = response.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("assistant request failed ({status}): {}", clip(&text)));
        }
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("assistant response: {e}"))?;
        let message = value
            .pointer("/choices/0/message")
            .cloned()
            .ok_or_else(|| format!("assistant response had no message: {}", clip(&text)))?;
        let content = message.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let tool_calls = message.get("tool_calls").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        if tool_calls.is_empty() {
            if !content.is_empty() {
                lines.push(("assistant".into(), content));
            } else {
                lines.push(("assistant".into(), "I have nothing to add.".into()));
            }
            break;
        }
        messages.push(message);
        if !content.is_empty() {
            lines.push(("assistant".into(), content));
        }
        for call in tool_calls {
            let id = call.get("id").and_then(|v| v.as_str()).unwrap_or("tool").to_string();
            let name = call
                .pointer("/function/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = call.pointer("/function/arguments").and_then(|v| v.as_str()).unwrap_or("{}");
            let parsed: Value = serde_json::from_str(arguments).unwrap_or(json!({}));
            let mut payload = json!({"tool": name});
            if let (Some(obj), Some(args)) = (payload.as_object_mut(), parsed.as_object()) {
                for (k, v) in args {
                    obj.insert(k.clone(), v.clone());
                }
            }
            let result = execute_tool_value(project, &payload).unwrap_or_else(|e| format!("error: {e}"));
            lines.push(("tool".into(), format!("{name}: {result}")));
            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": result,
            }));
        }
    }
    Ok(AssistDone {
        lines,
        project: project.clone(),
    })
}

fn anthropic_loop(
    keys: &KeyFile,
    model: &str,
    system: &str,
    history: &[(String, String)],
    project: &mut Project,
) -> Result<AssistDone, String> {
    let http = client()?;
    let mut messages: Vec<Value> = Vec::new();
    for (role, text) in history.iter().rev().take(12).collect::<Vec<_>>().into_iter().rev() {
        let role = if role == "assistant" { "assistant" } else { "user" };
        messages.push(json!({"role": role, "content": text}));
    }
    if messages.is_empty() {
        messages.push(json!({"role": "user", "content": "Hello"}));
    }
    let tools = anthropic_tools();
    let mut lines = Vec::new();
    for _ in 0..5 {
        let body = json!({
            "model": model,
            "max_tokens": 1200,
            "system": system,
            "tools": tools,
            "messages": messages,
        });
        let response = http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &keys.assistant_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        let status = response.status();
        let text = response.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("assistant request failed ({status}): {}", clip(&text)));
        }
        let value: Value = serde_json::from_str(&text).map_err(|e| format!("assistant response: {e}"))?;
        let blocks = value
            .get("content")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        messages.push(json!({"role": "assistant", "content": blocks}));
        let mut tool_results = Vec::new();
        let mut any_tool = false;
        for block in &blocks {
            let kind = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if kind == "text" {
                let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                if !text.is_empty() {
                    lines.push(("assistant".into(), text));
                }
            } else if kind == "tool_use" {
                any_tool = true;
                let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("tool");
                let input = block.get("input").cloned().unwrap_or(json!({}));
                let mut payload = json!({"tool": name});
                if let (Some(obj), Some(args)) = (payload.as_object_mut(), input.as_object()) {
                    for (k, v) in args {
                        obj.insert(k.clone(), v.clone());
                    }
                }
                let result = execute_tool_value(project, &payload).unwrap_or_else(|e| format!("error: {e}"));
                lines.push(("tool".into(), format!("{name}: {result}")));
                tool_results.push(json!({
                    "type": "tool_result",
                    "tool_use_id": id,
                    "content": result,
                }));
            }
        }
        if !any_tool {
            if lines.is_empty() {
                lines.push(("assistant".into(), "I have nothing to add.".into()));
            }
            break;
        }
        messages.push(json!({"role": "user", "content": tool_results}));
    }
    Ok(AssistDone {
        lines,
        project: project.clone(),
    })
}

fn fetch_image(keys: &KeyFile, model: &str, prompt: &str) -> Result<Vec<u8>, String> {
    let url = if keys.image_provider == "custom" {
        images_url(&keys.image_base_url)
    } else {
        "https://api.openai.com/v1/images/generations".into()
    };
    let mut body = json!({
        "model": model,
        "prompt": prompt,
        "n": 1,
        "size": "1024x1024",
    });
    if model.starts_with("dall-e") {
        body["response_format"] = json!("b64_json");
    }
    let http = client()?;
    let response = http
        .post(&url)
        .bearer_auth(&keys.image_key)
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let text = response.text().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("image request failed ({status}): {}", clip(&text)));
    }
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("image response: {e}"))?;
    if let Some(b64) = value.pointer("/data/0/b64_json").and_then(|v| v.as_str()) {
        return base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| e.to_string());
    }
    if let Some(url) = value.pointer("/data/0/url").and_then(|v| v.as_str()) {
        let bytes = http.get(url).send().map_err(|e| e.to_string())?.bytes().map_err(|e| e.to_string())?;
        return Ok(bytes.to_vec());
    }
    Err(format!("image response had no picture: {}", clip(&text)))
}

fn chat_url(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.is_empty() {
        return "https://api.openai.com/v1/chat/completions".into();
    }
    if b.ends_with("/chat/completions") {
        return b.to_string();
    }
    if b.ends_with("/v1") {
        format!("{b}/chat/completions")
    } else {
        format!("{b}/v1/chat/completions")
    }
}

fn images_url(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.is_empty() {
        return "https://api.openai.com/v1/images/generations".into();
    }
    if b.ends_with("/images/generations") {
        return b.to_string();
    }
    if b.ends_with("/v1") {
        format!("{b}/images/generations")
    } else {
        format!("{b}/v1/images/generations")
    }
}

fn clip(text: &str) -> String {
    let trimmed = text.trim().replace('\n', " ");
    if trimmed.chars().count() > 400 {
        let end = trimmed.char_indices().nth(400).map(|(i, _)| i).unwrap_or(trimmed.len());
        format!("{}…", &trimmed[..end])
    } else {
        trimmed
    }
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "parameters": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

fn tool_defs() -> Vec<Value> {
    vec![
        tool(
            "create_course",
            "Create an empty course.",
            json!({"name": {"type": "string"}, "w": {"type": "integer"}, "h": {"type": "integer"}}),
            &["name"],
        ),
        tool(
            "rename_course",
            "Rename a course by id.",
            json!({"level_id": {"type": "string"}, "name": {"type": "string"}}),
            &["level_id", "name"],
        ),
        tool(
            "paint_rect",
            "Paint a rectangle of an existing asset onto a course, in tile cells.",
            json!({
                "level_id": {"type": "string"},
                "asset_id": {"type": "string"},
                "x": {"type": "integer"},
                "y": {"type": "integer"},
                "w": {"type": "integer"},
                "h": {"type": "integer"},
                "layer": {"type": "string", "enum": ["back", "main", "front"]}
            }),
            &["level_id", "asset_id", "x", "y", "w", "h"],
        ),
        tool(
            "paint_cell",
            "Paint one cell of an existing asset.",
            json!({
                "level_id": {"type": "string"},
                "asset_id": {"type": "string"},
                "x": {"type": "integer"},
                "y": {"type": "integer"},
                "layer": {"type": "string"}
            }),
            &["level_id", "asset_id", "x", "y"],
        ),
        tool(
            "set_player_start",
            "Set the player start cell on a course.",
            json!({"level_id": {"type": "string"}, "x": {"type": "number"}, "y": {"type": "number"}}),
            &["level_id", "x", "y"],
        ),
        tool(
            "rename_asset",
            "Rename a library asset.",
            json!({"asset_id": {"type": "string"}, "name": {"type": "string"}}),
            &["asset_id", "name"],
        ),
        tool(
            "retarget_asset",
            "Change an asset kind. Kinds: player, enemy, terrain, background, foreground, item, hazard, goal, sfx, music.",
            json!({"asset_id": {"type": "string"}, "kind": {"type": "string"}}),
            &["asset_id", "kind"],
        ),
        tool(
            "add_overworld_node",
            "Add an overworld node that opens a course.",
            json!({"name": {"type": "string"}, "level_id": {"type": "string"}, "x": {"type": "number"}, "y": {"type": "number"}}),
            &["name", "level_id"],
        ),
        tool(
            "move_overworld_node",
            "Move an overworld node.",
            json!({"node_id": {"type": "string"}, "x": {"type": "number"}, "y": {"type": "number"}}),
            &["node_id", "x", "y"],
        ),
        tool(
            "link_nodes",
            "Link two overworld nodes. Clearing the source course unlocks the link.",
            json!({"from": {"type": "string"}, "to": {"type": "string"}}),
            &["from", "to"],
        ),
        tool(
            "set_start_node",
            "Choose the overworld node that is open at the start.",
            json!({"node_id": {"type": "string"}}),
            &["node_id"],
        ),
        tool(
            "audit_course",
            "Check whether a course has ground, a start inside the map, and a goal.",
            json!({"level_id": {"type": "string"}}),
            &["level_id"],
        ),
    ]
}

fn openai_tools() -> Value {
    Value::Array(
        tool_defs()
            .into_iter()
            .map(|spec| {
                json!({
                    "type": "function",
                    "function": {
                        "name": spec["name"],
                        "description": spec["description"],
                        "parameters": spec["parameters"],
                    }
                })
            })
            .collect(),
    )
}

fn anthropic_tools() -> Value {
    Value::Array(
        tool_defs()
            .into_iter()
            .map(|spec| {
                json!({
                    "name": spec["name"],
                    "description": spec["description"],
                    "input_schema": spec["parameters"],
                })
            })
            .collect(),
    )
}
