//! R3 decision rules for one reconstruction issue.
//!
//! A suggestion is a hypothesis. It is not a verified finding and it does not
//! write a sprite. Saved decisions live on `Project::issue_decisions`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub id: String,
    pub asset_id: String,
    pub question: String,
    pub evidence_ids: Vec<String>,
    pub suggestion: Suggestion,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub summary: String,
    /// Tile indexes in the order they were proposed. Empty means no grouping.
    pub grouping: Vec<u32>,
}

impl Suggestion {
    /// Suggestions from this module are never verified findings.
    pub fn is_verified_finding(&self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Accept,
    Skip,
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub issue_id: String,
    pub asset_id: String,
    /// Derived from the asset id and the tile order. Not a process id.
    #[serde(default)]
    pub fingerprint: String,
    pub kind: DecisionKind,
    pub grouping: Vec<u32>,
}

/// Stable issue id for an asset. The same asset keeps this id across runs.
pub fn stable_issue_id(asset_id: &str) -> String {
    format!("issue:{asset_id}")
}

/// Identity of one proposal. Tile order is significant. An empty list is not a grouping.
pub fn candidate_fingerprint(asset_id: &str, grouping: &[u32]) -> String {
    let mut text = format!("cand:{asset_id}|");
    for (index, tile) in grouping.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        text.push_str(&format!("{tile:02x}"));
    }
    text
}

/// One decision record. It does not write a sprite.
pub fn decide(issue: &Issue, kind: DecisionKind) -> Decision {
    Decision {
        issue_id: issue.id.clone(),
        asset_id: issue.asset_id.clone(),
        fingerprint: candidate_fingerprint(&issue.asset_id, &issue.suggestion.grouping),
        kind,
        grouping: issue.suggestion.grouping.clone(),
    }
}

/// Latest decision for this proposal, if one was stored.
pub fn latest_kind(decisions: &[Decision], issue: &Issue) -> Option<DecisionKind> {
    decisions
        .iter()
        .rev()
        .find(|decision| same_candidate(decision, issue))
        .map(|decision| decision.kind)
}

fn same_candidate(decision: &Decision, issue: &Issue) -> bool {
    let expected = candidate_fingerprint(&issue.asset_id, &issue.suggestion.grouping);
    if !decision.fingerprint.is_empty() {
        decision.fingerprint == expected
    } else {
        decision.asset_id == issue.asset_id && decision.grouping == issue.suggestion.grouping
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DecisionLog {
    decisions: Vec<Decision>,
}

impl DecisionLog {
    pub fn from_records(decisions: &[Decision]) -> Self {
        Self {
            decisions: decisions.to_vec(),
        }
    }

    pub fn record(&mut self, issue: &Issue, kind: DecisionKind) -> Decision {
        let decision = decide(issue, kind);
        self.decisions.push(decision.clone());
        decision
    }

    pub fn decisions(&self) -> &[Decision] {
        &self.decisions
    }

    /// A rejected non-empty grouping is not offered again when the tile tuple
    /// is unchanged. Order matters, matching `reinspect`. Skip and accept do
    /// not hide a later look at the same tiles. An empty grouping is not a
    /// tile proposal, so rejecting it does not block a later real grouping.
    pub fn propose<'a>(&self, issue: &'a Issue) -> Option<&'a Suggestion> {
        let grouping = &issue.suggestion.grouping;
        if !grouping.is_empty()
            && self.decisions.iter().any(|decision| {
                decision.kind == DecisionKind::Reject && same_candidate(decision, issue)
            })
        {
            return None;
        }
        Some(&issue.suggestion)
    }
}

/// One unresolved asset becomes one issue. No tile indexes are invented.
pub fn suggest_unresolved(
    id: &str,
    asset_id: &str,
    name: &str,
    tiles: &[u32],
    evidence_ids: &[String],
) -> Issue {
    let (question, summary, grouping) = if tiles.is_empty() {
        (
            format!("{name} has no documented tile mapping in this profile."),
            format!(
                "Suggestion, not a verified finding: leave {name} unresolved. No tiles were invented."
            ),
            Vec::new(),
        )
    } else {
        (
            format!(
                "{name} has proposed tile indexes only. Those indexes are not an assembled sprite."
            ),
            format!(
                "Suggestion, not a verified finding: review the proposed grouping for {name} before any save."
            ),
            tiles.to_vec(),
        )
    };
    Issue {
        id: id.to_string(),
        asset_id: asset_id.to_string(),
        question,
        evidence_ids: evidence_ids.to_vec(),
        suggestion: Suggestion { summary, grouping },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mario() -> Issue {
        suggest_unresolved(
            "issue-mario",
            "mario",
            "Mario",
            &[],
            &["coverage:mario".into()],
        )
    }

    fn proposed(tiles: &[u32]) -> Issue {
        suggest_unresolved(
            "issue-koopa",
            "koopa",
            "Koopa",
            tiles,
            &["candidate:koopa".into()],
        )
    }

    #[test]
    fn an_unresolved_name_does_not_invent_tiles() {
        let issue = mario();
        assert!(issue.suggestion.grouping.is_empty());
        assert!(!issue.suggestion.is_verified_finding());
        assert!(issue.suggestion.summary.contains("not a verified finding"));
        assert!(issue.question.contains("no documented tile mapping"));
        assert_eq!(issue.evidence_ids, vec!["coverage:mario".to_string()]);
    }

    #[test]
    fn a_proposed_grouping_stays_a_suggestion() {
        let issue = proposed(&[0x70, 0x71]);
        assert_eq!(issue.suggestion.grouping, vec![0x70, 0x71]);
        assert!(!issue.suggestion.is_verified_finding());
        assert!(issue.question.contains("not an assembled sprite"));
    }

    #[test]
    fn accept_skip_and_reject_stay_distinct() {
        let issue = proposed(&[0xA0]);
        let mut log = DecisionLog::default();
        assert_eq!(
            log.record(&issue, DecisionKind::Accept).kind,
            DecisionKind::Accept
        );
        assert_eq!(
            log.record(&issue, DecisionKind::Skip).kind,
            DecisionKind::Skip
        );
        assert_eq!(
            log.record(&issue, DecisionKind::Reject).kind,
            DecisionKind::Reject
        );
        assert_eq!(log.decisions().len(), 3);
        assert!(log
            .decisions()
            .iter()
            .all(|decision| decision.grouping == vec![0xA0]));
    }

    #[test]
    fn a_rejected_grouping_is_not_offered_unchanged() {
        let issue = proposed(&[0x70, 0x71]);
        let mut log = DecisionLog::default();
        log.record(&issue, DecisionKind::Reject);
        assert!(log.propose(&issue).is_none());
        let reordered = proposed(&[0x71, 0x70]);
        assert_eq!(
            log.propose(&reordered)
                .map(|suggestion| &suggestion.grouping),
            Some(&vec![0x71, 0x70])
        );
        let other = proposed(&[0x70, 0x72]);
        assert!(log.propose(&other).is_some());
    }

    #[test]
    fn skip_and_accept_do_not_hide_the_same_grouping() {
        let issue = proposed(&[0x10, 0x11]);
        let mut log = DecisionLog::default();
        log.record(&issue, DecisionKind::Skip);
        log.record(&issue, DecisionKind::Accept);
        assert!(log.propose(&issue).is_some());
    }

    #[test]
    fn rejecting_an_empty_suggestion_does_not_block_a_later_grouping() {
        let empty = mario();
        let mut log = DecisionLog::default();
        log.record(&empty, DecisionKind::Reject);
        assert!(log.propose(&empty).is_some());
        let later = suggest_unresolved("issue-mario-2", "mario", "Mario", &[0x3A], &[]);
        assert!(log.propose(&later).is_some());
    }

    #[test]
    fn fingerprint_is_the_same_on_every_run() {
        let first = candidate_fingerprint("koopa", &[0x70, 0x71]);
        let second = candidate_fingerprint("koopa", &[0x70, 0x71]);
        assert_eq!(first, second);
        assert_eq!(first, "cand:koopa|70,71");
        assert_ne!(first, candidate_fingerprint("koopa", &[0x71, 0x70]));
        assert_eq!(stable_issue_id("koopa"), "issue:koopa");
        assert!(!first.contains('/'));
        assert!(!first.contains(".nes"));
    }

    #[test]
    fn legacy_project_json_loads_without_decisions() {
        let project: crate::project::Project = serde_json::from_str(r#"{"name":"Old"}"#).unwrap();
        assert!(project.issue_decisions.is_empty());
        assert!(project.sprites.is_empty());
        assert!(project.blueprints.is_empty());
        assert!(project.transitions.is_empty());
        assert!(project.levels.is_empty());
    }

    #[test]
    fn a_new_project_serializes_an_empty_decision_list_by_omitting_it() {
        let project = crate::project::Project::blank("Empty");
        let text = serde_json::to_string(&project).unwrap();
        assert!(!text.contains("issue_decisions"));
        let loaded: crate::project::Project = serde_json::from_str(&text).unwrap();
        assert!(loaded.issue_decisions.is_empty());
        assert_eq!(rest_of(project), rest_of(loaded));
    }

    #[test]
    fn accept_skip_and_reject_survive_save_and_migration() {
        let mut project = crate::project::Project::blank("Decisions");
        let accepted = suggest_unresolved(
            &stable_issue_id("koopa"),
            "koopa",
            "Koopa",
            &[0x10, 0x11],
            &[],
        );
        let skipped = suggest_unresolved(&stable_issue_id("mario"), "mario", "Mario", &[], &[]);
        let rejected =
            suggest_unresolved(&stable_issue_id("plant"), "plant", "Piranha", &[0x20], &[]);
        project
            .issue_decisions
            .push(decide(&accepted, DecisionKind::Accept));
        project
            .issue_decisions
            .push(decide(&skipped, DecisionKind::Skip));
        project
            .issue_decisions
            .push(decide(&rejected, DecisionKind::Reject));
        let bare = crate::project::Project::blank("Decisions");
        let text = serde_json::to_string(&project).unwrap();
        assert!(!text.contains(".nes"));
        assert!(!text.contains("sk-"));
        let loaded = crate::blueprint::load_project_json(&text).unwrap();
        let loaded_bare =
            crate::blueprint::load_project_json(&serde_json::to_string(&bare).unwrap()).unwrap();
        assert_eq!(loaded.issue_decisions.len(), 3);
        assert_eq!(loaded.issue_decisions[0].kind, DecisionKind::Accept);
        assert_eq!(loaded.issue_decisions[1].kind, DecisionKind::Skip);
        assert_eq!(loaded.issue_decisions[2].kind, DecisionKind::Reject);
        assert_eq!(loaded.issue_decisions, project.issue_decisions);
        assert_eq!(rest_of(loaded.clone()), rest_of(loaded_bare));
        let log = DecisionLog::from_records(&loaded.issue_decisions);
        assert!(log.propose(&rejected).is_none());
        assert!(log.propose(&accepted).is_some());
        assert!(log.propose(&skipped).is_some());
        let revised =
            suggest_unresolved(&stable_issue_id("plant"), "plant", "Piranha", &[0x21], &[]);
        assert!(log.propose(&revised).is_some());
        assert_eq!(
            latest_kind(&loaded.issue_decisions, &accepted),
            Some(DecisionKind::Accept)
        );
        assert_eq!(
            latest_kind(&loaded.issue_decisions, &skipped),
            Some(DecisionKind::Skip)
        );
        assert!(!accepted.suggestion.is_verified_finding());
    }

    fn rest_of(project: crate::project::Project) -> serde_json::Value {
        let mut value = serde_json::to_value(project).unwrap();
        value.as_object_mut().unwrap().remove("issue_decisions");
        value
    }
}
