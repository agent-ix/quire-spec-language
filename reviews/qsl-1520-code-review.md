---
id: SR-20261103161229
title: "Code review — QSL-1520 multi-source replay (FR-098 / FR-480)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2196 diff vs origin/main: qsl-replay/src/{execute.rs,execute/state_clause.rs,execute/tests.rs,admitted.rs,outcome.rs,spine.rs,spine/lifecycle.rs,spine/clause/tests/temporal.rs}, tests/it/replay_multi_source.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: reviews
---

## Summary

Ticket: QSL-1520. Code review with the rust-review lane folded in. The PR lets the
replay executor recompile a package reference, and each dependency entry, that names
several source units, validated and sorted into canonical owner order. The new
`run_spine_units` replaces the single-unit `run_spine_with_temporal_selections_and_execution_points`
for every `recompile_with_temporal_selections` caller, single-source requests included.
That swap drops two behaviours the old path had, and the cross-library owner check in
`DependencyInput::with_library_units` inspects only one other library.

## Verdict

**CONDITIONAL**: FND-001, FND-002 and FND-003 change replay results or admit an invalid
dependency input, and need fixing before merge. Clean parts: the request-side list
validation (empty, repeated owner, definition document) and its pointers/codes, the
state-clause source lookup through the claim occurrence, the `pointer()` `Cow` change
(no callers outside `qsl-replay`), and the `library_units` threading through preflight,
admission, byte accounting and `resolved_libraries_for_unit`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `run_spine_units` calls `spine::check_units`, which builds its `Resolution` with `temporal_selections: &[]`. The old path passed the request's selections into the check, where `temporal_selections_for(source)` feeds each dependency library's `emit_unit`. Temporal clock selections on a library source are now dropped for every replay, model-check replay and graph replay, single-source included. Fix: thread `temporal_selections` into `check_units`, or give it a variant that takes them, and add a replay test with a selection on a library source. | qsl-replay/src/execute.rs:1830-1837; qsl-replay/src/spine/lifecycle.rs:1311 |
| FND-002 | medium | The old replay check ran `check_for_continuation` (`continue_protocol_refusals = true`, and `attach_protocol_results` on a check refusal). `check_units` returns `Err` on the first `protocol_failure` and attaches no protocol results to other check refusals. So a replay of a package with a protocol refusal now refuses at S3 rather than continuing, and other S3 refusals lose their protocol results. This applies to single-source replays too. Fix: keep the continuation semantics for the replay recompile (a continuation flag on `check_units`, or route a one-unit list through `check_for_continuation`) and pin it with a test. | qsl-replay/src/execute.rs:1830-1840; qsl-replay/src/spine/lifecycle.rs:1345-1351 |
| FND-003 | high | In `with_library_units`, the cross-library shared-owner check does `self.library_units.iter().find(\|(other, _)\| **other != name).and_then(...)`. It takes the first other library and searches only that library's units. With libraries A=[a1], B=[b1,s] and C=[c1,s], a unit `s` shared by B and C is never detected, because both B and C compare only against A. `DependencyInput::new` checks only first units, and `canonical_dependency_sources` checks only within one entry, so two libraries compile the same owner. Fix: use `find_map` over every other library (or keep one global owner→library map) and add a three-library test. | qsl-replay/src/spine.rs:1984-2005 |
| FND-004 | medium | `begin_library` calls `self.dependencies.without_library(&identity)`. That clones the whole `DependencyInput`, including every supplied library's `bytes` (twice, since `libraries` and `library_units` both hold `SuppliedLibrary` copies), once per compiled library. So the memory cost is O(libraries × total dependency bytes), which `DependencySourceBytes` does not bound. Fix: have `admit_parsed_unit_list` take an excluded-library parameter (or a borrowed owner view), not a clone. Also stop duplicating `SuppliedLibrary` bytes in `libraries` and `library_units`. | qsl-replay/src/spine/lifecycle.rs:2961; qsl-replay/src/spine.rs:1924,2016-2022 |
| FND-005 | medium | `replay_frame` still calls `one_source(request.sources())` after `recompile` now accepts multi-source references. A multi-source frame replay does the full recompile and then refuses `SourceCount(n)`, and the comment "`recompile` already required exactly one source" is now false. Fix: resolve the source as `state_clause.rs` does, through the operation's declaring unit, or refuse before the recompile. | qsl-replay/src/execute/frame.rs:223-224 |
| FND-006 | low | `package_sources` and `canonical_dependency_sources` are the same validation and sort, copied (quadratic duplicate scan included). Fold them into one helper that takes the refusal constructor. | qsl-replay/src/execute.rs:1708-1776 |
| FND-007 | low | `source_names_definition_document` re-runs `package_input(package_documents())` and `document_reference` over the whole byte provision for each source reference. Compute the set of definition-document owners once per request. | qsl-replay/src/execute.rs:1744-1751 |
| FND-008 | low | The doc comment `/// A source reference's two FR-001 labels.` now sits on `canonical_dependency_sources`, and `labels` lost its doc. The new helpers have no docs. | qsl-replay/src/execute.rs:1707-1708 |
| FND-010 | medium | Before this PR, `recompile_with_temporal_selections` called `dependencies.check_unit_owner(&labels(source))`; the PR removes that call. A dependency entry that reuses the proved unit's owner now refuses through the spine's unit-list preflight instead of `ReplayRefusal::DependencyInput(SharedOwner { first: SourceHolder::Unit, .. })`. Head commit 581b0cf deleted the test's variant assertion and kept only `code() == InvalidPackage`. FR-098 says a dependency-input refusal is carried as `DependencyInput` (`conflicting-definition`). Fix: restore the per-unit `check_unit_owner` call before `run_spine_units` and restore the assertion. Otherwise, state the new refusal class in FR-098 and assert that exact variant and cause. | qsl-replay/src/execute.rs:1601-1647; qsl-replay/src/execute/tests.rs:1573-1581 |
| FND-009 | low | `with_library_units` reports a unit whose `identity` differs from the entry as `DuplicateIdentity`, and an invalid `identity` as `EmptyLibraryUnits`. Both are the wrong variant for the cause. Use a variant that states the mismatch (or `EmptyIdentity`). | qsl-replay/src/spine.rs:1937-1960 |

## Verification

- Reviewed head: 581b0cf (includes the late test-only commit "align dependency refusal assertion").
- Ran `cargo test -p qsl-replay --lib -- tc_444_dependency_entries tc_444_the_package` at 581b0cf: 2 passed.

- Ran `cargo test -p qsl-replay --lib -- protocol temporal tc_444` via `locked-build.sh` with the shared port-target: 107 passed, 1 failed. The failure, `timestamped_counter_cover_uses_half_open_boundary_rules`, is environmental: `QSPEC_DIR` is unset. None of these tests cover FND-001, FND-002 or FND-003.
- Did not run `tests/it/replay_multi_source.rs` (the root `it` binary), clippy or the workspace gates.

## Author follow-up

The conditional findings FND-001 through FND-010 were addressed after the reviewed
head: temporal selections and protocol-refusal continuation are threaded through
the multi-unit check; cross-library owner scanning covers every library; dependency
source bytes are shared across the filtered inputs; frame replay resolves its
declaring source through the package graph; dependency identity mismatches have a
typed refusal; and the refusal assertions and source-list helpers are restored or
consolidated. Focused verification on the follow-up tree passed both workspace
clippy configurations and the multi-source unit and integration tests (9 integration
cases). The workspace gate was not run on this PR head.
