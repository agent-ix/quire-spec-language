---
id: SR-12822
title: "base review of STD-169 FR-214 and TC-659 step-row forms"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4ca372951028b143e8da7850d0645f7a34b48269; spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, spec/test-cases/TC-659-protocol-refinement-step-rows.md"
review_set: subset
---

## Summary

Ticket: STD-169 (PR agent-ix/quire-spec-language#977). The PR rewrites the
step rows quoted in FR-214-AC-1, AC-2 and AC-3 and in TC-659's procedure into
forms that QSpec's shared grammar accepts. Every rewritten row was checked
against QSpec `protocol-step-row` / `protocol-step-source` /
`protocol-step-target` (shared-grammar.md:544-551), FR-375's `step-row` /
`step-target` (`operation-name '(' expr {',' step-argument} ')'`, with
`operation-name = ident '::' member '::' member`), FR-432's left-side table,
QSL FR-136, ADR-020 line 133, ADR-027 PR-1, and the CST productions in
PR #974 (`StepSource`, `StepTarget`) and PR #970 (operation rows resolved as
`<concrete alias>::<Type>::<op>`). All five row shapes parse and resolve as
written. Behaviour and expected results are unchanged: the same five step
classes of ADR-027 §7's `Fill` (two operations, fork, join, finish) are
covered, and the refusals of AC-2/AC-3 and TC-659's Expected Results are
untouched.

## Verdict

PASS with one medium and two low findings; none is a merge blocker.
Row-by-row: `step M::Cell::inc -> Abs::Acc::add(self)` and the `setTwo` row
match FR-375 (`add()` has no parameter, so the receiver alone is the right
count) and the ADR-020 `Spec::Counter::inc(self)` precedent;
`step fork|join M::Fill::Both -> stutter` puts the kind word first as
`protocol-step-source` requires; `step M::Fill::Done -> stutter` and
`step M::Fill::A -> stutter` are node rows of FR-432's
`<C>::<protocol>::<node>` form. Rows are quoted without a terminating `;`,
matching the house style of FR-136's ACs. Ids, relationships and titles are
unchanged, no index or matrix carries AC text, and `quire validate` passes on
both files, so a `/quoin:specify` run would have added nothing. Open point 2
(placeholder names `Abs`/`Acc`): acceptable, the AC now declares them inline;
see FND-001 for what the naming exposes. Open point 3 (Refund `cend` and
Chatter `ch` in prose): acceptable for `cend` (one class, one row); see
FND-002 for `ch`.

## Examined Scope

```yaml
scope:
  - {id: FR-214-AC-1, path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, role: examined, excerpt: "A refinement of an abstract model `Abs`, whose object type `Acc` has operation `add()`, by ADR-027 §7's `Fill` over model `M`, with rows `step M::Cell::inc -> Abs::Acc::add(self)`, `step M::Cell::setTwo -> Abs::Acc::add(self)`, `step fork M::Fill::Both -> stutter`, `step join M::Fill::Both -> stutter` and `step M::Fill::Done -> stutter`, checks; every concrete step class has exactly one row, the attempts are `Visible` with `add`'s label and the control steps `Internal`."}
  - {id: FR-214-AC-2, path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, role: examined, excerpt: "Removing the `join` row refuses `missing_declaration`/`missing-name` naming `Both`'s join; removing the `finish` row refuses naming `Done`. Adding a second `step M::Cell::inc -> …` row refuses `invalid_model_binding`/`conflicting-binding`."}
  - {id: FR-214-AC-3, path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, role: examined, excerpt: "Adding `step M::Fill::A -> stutter` beside `step M::Cell::inc -> Abs::Acc::add(self)` maps `A`'s steps by the node row (`Internal`) and leaves the `inc` operation row unused by `A`. Over ADR-027 §7.1, omitting the `Refund` template's `cend` row refuses naming the template; over `Chatter` (FR-212), omitting the channel row refuses naming `ch`."}
  - {id: TC-659, path: spec/test-cases/TC-659-protocol-refinement-step-rows.md, role: examined, excerpt: "2. Remove the `step join M::Fill::Both` row, then the `step M::Fill::Done` row; add a second `step M::Cell::inc` row. 3. Add the node row `step M::Fill::A -> stutter`; check ADR-027 §7.1 without the `cend` row and `Chatter` without the channel row."}
  - {id: FR-214-AC-4, path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md, role: context_only, excerpt: "A row `-> any` gives `Either`. An abstract protocol whose profile lists `fork` as internal gives that abstract step internal, and its other abstract steps visible."}
  - {id: FR-136, path: spec/functional/FR-136-check-a-refinement-s-step-rows.md, role: context_only, excerpt: "The receiver SHALL conform to a reference to the abstract operation's receiver type after reference lifting ... each channel whose delivery policy admits `duplicate` or `lose` SHALL have a channel row for each step kind it admits"}
  - {id: FR-135, path: spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, role: context_only, excerpt: "For each population of the abstract model, the checker SHALL require exactly one `population` row ... For each object type that an abstract population admits, the checker SHALL require exactly one `object` row"}
```

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-214-AC-1 says the refinement "with rows" (five step rows) "checks", but now names an abstract model `Abs` with object type `Acc` and a receiver `self` typed against `Acc`. FR-135 refuses `missing_declaration`/`missing-name` for an abstract population with no `population` row and an admitted abstract type with no `object` row, and FR-136 types the receiver `self` (a `M::Cell`) against `Abs::Acc` only through reference lifting, which needs `object Abs::Acc from M::Cell`. A test author who writes exactly the listed rows gets a refusal, not "checks". Add one clause, e.g. "with a population row for `Abs`'s population from `M::cells` and an object row mapping `Acc` from `Cell`". | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md:76 | wrong-requirement |
| FND-002 | low | FR-214-AC-3's "omitting the channel row refuses naming `ch`" is singular, but FR-212's `Chatter` gives `ch` delivery `unknown`, so FR-136 requires two channel rows (`step duplicate M::Chatter::ch` and `step lose M::Chatter::ch`). Which row is omitted is unstated; either refuses naming `ch`, so the expected result holds, but the vector is underdetermined. TC-659 step 3 inherits it. | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md:78; spec/test-cases/TC-659-protocol-refinement-step-rows.md:23-24 | wrong-requirement |
| FND-003 | low | TC-659's oracle does not require the rows to be written as source and parsed, unlike QSpec TC-386 ("write the refinement declaration as each vector states and check it through the real local Rust boundary"). A test that builds `StepRow` forms directly would pass with a grammar-nonconformant row, which is how the old `step inc -> add()` and `Fill::Both.fork` rows survived until PR #974's CST review caught them. Add "written as source and checked from S2" to the procedure. | spec/test-cases/TC-659-protocol-refinement-step-rows.md:17-27 | correct-requirement-no-evidence |
