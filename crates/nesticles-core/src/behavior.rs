//! Restricted behavior graphs. Nodes are data. Nothing in a graph is compiled or executed
//! as Rust, Python, or a shell command.
//!
//! A derivative copies the parent graph and may replace one node. Hammer Bro's runtime
//! patrol and jump stay in the SMB1 enemy stepper. This module does not reskin that hammer.
//! A returning projectile is its own primitive with launch, outbound travel, turn, return,
//! and catch or despawn.

use serde::{Deserialize, Serialize};

pub const BEHAVIOR_SCHEMA: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorNode {
    /// Label only. The SMB1 walker already moves. This does not add a second physics step.
    Patrol,
    /// Label only. Hammer Bro already jumps in the SMB1 stepper.
    Jump,
    Wait {
        ticks: u32,
    },
    DetectPlayer,
    FacePlayer,
    ChangeAnimation {
        state: String,
    },
    RespondToHit,
    Die,
    TriggerEvent {
        name: String,
    },
    /// Straight shot. Not a boomerang.
    FireStraight {
        speed: f32,
    },
    /// Returning projectile. `range` is how far it travels before it turns, in pixels.
    FireReturning {
        speed: f32,
        range: f32,
    },
}

impl BehaviorNode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Patrol => "Patrol",
            Self::Jump => "Jump",
            Self::Wait { .. } => "Wait",
            Self::DetectPlayer => "Detect player",
            Self::FacePlayer => "Face player",
            Self::ChangeAnimation { .. } => "Change animation",
            Self::RespondToHit => "Respond to hit",
            Self::Die => "Die",
            Self::TriggerEvent { .. } => "Trigger event",
            Self::FireStraight { .. } => "Fire projectile",
            Self::FireReturning { .. } => "Fire returning projectile",
        }
    }

    /// Nodes the Rust Play loop actually steps. The others are stored and labeled unavailable.
    pub fn runtime_ready(&self) -> bool {
        matches!(
            self,
            Self::Wait { .. }
                | Self::FireReturning { .. }
                | Self::ChangeAnimation { .. }
                | Self::Die
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BehaviorGraph {
    pub schema: u32,
    pub id: String,
    pub name: String,
    /// Parent graph this one was copied from. Empty when it is an original.
    #[serde(default)]
    pub parent_id: String,
    /// Archetype name that supplied the copied patrol and jump, when one existed.
    #[serde(default)]
    pub derived_from: String,
    /// Blueprint this graph is attached to. Empty until the user attaches it.
    #[serde(default)]
    pub blueprint_id: String,
    pub nodes: Vec<BehaviorNode>,
}

impl BehaviorGraph {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: BEHAVIOR_SCHEMA,
            id: id.into(),
            name: name.into(),
            parent_id: String::new(),
            derived_from: String::new(),
            blueprint_id: String::new(),
            nodes: Vec::new(),
        }
    }

    /// Copy every node, then drop straight shots and append a returning projectile.
    /// Patrol and jump nodes are kept as labels. They are not a new physics model.
    pub fn derive_returning(
        &self,
        id: impl Into<String>,
        name: impl Into<String>,
        speed: f32,
        range: f32,
    ) -> Self {
        let mut nodes: Vec<BehaviorNode> = self
            .nodes
            .iter()
            .filter(|node| {
                !matches!(
                    node,
                    BehaviorNode::FireStraight { .. } | BehaviorNode::FireReturning { .. }
                )
            })
            .cloned()
            .collect();
        nodes.push(BehaviorNode::FireReturning { speed, range });
        Self {
            schema: BEHAVIOR_SCHEMA,
            id: id.into(),
            name: name.into(),
            parent_id: self.id.clone(),
            derived_from: self.name.clone(),
            blueprint_id: self.blueprint_id.clone(),
            nodes,
        }
    }

    pub fn unavailable(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|node| !node.runtime_ready())
            .map(|node| {
                format!(
                    "{} is stored but the Rust Play loop does not step it on its own.",
                    node.label()
                )
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flight {
    Out,
    Turn,
    Back,
    Done,
}

/// One returning projectile. Positions are course pixels. Speed is pixels per tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Boomerang {
    pub x: f32,
    pub y: f32,
    pub owner_x: f32,
    pub owner_y: f32,
    pub dir: f32,
    pub speed: f32,
    pub range: f32,
    pub traveled: f32,
    pub flight: Flight,
    /// Set when the projectile overlaps the owner on the way back, or when it expires.
    pub caught: bool,
}

impl Boomerang {
    pub fn launch(owner_x: f32, owner_y: f32, dir: f32, speed: f32, range: f32) -> Self {
        let dir = if dir < 0.0 { -1.0 } else { 1.0 };
        Self {
            x: owner_x,
            y: owner_y,
            owner_x,
            owner_y,
            dir,
            speed: speed.abs().max(0.5),
            range: range.abs().max(1.0),
            traveled: 0.0,
            flight: Flight::Out,
            caught: false,
        }
    }

    /// One tick. Outbound, a one-tick turn, return, then catch near the owner or a hard stop.
    pub fn step(&mut self) {
        if self.flight == Flight::Done {
            return;
        }
        match self.flight {
            Flight::Out => {
                self.x += self.dir * self.speed;
                self.traveled += self.speed;
                if self.traveled >= self.range {
                    self.flight = Flight::Turn;
                }
            }
            Flight::Turn => {
                self.dir = -self.dir;
                self.flight = Flight::Back;
            }
            Flight::Back => {
                self.x += self.dir * self.speed;
                let back = (self.x - self.owner_x).abs() <= self.speed;
                if back {
                    self.x = self.owner_x;
                    self.flight = Flight::Done;
                    self.caught = true;
                } else if self.traveled > self.range * 4.0 {
                    self.flight = Flight::Done;
                    self.caught = false;
                }
                self.traveled += self.speed;
            }
            Flight::Done => {}
        }
    }
}

/// Launch from the first returning node. Other nodes are not run as code.
pub fn launch_from(
    graph: &BehaviorGraph,
    owner_x: f32,
    owner_y: f32,
    dir: f32,
) -> Option<Boomerang> {
    graph.nodes.iter().find_map(|node| match node {
        BehaviorNode::FireReturning { speed, range } => {
            Some(Boomerang::launch(owner_x, owner_y, dir, *speed, *range))
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hammer() -> BehaviorGraph {
        let mut graph = BehaviorGraph::new("hammer", "Hammer Bro");
        graph.nodes = vec![
            BehaviorNode::Patrol,
            BehaviorNode::Jump,
            BehaviorNode::DetectPlayer,
            BehaviorNode::FireStraight { speed: 2.0 },
        ];
        graph
    }

    #[test]
    fn derivative_replaces_the_straight_shot() {
        let derived = hammer().derive_returning("boom", "Boomerang Bro", 2.0, 16.0);
        assert_eq!(derived.parent_id, "hammer");
        assert!(derived
            .nodes
            .iter()
            .any(|node| matches!(node, BehaviorNode::FireReturning { .. })));
        assert!(!derived
            .nodes
            .iter()
            .any(|node| matches!(node, BehaviorNode::FireStraight { .. })));
        assert!(derived
            .nodes
            .iter()
            .any(|node| matches!(node, BehaviorNode::Patrol)));
        assert!(!derived.unavailable().is_empty());
    }

    #[test]
    fn boomerang_leaves_turns_returns_and_is_caught() {
        let mut boom = Boomerang::launch(0.0, 8.0, 1.0, 4.0, 16.0);
        let mut saw_out = false;
        let mut saw_back = false;
        for _ in 0..30 {
            boom.step();
            if boom.flight == Flight::Out {
                saw_out = true;
                assert!(boom.x > 0.0);
            }
            if boom.flight == Flight::Back {
                saw_back = true;
            }
            if boom.flight == Flight::Done {
                break;
            }
        }
        assert!(saw_out);
        assert!(saw_back);
        assert_eq!(boom.flight, Flight::Done);
        assert!(boom.caught);
        assert_eq!(boom.x, 0.0);
    }

    #[test]
    fn a_straight_shot_does_not_become_a_boomerang() {
        assert!(launch_from(&hammer(), 0.0, 0.0, 1.0).is_none());
    }
}
