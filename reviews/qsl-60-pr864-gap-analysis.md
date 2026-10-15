---
id: SR-11256
title: "Gap analysis — QSL-60 qualified systems-name binder (PR 864)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@880285f273754140fe4d777a52c152d78b5b07cd; FR-151-AC-7, FR-152-AC-1, FR-152-AC-3, FR-152-AC-5, FR-152-AC-7 against qsl-semantics/src/model/qualified.rs, qsl-semantics/tests/it/model_systems_resolve.rs, qsl-semantics/tests/it/model_systems.rs, qsl-eval/tests/it/dispatch_calls.rs"
review_set: subset
---

## Summary

Ticket: QSL-60. Planless gap analysis of the PR 864 diff against QSpec FR-151-AC-7, FR-152-AC-1/3/5/7 and TC-196 D06, TC-197 Y01 and Y07-Y09. Plan completion: not assessed. Quoin matrix not run (no builds per brief); bindings read from the trace tags in the diff and on origin/main.

## Verdict

**Two blocking findings.** FR-152-AC-1 is backed: the Y01 test asserts kind, declaration key, effective id and owner for all five kinds against entries the test reads independently from the view, plus the eight `systems.kind` admission charges; the coder's mutation run (last owner instead of first) fails it. FR-152-AC-5's ModelSelection half is backed for qualified names by the foreign-alias test (refused `foreign_reference`/`foreign-model-selection`, no charge, while the same id under the owning alias resolves). FR-152-AC-3 was already tagged on origin/main for a dangling end (model_systems.rs); the new tags add refusal of unresolvable qualified names; the ambiguous-effective-target half (Y08) is honestly reported blocked. FR-152-AC-7 is untagged and reported blocked. FR-151-AC-7: the "no change needed" claim is true; origin/main already carries D06 tests in qsl-eval/tests/it/dispatch_calls.rs (both-false precondition is `undefined`/`precondition-false` naming `model.B.size`, never `A/size`) and in checked_dispatch.rs; the ticket text predates them.

Recorded, not blocking: `a_denied_systems_resolve_charge_is_incomplete` carries FR-152-AC-1 but asserts only the charge-before-resolve order (AC-1 is backed by Y01, so the matrix is unchanged); this repo's FR-086 Dependencies still says systems records receive no effective-declaration identity and that FR-152-AC-1 is unbacked under #147, and FR-085:33 and intake.rs:5686 carry the same stale statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `an_unresolvable_name_refuses_without_ambient_lookup` (tagged FR-152-AC-3) claims the mutation "falling back to any entry whose original matches regardless of owner resolves `pump_alloc` through `Sys`'s entry and fails the refusal". It does not: the third case builds the view from Y without `pump_alloc`, so the view holds NO `pump_alloc` entry under any owner, and that fallback also finds nothing and still refuses. No test has a view holding only another owner's entry, so the "without ambient lookup" property this test is tagged for survives the named mutation. Fix: view a package where `pump_alloc`'s only entry is under `Sys` (e.g. viewed package with the allocation's source moved to a `Sys`-owned element) and assert the refusal. | qsl-semantics/tests/it/model_systems_resolve.rs:296-337 |
| FND-002 | high | `a_name_resolving_to_another_kind_is_wrong_export` is tagged FR-152-AC-5 but asserts only AC-2 content (a Part requested as a Port and a Connection as an Allocation refuse `wrong-export`). AC-5's display-name half needs two declarations sharing a display name; none is built, and the half is reported blocked. This is the same tag PR #144's review dropped from model_systems.rs for the same reason. Fix: drop FR-152-AC-5 from this test (keep AC-2). | qsl-semantics/tests/it/model_systems_resolve.rs:409-424 |
