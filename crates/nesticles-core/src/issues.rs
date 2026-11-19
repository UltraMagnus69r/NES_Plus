//! R3 decision rules for one reconstruction issue.
//!
//! A suggestion is a hypothesis. It is not a verified finding and it does not
//! write a sprite. Decisions stay in this log. Choosing where they survive a
//! save is a separate project-file decision and is not done here.

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionKind {
    Accept,
    Skip,
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub issue_id: String,
    pub asset_id: String,
    pub kind: DecisionKind,
    pub grouping: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DecisionLog {
    decisions: Vec<Decision>,
}

impl DecisionLog {
    pub fn record(&mut self, issue: &Issue, kind: DecisionKind) -> Decision {
        let decision = Decision {
            issue_id: issue.id.clone(),
            asset_id: issue.asset_id.clone(),
            kind,
            grouping: issue.suggestion.grouping.clone(),
        };
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
                decision.kind == DecisionKind::Reject
                    && decision.asset_id == issue.asset_id
                    && decision.grouping == *grouping
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
}
