---
id: SR-670
title: "QSL-266 code and Rust review of per-occurrence requirement records"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ba8b2f82ae99bb898bb6b33d6b1dd9b8bdfb708d; qsl-semantics/src/check/claims.rs; qsl-semantics/src/check/claims/tests.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/family.rs; qsl-semantics/src/check/ir.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/refusal.rs; qsl-semantics/src/family/contract.rs; qsl-semantics/src/family/mod.rs; qsl-semantics/src/family/requirements.rs; qsl-route/src/request.rs; qsl-route/Cargo.toml; qsl-package/src/emit/extent_agreement.rs; qsl-eval/src/value/expression/family.rs; tests/it/request_builder.rs; tests/it/family_outcome_layering.rs; tests/it/main.rs; Cargo.lock"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: reviews
---

## Summary

Ticket: QSL-266 (PR quire-spec-language#459). `/code-review` with the
`/rust-review` lane over `git diff origin/main...HEAD` at `ba8b2f82`.

The PR replaces identity-based keying of one optional `Requirements` per
declaration with one `value-validity` claim per scalar operation
application. `check` finds each site on the checked tree (`claims_of`),
and `key_claims` pairs it after lowering with the `expression` occurrence
at the site's own `Location`.

What was checked and found sound:

- **Claim-site discovery.** `is_scalar_application` (checked side) and
  `is_scalar_identity` (lowered side) were compared arm by arm against
  every scalar-family identity `lowering.rs` emits. They agree: `Coerce`
  builds only `narrow`, `ConvertScalar` builds nothing for a same-type
  target (both sides now share `scalar_conversion_target`), and
  `Equality` is scalar exactly where `Lowering::equality` picks a scalar
  family. `quire.op.collection.sum.integer` and `count` are outside the
  scalar families, as required.
- **Guards.** An `if` pushes a guard for `then` (true) and `otherwise`
  (false). `and` and `implies` push their left operand as true, and `or`
  as false. Each guard links to its enclosing guard, and `claim` reverses
  the chain so the outermost guard comes first. The condition and left
  operand themselves carry no guard, which is correct.
- **Result-bound narrow rule.** `Coerce` is only ever built at its
  operand's `Location` (`check.rs:839-845`), and only when the interval
  does not contain the operand's type. So the narrow's occurrence is at
  the application's own location, and `key_claims` finds it there,
  applied to a `Reference` to the application.
- **`let` resolution.** Checker slots come from a per-body monotonic
  counter (`Typer::bind`), so a slot-keyed map cannot confuse two
  bindings. A `let` read expands to its bound value's roots, and this is
  transitive through nested `let`s.
- **Binder sites.** Parameter, query (and `count`/`sum`), `flat_map`
  (bound at the `Flatten` location), and `fold`/`reduce` (accumulator and
  element) sites are the same on both sides: `claims_of`'s
  `Frame::binding` and `Lowering::bind` / `record_binder`.
- **Keying.** Each site must match exactly one scalar occurrence at its
  location. A second claim at the same key faults. Every scalar
  `expression` occurrence under a `Body` origin must be claimed. Nothing
  is dropped silently. The measure's application is at a non-`Body`
  origin and is correctly excluded.
- **Panic surface.** There is no `unwrap`, `expect` or indexing panic in
  non-test code. Invariant breaks return `InternalFault` (mapped to
  `KeyFault::UnclassifiedExtent`) or `KeyFault::UnkeyableRequirements`.
- **Dead code.** `key_requirements`, `requirements_per_index` and the
  old `key_requirements_tests` are deleted, and no Rust reference to them
  remains. One prose leftover is FND-006.
- **ADR-011 layering.** `qsl-route` still depends only on
  `qsl-foundation`, `qsl-semantics` and `thiserror`. The `quire-exact`
  dev-dependency is leader-accepted, and `family_outcome_layering`
  allows it explicitly.

## Verdict

**CONDITIONAL.** No correctness defect was found in the claim-site walk
or in keying. One medium finding: the false-outcome and multi-guard paths
of the path condition have no test. The low findings are idiom and doc
issues.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test drives a guard with `holds == false` (an `if`'s `otherwise`, or `or`'s right operand), `implies`, or a path condition of two or more guards. Flipping `index == 1` to `index == 2`, treating `Or` as true, or deleting `path_condition.reverse()` would still pass every test. RR-11, RR-12, RR-13 and RR-16 each exercise one guard, always true. | qsl-semantics/src/check/claims.rs:414-420; qsl-semantics/src/check/claims.rs:522; qsl-semantics/src/check/claims/tests.rs:224-465 |
| FND-002 | low | A binder read inside a nested query, fold or `let` body becomes an extent root of an application outside that binder's scope. `reads` propagate to every ancestor with no scope filter. Example: `size(filter(v in s: v > 0)) + 1` makes `v` a root of the outer `+`, beside `s`. This matches FR-062's literal "read anywhere in the application's argument subtrees". But it can double-count one unbounded position (`s`'s element and `v`) and may disagree with IR's per-node predicate. The spec should state which reading is intended. | qsl-semantics/src/check/claims.rs:484-489 |
| FND-003 | low | `RequestWriter::item` and `bounded_item` take both `occurrence` and `node`, and `RequestItem` stores both. A caller can pass an occurrence of record A with node B, and the item then joins A's record while claiming over B's node. `RequirementItem::node` already derives the node from `occurrence.node()`, and `RequestItem` should do the same. | qsl-route/src/request.rs:212-240 |
| FND-004 | low | `claim` handles a missing guard index with `let Some(guard) = guards.get(index) else { break; }`. That silently truncates the path condition on a broken invariant, where every other invariant in the module faults. Idiom: `ClaimSite` holds a `narrowed: bool` beside `result_bound`, where an `Option<IntegerInterval>` would make a narrowed non-`Int` bound unrepresentable. `claim` needs `#[allow(clippy::too_many_arguments)]` for walk state that could be one struct. | qsl-semantics/src/check/claims.rs:497-514; qsl-semantics/src/check/claims.rs:113-118 |
| FND-005 | low | The extents of `fold`, `reduce` and `flat_map` binder roots are never asserted. The lowering row tests (`lowering/tests/rows.rs`, `binder_scope.rs`) only show that a `fold` body with `acc + x` keys without a fault. RR-14, the leader-decided fix, covers the query binder only. | qsl-semantics/src/check/claims.rs:424-438; qsl-semantics/src/check/claims/tests.rs:224 |
| FND-006 | low | Stale prose from the old shape: `docs/family-migration-recipe.md` still describes `requirements()` as returning "zero or one `Requirements` value", but it is now `Vec<Self::Claim>`. | docs/family-migration-recipe.md:61-62 |

## Rust review

- Errors: `ClassifyFailure::{Limit, Fault}` maps to the declaration's
  `StageFailure::Limit` or to `KeyFault::UnclassifiedExtent(InternalFault)`,
  whose `invariant()` returns the inner stable identifier. Keying failures
  are one `KeyFault::UnkeyableRequirements`.
- Matching: `is_scalar_application` is exhaustive under
  `#[deny(clippy::wildcard_enum_match_arm)]`, including the `seam_probe`
  arm. `scalar_equality` matches every `ValueType` explicitly.
- Resource bounds: the claim walk uses an explicit stack. Classification
  is capped per claim at the declaration's node-count limit (TC-160
  `a_classification_past_the_node_count_limit_is_a_stage_limit`). The
  claim count is bounded by body size, so total work is at most
  quadratic in `node_count`. That is acceptable here.
- Allocation: each frame clones `BTreeSet<BinderSite>` of reads into its
  parent. This is linear in body size times binders, which is fine at
  current sizes.
- Tests: the fixtures in `claims/tests.rs` take expected values from
  FR-062's table (hard-coded identities, ordinals, bound types and
  roots). They find occurrences by source span and type nodes from the
  function node, independently of the record under test. This is a
  strong oracle. `tc_449_*` compares item fields with record fields, which
  is the mapping under test, plus independent `Bounded`/`Unbounded` and
  count assertions.
- TC-440: the ignored
  `tc_440_an_unbounded_application_record_requires_a_bound_in_ir_pending_ir_283`
  was run with `--ignored`. It fails on its one assertion because IR
  returns `Lowered` for the outer `+`, which is the IR-283 behaviour its
  reason states. The ignore is honest.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 44750e77: `tc_160_guards_carry_their_outcome_outermost_first` covers an `if`'s `otherwise` guard (false), `or` (false), `implies` (true) and two nested guards outermost first. Each of these mutants fails it: swapping `then`/`otherwise`, treating `or` as true, deleting `path_condition.reverse()`. |
| FND-002 | fixed | e0d67230, 44750e77, 0872c86a: per the leader's ruling, binder reads are scoped. A node's roots exclude binders bound inside it (`bound_by`), so the outer `+` of `size(filter(v in s: v > 0)) + 1` is rooted at `s` only (`tc_160_a_binder_is_a_root_only_inside_its_scope`, which fails without the scoping). FR-062 and ADR-014 §4 state the rule. |
| FND-003 | fixed | 29ecb52a: `RequestWriter::item(occurrence, requirements)` and `bounded_item(occurrence, requirements, bounds)`. `RequestItem::node` is `occurrence.node()`. FR-075's design-level signature matches (0872c86a). |
| FND-004 | fixed | e0d67230: a missing guard index is an `InternalFault` (`guard-in-arena`); `ClaimSite` holds `SiteBound::{Narrowed(IntegerInterval), Own(ValueType)}`; the walk state is grouped in `Classify`, and the `too_many_arguments` allow is gone. |
| FND-005 | fixed | 44750e77: `tc_160_fold_reduce_and_flat_map_binders_are_roots` asserts that `fold` and `reduce` steps are unbounded at the accumulator and at the element, and that a `flatMap` step is unbounded at its binders. It fails when the accumulator is not a root. |
| FND-006 | fixed | 0872c86a: `docs/family-migration-recipe.md` describes `requirements()` as one `Self::Claim` per claim site, and its `requirements()` note and migration entry are rewritten. |
