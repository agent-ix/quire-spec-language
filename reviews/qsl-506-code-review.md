---
id: SR-12089
title: "QSL-506 PR #941 code review: an absent bound reads as None (FR-333, FR-334, FR-335)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@cbbbe83f3ef4d0f4415a23b01b5978716882aeec; qsl-cst/src/grammar.rs, qsl-semantics/src/check/type_form.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/lowering/tests.rs, qsl-eval/tests/it/support/type_form.rs, qsl-eval/tests/it/unbounded_set_call.rs, qsl-eval/tests/it/state_model_family.rs, tests/it/unbounded_declarations.rs, tests/it/request_builder.rs, spec/functional/FR-333..FR-335, spec/test-cases/TC-843..TC-845"
review_set: subset
---

## Summary

Ticket: QSL-506. PR quire-spec-language#941. Code review with the rust-review lane, scoped to `git diff origin/main...HEAD` (17 files, +686/-88). Read only. No build was run, and the merged tree was read for compile and type errors. The author's recorded gap is not reported again: the FR-335-AC-5 charge-point assertion was a guess. The guess is checked below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-335-AC-5 test asserts the small-meter stop is at `CollectionBound` or `CollectionElement`, but `form_collection` on a `Set` stops at `CollectionMemberWalk`. `coalesce` charges `collection.member-walk` (work = sum of the two occurrence counts, 2 here) and `collection.member-test` (1) for each comparison before `bound_and_retain` charges `collection.bound`. `collection.element` is charged only by `construct_collection`, which this test never calls. With `work_units: 10` the totals run 3, 6, 9, and the 4th candidate's first member-walk goes to 11 and is denied, so the `matches!` fails and the test panics. The AC text names the same two charge points, which this path cannot reach for a set. The test also builds the set outside `f`, so "f returns Incomplete" and "the AC-1 item's disposition is unchanged" are never checked | qsl-eval/tests/it/unbounded_set_call.rs:106-118, quire-exact src/collection.rs:401-403, quire-exact src/collection.rs:437-498, spec/functional/FR-335-settle-a-claim-over-an-unbounded-declaration.md:101 |

## Verdict

FIX, for FND-001 only. The FR-335-AC-5 test fails as written. Either assert `ChargePoint::CollectionMemberWalk`, which is the charge point this path reaches, and amend FR-335-AC-5 to match, or build the argument with `construct_collection`, where `collection.element` is charged first. Everything else checked is sound:

- **Every absent-bound path now gives `None`.** S1 `TypeReference` now has `opt([uint, uint])` on collections. Population already had `opt([uint])`. S2 `type_head` pushes no bounds when none are written. In `resolve_head`, Population calls `optional_bounds(form, 1)` and then `Population(None|Some(N))`. In `finish` for a collection, `optional_bounds(form, 2)` gives `bound: None` or the checked `CardinalityBound`. `population_type` no longer refuses `None` and writes a `model_population` node with no `max` member, which is a distinct node from a bare `Set<Reference<T>>`. Downstream, every consumer already took the `Option`: `state_model` `AllInstances` (`None` gives an unbounded `Set`), `family` type-tag (`unbounded`), `requirements` (`DomainKind::Population`/`Collection`), S6a `resolve_population` and spine `call.rs` (`declared_maximum() == maximum`, absence included), and the replay wire `maximum.map(..)`.
- **The removed panicking `type_form` helper had one caller.** `qsl-eval/tests/it/support/type_form.rs::collection_type_form` now writes empty bounds for `None`, which is the exact inverse of the resolver. No production `.bound().expect/unwrap` remains (grep over `src/` crates).
- **Bounded values are unchanged.** `[a, b]` with `a > b` still gives `EmptyCardinality` and then `type-mismatch` at the form span. `[0, 0]` and `Population[0]` still resolve. A count other than 0 or 2 (collection) or 0 or 1 (population) is now `Malformed`, which the grammar never produces.
- **No new panics or classifier changes.** No `unwrap`, `expect` or panic was added on a production path. `CheckCause::UnrepresentableBound` keeps its other three callers and its `refusal.rs` and `spine/refinement.rs` arms. No cause was added.
- **Contract IR's reader does not enforce a `max` binding on `model_population`.** In derived_keys.rs, `Shape::of` returns `None` for that form, so a package with an unbounded-population parameter still reads back through `read_checked_package` at S5 bind.
- **The new tests should compile, as far as reading can tell.** I checked the imports and APIs used: `RequestIndex::get(self)`, `RequestItem: PartialEq + Debug`, `ProofBound::new -> Result`, `Requirements::new`, `ClaimExtent::from_domains`, `ScalarLimits` field set, `Incomplete.charge_point`, `TypeFormFault: PartialEq`, `resolve_form(&Scope)`. The `request_builder` items made `pub(super)` are reachable from the sibling module.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The test asserts `CollectionMemberWalk` for forming the set on a 10-unit meter, and `FunctionCall` for calling `f` on a meter with no work left. FR-335-AC-5 and TC-845 step 5 now say so. The "AC-1 item unchanged" clause is replaced by the statement that evaluation reads no request item. |
