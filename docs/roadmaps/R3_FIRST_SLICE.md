# R3 issue decisions

R3's finished condition is still the roadmap sentence: one unresolved issue is presented with a suggestion, accept / skip / reject are stored, and a rejected grouping is not offered again unchanged. R3 is not complete.

## Slice 2 — persisted on the project

The approved store is `Project.issue_decisions`, a serde-default list of the existing `Decision` record. Old project JSON omits the field and loads an empty list. An empty list is omitted on save.

Accept, skip, and reject append a record. They do not write `SpriteDef`, `AnimSet`, or a blueprint. Accept does not mean the suggestion was verified. Skip leaves the issue eligible. Reject hides that exact tile order. A different order is a different candidate.

The fingerprint is `cand:{asset_id}|{tile hex in proposal order}`. It is not a process id and it does not store a ROM path. The issue id is `issue:{asset_id}`.

Investigate shows the first eligible issue from the current session, using the glossy capsule buttons. A name that `assemble` already accepted is not offered again. If nothing is eligible, the screen says there is no open issue.

## Slice 1 — in-memory rules

The text below is the first slice. The store choice it left open was decided in slice 2.

## Dependency

`ExtractionSession` is still process memory. `Attempt` and `Recipe.rejected` already remember tile tuples for a recipe, and `reinspect` already refuses an unchanged tuple on a failed candidate. Neither store is a general issue decision, and neither is in the project file. Skip has no field. A name with no tiles, such as Mario, has no recipe.

The project file is the lifecycle the audit chose. Slice 1 did not add the field. Slice 2 did, as `issue_decisions`.

Two stores are available for the next step:

1. A serde-default list on `Project`. Old projects open unchanged. This matches the lifecycle decision and keeps learning next to the creation.
2. The existing knowledge recipe file. It already has `rejected` tile lists, but it cannot represent skip, and it cannot represent an asset that has no tiles.

Slice 2 picked the project list and shows one issue in Investigate. The knowledge file was not used.

## What this slice does

`nesticles_core::issues` builds one issue from a name and an optional tile tuple.

- The suggestion text says it is not a verified finding.
- An empty tile list does not invent indexes.
- A non-empty list is kept in proposal order and is still only a suggestion.
- Accept, skip, and reject are stored as different decisions in `DecisionLog`.
- Reject of a non-empty tuple hides that same tuple for that asset. A different order is a different grouping, as in `reinspect`.
- Skip and accept do not hide the grouping.
- Reject of an empty suggestion does not block a later real grouping.
- Accept does not write `SpriteDef` or `AnimSet`. The Goomba approve path stays the only writer for that sprite.

`IssueCard.suggestion` is still `None`. Slice 2 shows one `issues::Issue` on Investigate instead of changing that card.

## Tests

`issues::tests` in `crates/nesticles-core/src/issues.rs`.

## Acceptance for this slice

- The six unit tests pass.
- `Project` JSON gains no field.
- No ROM is read and no network call is made.
- The R6.1 chrome is unchanged.

## Not this slice

Mario or Koopa reconstruction, reference images, AI, animation editing, blueprint attachment, Maker handoff, and historical Phase 10.
