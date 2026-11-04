---
id: SR-20261104115609
title: "QSL-1613 gap analysis of PR #2287"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2287 head 170e02f8b; FR-480 Selections and imports, FR-480-AC-9, FR-480-AC-11, FR-480-AC-12; qsl-semantics/src/check/{model_spelling.rs,assemble.rs,check.rs}; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/tests/it/{variant_fields,model_operations,model_identity_links,import_view_names,library_resolution,dispatch_linking}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
---

## Summary

Ticket: QSL-1613. Plan completion: not assessed (planless). FR-480-AC-12 is tagged by three new assembler tests:

- `one_alias_two_units_bind_to_two_packages_names_each_units_own_package`
- `one_first_alias_of_two_packages_names_each_units_own_package`
- `a_unit_reaches_no_package_through_another_units_alias`

Two unit tests in `model_spelling.rs` also cover it. Each test asserts real types: equality with `M::ConfigVersion`, inequality across units, and the clause context equal to the parameter type. They are not tautological. FR-480-AC-9 and AC-11 (further alias across units) keep their existing tests in `assemble/tests.rs`, `tests/it/variant_fields.rs` and `tests/it/model_operations.rs`. The ticket asks for "the source unit in every type-name lookup". The production code no longer has any type-name lookup without one, except the timed-model clause views (code-review SR-20261104115608 FND-002), which no test covers.

Test evidence on head 170e02f8b, focused runs only, with QSPEC_DIR set to a read-only QSpec `origin/main` worktree:

- `cargo test -p qsl-semantics --lib -- check::assemble check::model_spelling check::type_form`: 122 passed.
- `cargo test -p qsl-semantics --test it -- variant_fields model_operations model_identity_links import_view_names library_resolution dispatch_linking timed_model`: 72 passed.
- `cargo test -p qsl-replay --test it -- aggregate_dispatch static_subject`: 15 passed and 2 FAILED. The failures are `aggregate_dispatch::split_pre_clauses_link_independently_of_request_order` and `split_d10_clauses_match_the_single_order_oracle_in_every_link_order`, both `UnresolvedTypeName` on `M::...` in a unit that selects the same `M` as another unit (code-review FND-001).

## Verdict

**CONDITIONAL.** Two existing multi-unit replay tests fail on this head (code-review FND-001). AC-12 itself is fully backed. The FR-480 sentence "in every position a type name is written" has no test for expression positions in the cross-unit layout. The one position found unit-blind, the timed-model predicates, also has no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers per-unit resolution in an expression position (variant case `N::T::Case`, `allInstances<N::T>`, `lookup<N::T>`) or in a timed-model predicate in a non-primary unit. That is where the unit-less timed-model views regress (code-review FND-002). Fix: add a two-unit test. Unit `b` alone selects `M`, and a time invariant or state clause body in `b` reads `M::V::Case` and `allInstances<M::T>`. Also cover the same-alias-two-packages variant. | qsl-semantics/src/check/assemble/tests.rs:4677-4812; qsl-semantics/src/check/timed_model.rs:4615 |
| FND-002 | medium | The qsl-semantics suite has no test for two units that each select one package under the SAME alias (`model M = X;` in both) and write `M::T` in the second unit. This is the layout code-review FND-001 breaks. Only the downstream `qsl-replay` aggregate_dispatch tests catch it. Fix: add an FR-480 (AC-9/AC-11) assembler test for that layout, in both unit orders. | qsl-semantics/src/check/assemble/tests.rs:4632-4676; qsl-replay/tests/it/aggregate_dispatch.rs:153-190 |
