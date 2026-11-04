---
id: SR-20261104130235
title: 'Code review of PR #2296: project computed protocol roots from reader data, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@52012fa99; git diff origin/main...52012fa99 (PR #2296): qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/evaluate/reader_source.rs, qsl-eval/tests/it/protocol_slots/acceptance.rs, qsl-package/src/reader_protocol_projection.rs, qsl-package/src/checked_v2.rs, qsl-package/src/evaluation_view.rs, qsl-package/src/emit.rs, qsl-package/src/emit/equality_reader.rs, qsl-package/src/emit/monitor_plan.rs, qsl-package/src/lib.rs; ticket QSL-1625'
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: reviews
date: '2026-11-04'
---

## Summary

Ticket: QSL-1625. PR: quire-spec-language#2296, head 52012fa99. Method: code-review with the rust-review lane. The PR adds an owned reader projection in qsl-package and a `ReaderNode` that implements the `NodeSource` trait from #2273. Computed protocol roots now run on the one generic `Machine` over that projection. It also un-ignores the FR-390-AC-15 and AC-24 to AC-27 tests.

Focused tests on the head: 40 passed, 6 failed in `protocol_slots::acceptance`. One of the failures, `reader_backed_shared_meter_sums_and_bounds_in_both_orders`, passes on origin/main ee0f6c400, so it is a regression. Another, the TC-1073 test, also passes on main in its main form. The head commit (narrowing the CheckedBody location fallback to literal roots) leaves Local reads and composed nodes with no location source. Commit 99daad9f1, the one before the head, does not compile on its own, so no earlier commit gives a green baseline.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| project_protocol_root, ReaderBuilder | qsl-package/src/reader_protocol_projection.rs | examined |
| ReaderNode NodeSource impl | qsl-eval/src/value/expression/evaluate/reader_source.rs | examined |
| evaluate_protocol_slot reader branch | qsl-eval/src/value/expression/mod.rs:1941-2008 | examined |
| protocol_reference_slot exact type | qsl-package/src/checked_v2.rs:1059-1260 | examined |
| project_reader_protocol_root, selector comment | qsl-package/src/evaluation_view.rs:54-95,183-196 | examined |
| Emission::admitted_equality | qsl-package/src/emit/equality_reader.rs | examined |
| BaseDimension witness row | qsl-package/src/emit.rs:2835-2867 | examined |
| TC-1073, AC-15, AC-24..27 tests | qsl-eval/tests/it/protocol_slots/acceptance.rs | examined |
| ReaderRoot / Task::ReaderRead | qsl-eval/src/value/expression/evaluate.rs:493-503,1005,1745 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Six reader tests fail on the head with `reader-local-location-is-admitted` or `reader-expression-location-is-admitted`: TC-1073, AC-15, AC-24, AC-26, AC-27 and the shared-meter test. The shared-meter test passes on origin/main, so this is a regression. Locals and non-application nodes take a location only from `protocol_locations` rows, which name only applications, or from an inherited parent location. Function bodies get none (`read_functions(root, None)`). | qsl-package/src/reader_protocol_projection.rs:108-111,220-224,243-246,1050-1055 |
| FND-002 | high | The literal-root location is taken from the source package's CheckedBody (`selected.checked_body().root().location()`). FR-390 says a location SHALL NOT be taken from the source package, from a source region, or defaulted. It also says a literal or Local reads its own admitted node's location (AC-50, AC-60). | qsl-eval/src/value/expression/mod.rs:1949-1953; qsl-package/src/reader_protocol_projection.rs:105-110 |
| FND-003 | high | A root-kind special case remains. Plain-data roots (bare Reference or record.project) still evaluate the source `body.root()` CheckedBody through `ReaderRoot`/`Task::ReaderRead`. Every other root goes through `ReaderNode`. That is two reader paths chosen by root shape, which AC-16 and the ticket design (replace ReaderRoot/Task::ReaderRead) forbid. | qsl-eval/src/value/expression/mod.rs:1943,1975-2003; qsl-eval/src/value/expression/evaluate.rs:493,1005,1745 |
| FND-004 | high | Location pairing is wrong. `location()` takes the first emitted row whose application matches and ignores both the slot's origin name and the visit index. The memo on (context, node) merges a shared node into one handle. A missing row falls back silently to the parent location instead of `MissingLocationProjection`. The keys also come from the producer's `Emission` rows, not from the admitted wire. As a result, an application visited twice or held by two slots reports the wrong location. | qsl-package/src/reader_protocol_projection.rs:217-219,243-246,1050-1064 |
| FND-005 | medium | The TC-1073 test was weakened. The reader halt assertion `Unavailable { location: None, .. }` became `Unavailable { .. }`, and nothing replaces it to check the reader location against the admitted leaf or row (AC-64). This is the assertion that would catch FND-002 and FND-004. | qsl-eval/tests/it/protocol_slots/acceptance.rs:3052-3054 |
| FND-006 | medium | Unsupported forms and operations return an `InternalFault` during evaluation, not a typed `ViewFailure::Unsupported` at view creation. No test covers any projection refusal. `project_protocol_root` always returns `Ok(Some)`, so its `Option` is vacuous and the `reader-computed-root-is-projected-or-typed-refused` fault is unreachable. The `selected.reader_projection().is_none()` re-check is also redundant. | qsl-package/src/reader_protocol_projection.rs:84,114,384,818; qsl-eval/src/value/expression/mod.rs:1954-1960 |
| FND-007 | medium | The projection is rebuilt on every `evaluate_protocol_slot` call. Each projected node does a linear scan of all admitted nodes, and each call recomputes `selected_exact_identities`. `wire_graph()` is rebuilt for every union, record, case and `reader_node_id` lookup. `admitted_equality` rebuilds every producer application and fact row for each equality node. The result is roughly O(nodes^2) per slot call. Fix: build the projection once per view and index nodes by id. | qsl-eval/src/value/expression/mod.rs:1943-1953; qsl-package/src/reader_protocol_projection.rs:146-152,236-242,269,560,737; qsl-package/src/emit/equality_reader.rs:22-170; qsl-package/src/checked_v2.rs:577-596 |
| FND-008 | medium | Root-context case binders take slots `max(locals)+1..`, but only function contexts grow `slots` (`functions.get_mut(context)` is None at the root). The frame is sized from the source `body.frame_slots`, which FR-390 forbids the reader to consult. | qsl-package/src/reader_protocol_projection.rs:591-595,642-644; qsl-eval/src/value/expression/mod.rs:1942 |
| FND-009 | low | A stale comment says the reader never takes a location from the source CheckedBody. FND-002 contradicts it. | qsl-package/src/evaluation_view.rs:187-191 |
| FND-010 | low | `project_node` recurses once per expression depth, so a very deep reader expression can overflow the stack. The PR body defers this to FR-460. | qsl-package/src/reader_protocol_projection.rs:210-395 |

## Verdict

CONDITIONAL. Not mergeable at 52012fa99.

What is right: computed roots run through the same generic `Machine` over a `NodeSource` (`ReaderNode`), with no second walker. No test is re-ignored: all six `#[ignore]`s were removed and none were added. `protocol_reference_slot` now takes its exact type from the admitted graph instead of the CheckedBody root type. The new `equality_reader.rs` matches exhaustively over `EqualityFactWitness`, the operand, unit and dimension roles, and `EqualityDimensionRole` with no wildcard. The `Emission` destructure in `DebugWith` includes the new field. No touched enum gained a variant.

CheckedNode hot path: no regression. The diff does not touch `Machine` or `CheckedNode`, and the timings match within noise (3 runs each, wall time).

- The 10k-call predicate chain took 8.25, 7.41 and 7.81 s on the head, against 7.44, 8.24 and 8.26 s on main.
- `timed_trace::the_repeated_interval_forms_agree_with_the_evaluator` took 9.08, 9.16 and 9.85 s on the head, against 9.33, 8.62 and 10.33 s on main.

Fixes needed:

- FND-001: give Local and literal reads their own admitted node location, and pair application rows per slot walk.
- FND-002: drop the CheckedBody location.
- FND-003: route plain-data roots through `ReaderNode`, then delete `ReaderRoot` and `Task::ReaderRead`.
- FND-005: restore a real location assertion.
