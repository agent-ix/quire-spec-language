---
id: SR-16217
title: "spec-review of a refinement's refusals when the unit stops before its clauses (quire-spec-language PR 1315)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c28bc3d7; spec/functional/FR-136-check-a-refinement-s-step-rows.md; spec/test-cases/TC-541-s3-checks-a-refinement-s-step-rows.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: references
---

# Spec review of a refinement's refusals when the unit stops before its clauses (quire-spec-language PR 1315)

## Summary

Ticket: STD-364 (spec gap behind QSL-954). One spec-review pass over
`git diff origin/main...c28bc3d7` (2 files, +26/-1). origin/main is an
ancestor of the head. Oracles: FR-135 (population and object maps), FR-147
(`CasTwice`/`Twice` fixture), ADR-020 §8, FR-136-AC-4/AC-6, and the S3 gate
in `qsl-semantics/src/check/mod.rs` on main (the `if !refusals.is_empty()`
return after the model-side refinements).

Gates run by the reviewer: `make check-spec-validation` exits 0.

Examined, clean:

- The rule separates the clause-free checks (header except abstract-protocol
  resolution, population/object/fairness/history rows, model step rows,
  classification and selection) from the clause-dependent ones (abstract
  header resolution, protocol-subject coverage), which are skipped without a
  refusal. This is consistent with FR-136-AC-8 and with "Order of the
  checks". Skipping clause-dependent checks when no checked clause exists is
  no narrowing: the unit is refused either way, and the full set of
  refusals is reported once the stopping refusal is fixed (the AC's second
  variant).
- AC-11's first-variant refusal values: `non-boolean-root` for the
  invariant, `missing-name` naming `Spec::counters` (FR-135 population map,
  ADR-020 §8 `population Spec::counters from Impl::counters`), and
  `conflicting-binding` at `Other`'s first row (FR-136-AC-6).
- Ids: FR-136-AC-11 and TC-541 step 11 are unused on main and in every open
  QSL PR.
- Behaviour-only wording; lead defaults labelled. The PR body's "Code
  follow-up (QSL-954)" is the implementing ticket, not a spec deferral.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [ambiguous] The bullet does not say which refusals stop the unit before its protocol clauses ("a refusal elsewhere in the unit"), or whether a refinement's own clause-free refusals count. AC-11 depends on the answer. In its second variant, `CasTwice`'s `missing-name` must not stop the clause check, since `Run`'s `fork` coverage refusal is still reported. In its first variant, `Run`'s `conflicting-binding` comes after `CasTwice`'s. On main, `Run`, whose abstract side is a model, is checked before the gate, and its refusals are among those that stop the unit. Under "These refusals SHALL follow the refusals that stopped the unit", that puts `Run` before `CasTwice`. Fix: say that a refinement's clause-free refusals never stop the unit before its protocol clauses, name what does stop it (a refusal of a non-refinement declaration, or an S3 stage that refuses the unit), and say that every refinement's clause-free refusals, model-side ones included, follow those in source order. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:177-189, 273 | wrong-requirement |
| FND-002 | low | [untestable-ac] AC-11's "exactly" three refusals assumes that `CasTwice`'s object row `Spec::Counter from Impl::Counter` still checks once its `population` row is removed. FR-135's object map needs the `from` type to be "an object type of the row's source population", and with no population row there is no source population. That could give a second refusal (`malformed-declaration`), and no rule says it does not. Fix: say in FR-135 (or in AC-11) that an object row whose abstract population has no row is not checked against a source population, or keep the population row and remove another clause-free row. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:273; spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:127-135 | missing-requirement |

## Verdict

Not mergeable yet: one medium finding. The split between clause-free and
clause-dependent checks is sound and narrows nothing. FND-001 needs one
sentence on which refusals stop the unit, so that AC-11's order follows
from the FR text and not only from the vector.

## Dispositions

Round 1. `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` exits 0, and origin/main is an ancestor of the head.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | A new bullet names what stops the unit before its protocol clauses: a refused declaration that is not a refinement (function body, state clause, repeated protocol reference), or a whole-unit S3 stage such as model conformance. A refinement's own refusals never stop it. The refinements' refusals, of every subject kind, follow in source order, so AC-11's order invariant, `CasTwice`, `Run` follows from the text. This changes main, where `Run`'s refusals currently feed the gate; QSL-954's code change carries that. |
| FND-002 | fixed | AC-11 and TC-541 step 11 now write `CasTwice`'s `population` row twice, which is FR-135-AC-2's single `conflicting-binding`, instead of removing it. The source population therefore exists and the object row checks. |
