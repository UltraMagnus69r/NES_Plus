//! Named entrances and directed exits.
//!
//! An entrance is where the player appears. A transition is the placed part
//! that sends them there. A return trip is a second transition, not a flag on
//! the first one. Picture names are not consulted.
//!
//! These records are plain data with stable ids, so a later Godot export can
//! turn an entrance into a spawn marker and a transition into a resource.
//! This module does not build that export.
//!
//! Opening a project does not rewrite it. [`ensure_legacy_transitions`] is the
//! explicit, idempotent adapter from a Phase 4 coordinate warp.

use crate::blueprint::{LegacyHazard, LegacyRole};
use crate::project::nid;
use crate::project::{
    Cell, Entrance, EntryDirection, Facing, PlacementWarp, Project, Transition, WarpTarget,
};

/// Where a named exit puts the player, already checked against the course.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BakedExit {
    /// Empty means the course the player is already in.
    pub level_id: String,
    pub x: i32,
    pub y: i32,
    pub facing: i32,
}

/// Result of reading `cell.transition_id`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamedExit {
    /// No named exit. The coordinate warp, if any, still applies.
    Absent,
    Ready(BakedExit),
    /// The named exit is broken. Do not use a coordinate warp instead.
    Fault(String),
}

pub fn add_entrance(
    project: &mut Project,
    level_id: &str,
    name: &str,
    x: i32,
    y: i32,
    facing: Facing,
) -> Result<String, String> {
    let id = nid("entrance-");
    insert_entrance(project, level_id, id, name, x, y, facing)
}

pub fn update_entrance(
    project: &mut Project,
    level_id: &str,
    entrance_id: &str,
    name: &str,
    x: i32,
    y: i32,
    facing: Facing,
) -> Result<(), String> {
    check_cell(project, level_id, x, y)?;
    let name = clean_name(name)?;
    let level = project
        .level_mut(level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    let entrance = level
        .entrances
        .iter_mut()
        .find(|entrance| entrance.id == entrance_id)
        .ok_or_else(|| format!("No entrance with id {entrance_id}."))?;
    entrance.name = name;
    entrance.x = x;
    entrance.y = y;
    entrance.facing = facing;
    Ok(())
}

pub fn remove_entrance(
    project: &mut Project,
    level_id: &str,
    entrance_id: &str,
) -> Result<(), String> {
    if let Some(link) = project.transitions.iter().find(|link| {
        destination_level(link) == level_id && link.destination_entrance_id == entrance_id
    }) {
        return Err(format!(
            "Entrance is used by the exit at {},{}. Clear that exit before removing it.",
            link.source_x, link.source_y
        ));
    }
    let level = project
        .level_mut(level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    let before = level.entrances.len();
    level
        .entrances
        .retain(|entrance| entrance.id != entrance_id);
    if level.entrances.len() == before {
        return Err(format!("No entrance with id {entrance_id}."));
    }
    Ok(())
}

/// Point one placed pipe at an entrance. Does not create a return trip.
/// The pipe's entry direction is updated to match. Its picture is not changed.
pub fn set_exit(
    project: &mut Project,
    level_id: &str,
    x: i32,
    y: i32,
    destination_level_id: &str,
    destination_entrance_id: &str,
    entry: EntryDirection,
) -> Result<String, String> {
    let dest_level = if destination_level_id.is_empty() {
        level_id
    } else {
        destination_level_id
    };
    let entrance = project
        .level(dest_level)
        .ok_or_else(|| format!("No course with id {dest_level}. The exit was not saved."))?
        .entrances
        .iter()
        .find(|entrance| entrance.id == destination_entrance_id)
        .cloned()
        .ok_or_else(|| {
            format!("No entrance with id {destination_entrance_id}. The exit was not saved.")
        })?;
    check_cell(project, dest_level, entrance.x, entrance.y)?;
    point_pipe(project, level_id, x, y, entry)?;
    let id = {
        let existing = project.transitions.iter().find(|link| {
            link.source_level_id == level_id && link.source_x == x && link.source_y == y
        });
        if let Some(link) = existing {
            link.id.clone()
        } else {
            format!("exit-{level_id}-{x}-{y}")
        }
    };
    let record = Transition {
        id: id.clone(),
        source_level_id: level_id.to_string(),
        source_x: x,
        source_y: y,
        destination_level_id: if dest_level == level_id {
            String::new()
        } else {
            dest_level.to_string()
        },
        destination_entrance_id: destination_entrance_id.to_string(),
        entry,
    };
    if let Some(link) = project.transitions.iter_mut().find(|link| link.id == id) {
        *link = record;
    } else {
        project.transitions.push(record);
    }
    let cell = cell_mut(project, level_id, x, y)?;
    cell.transition_id = Some(id.clone());
    Ok(id)
}

/// Add the opposite trip. The return pipe must already be placed.
/// Calling this again updates that same return instead of adding a third link.
pub fn add_return(
    project: &mut Project,
    transition_id: &str,
    return_x: i32,
    return_y: i32,
    return_entrance_id: &str,
    return_entry: EntryDirection,
) -> Result<String, String> {
    let forward = project
        .transitions
        .iter()
        .find(|link| link.id == transition_id)
        .cloned()
        .ok_or_else(|| format!("No exit with id {transition_id}."))?;
    let return_level = destination_level(&forward).to_string();
    let home = forward.source_level_id.clone();
    set_exit(
        project,
        &return_level,
        return_x,
        return_y,
        &home,
        return_entrance_id,
        return_entry,
    )
}

pub fn remove_return(project: &mut Project, transition_id: &str) -> Result<(), String> {
    let forward = project
        .transitions
        .iter()
        .find(|link| link.id == transition_id)
        .cloned()
        .ok_or_else(|| format!("No exit with id {transition_id}."))?;
    let return_level = destination_level(&forward).to_string();
    let home = forward.source_level_id.clone();
    let spot = project
        .level(&return_level)
        .and_then(|level| {
            level
                .entrances
                .iter()
                .find(|entrance| entrance.id == forward.destination_entrance_id)
        })
        .map(|entrance| (entrance.x, entrance.y));
    let Some((sx, sy)) = spot else {
        return Err("This exit has no return connection.".into());
    };
    let reverse = project
        .transitions
        .iter()
        .find(|link| {
            link.id != forward.id
                && link.source_level_id == return_level
                && destination_level(link) == home
                && opens_onto(link.entry, link.source_x, link.source_y, sx, sy)
        })
        .cloned();
    let Some(reverse) = reverse else {
        return Err("This exit has no return connection.".into());
    };
    clear_named_exit(
        project,
        &reverse.source_level_id,
        reverse.source_x,
        reverse.source_y,
    )
}

/// Drop the named exit and keep the coordinate warp, if one is still stored.
pub fn clear_named_exit(
    project: &mut Project,
    level_id: &str,
    x: i32,
    y: i32,
) -> Result<(), String> {
    let id = {
        let cell = cell_mut(project, level_id, x, y)?;
        cell.transition_id.take()
    };
    if let Some(id) = id {
        let still = project
            .levels
            .iter()
            .flat_map(|level| level.cells.iter())
            .any(|cell| cell.transition_id.as_deref() == Some(id.as_str()));
        if !still {
            project.transitions.retain(|link| link.id != id);
        }
    }
    Ok(())
}

/// Named exit off, and the coordinate warp off, so nothing is left to fall back to.
pub fn clear_exit(project: &mut Project, level_id: &str, x: i32, y: i32) -> Result<(), String> {
    clear_named_exit(project, level_id, x, y)?;
    crate::blueprint::set_placement_warp(project, level_id, x, y, PlacementWarp::Cleared)
}

pub fn named_exit(project: &Project, source_level: &str, cell: &Cell) -> NamedExit {
    let Some(id) = cell.transition_id.as_deref() else {
        return NamedExit::Absent;
    };
    let Some(link) = project.transitions.iter().find(|link| link.id == id) else {
        return NamedExit::Fault(format!(
            "The part at {},{} uses exit \"{id}\", but that exit is missing. The player will not be moved.",
            cell.x, cell.y
        ));
    };
    let dest_level = destination_level(link);
    let Some(level) = project.level(dest_level) else {
        return NamedExit::Fault(format!(
            "The part at {},{} points at course \"{dest_level}\", but that course is missing. The player will not be moved.",
            cell.x, cell.y
        ));
    };
    let Some(entrance) = level
        .entrances
        .iter()
        .find(|entrance| entrance.id == link.destination_entrance_id)
    else {
        return NamedExit::Fault(format!(
            "The part at {},{} points at entrance \"{}\", but that entrance is missing. The player will not be moved.",
            cell.x, cell.y, link.destination_entrance_id
        ));
    };
    if entrance.x < 0
        || entrance.y < 0
        || entrance.x >= level.w as i32
        || entrance.y >= level.h as i32
    {
        return NamedExit::Fault(format!(
            "Entrance \"{}\" at {},{} is outside its course. The player will not be moved.",
            entrance.name, entrance.x, entrance.y
        ));
    }
    let same = dest_level == source_level;
    NamedExit::Ready(BakedExit {
        level_id: if same {
            String::new()
        } else {
            dest_level.to_string()
        },
        x: entrance.x,
        y: entrance.y,
        facing: entrance.facing.sign(),
    })
}

/// Turn concrete coordinate warps into entrances and exits.
/// Runs only when called. A second call keeps the same ids and does not add a return trip.
pub fn ensure_legacy_transitions(project: &mut Project) {
    let sources: Vec<(String, i32, i32)> = project
        .levels
        .iter()
        .flat_map(|level| {
            level.cells.iter().filter_map(|cell| {
                if cell.transition_id.is_some()
                    || matches!(cell.warp_choice, PlacementWarp::Cleared)
                {
                    return None;
                }
                Some((level.id.clone(), cell.x, cell.y))
            })
        })
        .collect();
    for (level_id, x, y) in sources {
        let Some(target) = concrete_target(project, &level_id, x, y) else {
            continue;
        };
        let entry = cell_entry(project, &level_id, x, y);
        let dest_level = if target.level_id.is_empty() {
            level_id.clone()
        } else {
            target.level_id.clone()
        };
        if project.level(&dest_level).is_none() {
            continue;
        }
        let entrance_id = format!("entrance-{dest_level}-{}-{}", target.x, target.y);
        let transition_id = format!("exit-{level_id}-{x}-{y}");
        if let Some(level) = project.level_mut(&dest_level) {
            if !level
                .entrances
                .iter()
                .any(|entrance| entrance.id == entrance_id)
            {
                if target.x < 0
                    || target.y < 0
                    || target.x >= level.w as i32
                    || target.y >= level.h as i32
                {
                    continue;
                }
                level.entrances.push(Entrance {
                    id: entrance_id.clone(),
                    name: format!("At {},{}", target.x, target.y),
                    x: target.x,
                    y: target.y,
                    facing: Facing::Right,
                });
            }
        }
        let record = Transition {
            id: transition_id.clone(),
            source_level_id: level_id.clone(),
            source_x: x,
            source_y: y,
            destination_level_id: if dest_level == level_id {
                String::new()
            } else {
                dest_level
            },
            destination_entrance_id: entrance_id,
            entry,
        };
        if let Some(link) = project
            .transitions
            .iter_mut()
            .find(|link| link.id == transition_id)
        {
            *link = record;
        } else {
            project.transitions.push(record);
        }
        if let Some(cell) = cell_mut(project, &level_id, x, y).ok() {
            cell.transition_id = Some(transition_id);
        }
    }
}

pub fn pipe_hazard(entry: EntryDirection) -> LegacyHazard {
    match entry {
        EntryDirection::Up => LegacyHazard::PipeUp,
        EntryDirection::Down => LegacyHazard::PipeDown,
        EntryDirection::Left => LegacyHazard::PipeLeft,
        EntryDirection::Right => LegacyHazard::PipeRight,
    }
}

fn insert_entrance(
    project: &mut Project,
    level_id: &str,
    id: String,
    name: &str,
    x: i32,
    y: i32,
    facing: Facing,
) -> Result<String, String> {
    check_cell(project, level_id, x, y)?;
    let name = clean_name(name)?;
    let level = project
        .level_mut(level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    if level.entrances.iter().any(|entrance| entrance.id == id) {
        return Err(format!(
            "An entrance with id {id} is already on this course."
        ));
    }
    level.entrances.push(Entrance {
        id: id.clone(),
        name,
        x,
        y,
        facing,
    });
    Ok(id)
}

fn check_cell(project: &Project, level_id: &str, x: i32, y: i32) -> Result<(), String> {
    let level = project
        .level(level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    if x < 0 || y < 0 || x >= level.w as i32 || y >= level.h as i32 {
        return Err(format!(
            "{},{} is outside {} ({} by {}).",
            x, y, level_id, level.w, level.h
        ));
    }
    Ok(())
}

fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("An entrance needs a name.".into());
    }
    Ok(name.to_string())
}

fn opens_onto(entry: EntryDirection, sx: i32, sy: i32, ex: i32, ey: i32) -> bool {
    let on_cell = sx == ex && sy == ey;
    let at_opening = match entry {
        EntryDirection::Down => sx == ex && sy == ey + 1,
        EntryDirection::Up => sx == ex && sy == ey - 1,
        EntryDirection::Left => sx == ex - 1 && sy == ey,
        EntryDirection::Right => sx == ex + 1 && sy == ey,
    };
    on_cell || at_opening
}

fn destination_level(link: &Transition) -> &str {
    if link.destination_level_id.is_empty() {
        link.source_level_id.as_str()
    } else {
        link.destination_level_id.as_str()
    }
}

fn cell_mut<'a>(
    project: &'a mut Project,
    level_id: &str,
    x: i32,
    y: i32,
) -> Result<&'a mut Cell, String> {
    let level = project
        .level_mut(level_id)
        .ok_or_else(|| format!("no course with id {level_id}"))?;
    level
        .cells
        .iter_mut()
        .rev()
        .find(|cell| cell.x == x && cell.y == y)
        .ok_or_else(|| format!("no part at {x},{y}"))
}

fn point_pipe(
    project: &mut Project,
    level_id: &str,
    x: i32,
    y: i32,
    entry: EntryDirection,
) -> Result<(), String> {
    let parts = crate::blueprint::placed_fixture(project, level_id, x, y)?
        .ok_or_else(|| "This part is not a pipe. Set a pipe direction first.".to_string())?;
    if !matches!(
        parts.hazard,
        Some(
            LegacyHazard::PipeDown
                | LegacyHazard::PipeUp
                | LegacyHazard::PipeLeft
                | LegacyHazard::PipeRight
        )
    ) {
        return Err("This part is not a pipe. Set a pipe direction first.".into());
    }
    let block = parts.block.or(Some(crate::blueprint::LegacyBlock::Solid));
    let role = crate::blueprint::compose_fixture(block, parts.item, Some(pipe_hazard(entry)))?;
    crate::blueprint::assign_cell(project, level_id, x, y, role)?;
    Ok(())
}

fn concrete_target(project: &Project, level_id: &str, x: i32, y: i32) -> Option<WarpTarget> {
    let level = project.level(level_id)?;
    let cell = level
        .cells
        .iter()
        .rev()
        .find(|cell| cell.x == x && cell.y == y)?;
    match &cell.warp_choice {
        PlacementWarp::Cleared => None,
        PlacementWarp::Destination(target) => Some(target.clone()),
        PlacementWarp::Inherit => {
            let id = cell.blueprint_id.as_deref()?;
            let role = project
                .blueprints
                .iter()
                .find(|blueprint| blueprint.id == id)
                .map(|blueprint| &blueprint.legacy_role)?;
            role_warp(role)
        }
    }
}

fn role_warp(role: &LegacyRole) -> Option<WarpTarget> {
    let warp = match role {
        LegacyRole::Fixture { warp, .. }
        | LegacyRole::Enemy { warp, .. }
        | LegacyRole::Goal { warp } => warp.clone(),
        LegacyRole::Inert => None,
    }?;
    Some(WarpTarget {
        level_id: warp.level_id,
        x: warp.x,
        y: warp.y,
    })
}

#[cfg(test)]
#[path = "transition_tests.rs"]
mod transition_tests;

fn cell_entry(project: &Project, level_id: &str, x: i32, y: i32) -> EntryDirection {
    let Ok(Some(parts)) = crate::blueprint::placed_fixture(project, level_id, x, y) else {
        return EntryDirection::Down;
    };
    match parts.hazard {
        Some(LegacyHazard::PipeUp) => EntryDirection::Up,
        Some(LegacyHazard::PipeLeft) => EntryDirection::Left,
        Some(LegacyHazard::PipeRight) => EntryDirection::Right,
        _ => EntryDirection::Down,
    }
}
