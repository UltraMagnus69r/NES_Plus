# R3 Slice 3 — Evidence-backed issue suggestions

R3 remains incomplete. This slice fills the user-facing suggestion gap identified after Slice 2: `IssueCard.suggestion` was always `None`, and Investigate needed clearer Issue / Action / Why / Uncertainty / Decision sections.

## Architecture reused

- `Project.issue_decisions` (Slice 2) — unchanged persistence.
- `nesticles_core::issues::{Issue, Decision, DecisionLog, fingerprint, decide}` — extended `Suggestion` only.
- `open_investigation_issue` / Investigate capsule Accept·Skip·Reject — same decision path.
- Goomba Approve & Save on Review remains the only sprite writer.

## Suggestion model

`Suggestion` is regenerated from evidence. It is **not** stored on the project file.

| Field | Meaning |
| --- | --- |
| `summary` | Short honesty-tagged line |
| `grouping` | Proposed tile indexes in order (empty = no grouping) |
| `action` | Recommended next step |
| `why` | Observed evidence + derived interpretation |
| `uncertainty` | What is not verified |
| `accept_effect` | What Accept records (never fabricates sprites here) |

`suggest_issue(SuggestContext)` is deterministic: identical inputs → identical `Issue`.

## Supported categories (from present data only)

1. **No tiles** (coverage name only, e.g. Mario) — leave unresolved; do not invent indexes.
2. **Proposed grouping** (candidate with tile indexes) — review that exact order.
3. **Goomba candidate** (`character_id == "goomba"`) — confirm grouping; publish still via Review Approve & Save when the payload matches.

No AI, no network, no invented frames or offsets.

## Accept / Skip / Reject

Unchanged semantics:

- **Accept** — append decision; does not write `SpriteDef` / `AnimSet` / blueprint.
- **Skip** — append decision; grouping stays eligible.
- **Reject** — append decision; non-empty unchanged fingerprint is not re-offered; a different tile order is a new candidate.

## UI

Investigate shows Issue, Suggested action, Why, Uncertainty, Accept effect, expandable technical evidence, then Accept / Skip / Reject. Review lists the card suggestion line when present. R6.1 chrome tokens are reused; no redesign.

## Tests

- Core: determinism, goomba path, empty/proposed suggestions, reject-hide, save/reopen decisions, legacy JSON.
- UnMaker model: IssueCard suggestion filled; open-issue reject/revision boundary retained.

## Remaining R3 acceptance

Owner must still accept the milestone after exercising Investigate with a real or synthetic session. Mario/Luigi reconstruction, R4 reference AI, and history rewrite are out of scope.
