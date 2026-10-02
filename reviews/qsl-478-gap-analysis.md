---
id: SR-1232
title: "QSL-478 gap analysis of PR #608 (QSL-358 slice 4; ADR-011 §6.2 as amended by AM3)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@990e58a9e2bb3f54a0ea044cf20f412771796125; PR #608 diff against origin/main; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (§6.2 module table rows, X-11); spec/functional/FR-089-carry-population-identity-across-the-kernel-boundary.md; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-109-run-a-state-clause-through-the-spine.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-478. PR: quire-spec-language#608. Manual check of the slice's
acceptance against its tests. No new tests were added; the existing ones
moved with the API.

What backs the ADR-011 §6.2 / X-11 rule (the closure is in SV, imports
nothing of `model`, and SV stays `no_std`):
- The crate graph enforces it at compile time. SV cannot name a `model`
  type. TC-390's dependency check asserts SV's exact `[dependencies]` table,
  and that table is unchanged. `make ci` builds SV for `thumbv7em-none-eabi`,
  and I repeated that build here.
- "No re-export" is checked by absence (grep at the head). Nothing in the
  suite asserts it, but X-11's rule is what forbids it.

What backs the moved behaviour:
- `Attribute` refusals (MissingField, UndeclaredField):
  type_environment_model.rs, `an_inherited_field_is_flattened_into_the_subtype`
  and `a_renamed_redefinition_replaces_the_inherited_name`.
- `DanglingReference`: qsl-eval composite_values.rs:482.
- `tolerated_dangling`: state_clauses.rs TC-465
  `a_dangling_reference_into_an_incomplete_population_still_admits`.
- `attribute` resolution through redefinition and diamonds:
  type_environment_model.rs.
- `find`: state_clauses.rs:2161 and :4057, and qsl-replay TC-468.
- `contains`: state_clauses.rs:2560.

So "no new tests" is honest for the move and for the ADR rule. Three
behaviours of the moved type have never had a test, before this PR or after
it: `DuplicateObject`, `UnknownObjectType`, and `find` returning `None` when
more than one object matches. They now form the public API of a shared leaf
that RT is meant to migrate onto (FND-001).

## Verdict

Changes requested: one medium coverage finding. The rest of the slice is
backed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three behaviours of the moved type have no test anywhere in the workspace, and no AC owns them: `ObjectClosure::new`'s `DuplicateObject` and `UnknownObjectType` refusals, and `find`'s `None` on more than one match. A grep for both causes in the qsl-* tests finds nothing. The module cites "FR-143" for the closure, but QSL's FR-143 is now the refinement-liveness requirement, so the citation points at an unrelated record. Slice 4 makes `ObjectClosure` the shared-leaf API that RT migrates onto and deletes its copy against, so this contract should be pinned where it now lives. Fix: (1) Add an SV unit test module in `object_closure.rs`, with three cases: two objects with one identity triple refuse `DuplicateObject` on the second; a reference whose object type is not a model object type refuses `UnknownObjectType`; and two admitted objects that share universe and key under different types make `find` return `None`. (2) Replace the stale FR-143 citation with the requirement that owns the closure. If none does, add an AC for these three behaviours (FR-106, whose admission builds the closure, is the natural owner) and trace the tests to it. | quire-semantic-value/src/object_closure.rs:3; quire-semantic-value/src/object_closure.rs:69-115 |
