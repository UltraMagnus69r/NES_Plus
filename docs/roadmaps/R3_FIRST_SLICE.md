# R3 first slice — one issue, three decisions

R3's finished condition is still the roadmap sentence: one unresolved issue is presented with a suggestion, accept / skip / reject are stored, and a rejected grouping is not offered again unchanged.

## Dependency

`ExtractionSession` is still process memory. `Attempt` and `Recipe.rejected` already remember tile tuples for a recipe, and `reinspect` already refuses an unchanged tuple on a failed candidate. Neither store is a general issue decision, and neither is in the project file. Skip has no field. A name with no tiles, such as Mario, has no recipe.

The project file is the lifecycle the audit chose. Adding decisions there changes a persistent contract. This slice does not add that field.

Two stores are available for the next step:

1. A serde-default list on `Project`. Old projects open unchanged. This matches the lifecycle decision and keeps learning next to the creation.
2. The existing knowledge recipe file. It already has `rejected` tile lists, but it cannot represent skip, and it cannot represent an asset that has no tiles.

The slice does not pick one. Wiring either store, or showing the issue in Investigate, is the next R3 change.

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

The Investigate screen still builds `IssueCard` values with `suggestion: None`. This slice does not restyle that screen.

## Tests

`issues::tests` in `crates/nesticles-core/src/issues.rs`.

## Acceptance for this slice

- The six unit tests pass.
- `Project` JSON gains no field.
- No ROM is read and no network call is made.
- The R6.1 chrome is unchanged.

## Not this slice

Mario or Koopa reconstruction, reference images, AI, animation editing, blueprint attachment, Maker handoff, and historical Phase 10.
