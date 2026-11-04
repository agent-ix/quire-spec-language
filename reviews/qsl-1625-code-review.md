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

## Dispositions

Disposition pass 1, head 391cb15ed (trial merge with origin/main 0642337ef). The coder's disposition notes in the PR's copy of this file were treated as claims and checked against the code and the focused runs.

Evidence on the trial merge: `cargo check --locked --workspace --all-targets` passed. Both `cargo clippy --workspace --all-targets -D warnings` runs passed, default and `--all-features`. `qsl-eval --test it protocol_slots::acceptance`: 46 passed, 0 ignored. qsl-package protocol_location/reader/equality/evaluation_view/checked_v2: 123 passed. qsl-replay `reader` tests: 17+4 passed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | All 46 `protocol_slots::acceptance` tests pass on the trial merge, with 0 ignored. Leaf nodes, `case` nodes and function-body nodes carry `location: None`. An application in the slot walk reads its paired row (reader_protocol_projection.rs:441-516). |
| FND-002 | fixed | Nothing reads a `CheckedBody` location. The source body supplies only the slot origin name (`slot_origin`) and the root key (`root_node`), which the selector is allowed to supply (FR-390). |
| FND-003 | fixed | `ReaderRoot`, `Task::ReaderRead`, `eval_reader_read`, `protocol_reference_slot` and `ReaderProtocolField` are deleted. Every reader slot runs `run_protocol_frame(ReaderNode::root(..))` on the one `Machine` (qsl-eval/src/value/expression/mod.rs:1978-2003). |
| FND-004 | fixed | Rows come from the admitted `checked_term_data.protocol_locations`. They are grouped by origin name and application and sorted by (occurrence ordinal, `term_ordinal`). Visit k takes row k. A visit with no row, or a row with no visit, refuses the whole package with `MissingLocationProjection` (reader_protocol_projection.rs:402-516). Tests cover AC-31/33/35/37/38/52/53/54/57 and pass. |
| FND-005 | fixed | acceptance.rs:3058-3064 asserts `Unavailable { location: None, .. }` again for the reader halt. |
| FND-006 | still-open | The refusal is now a typed `UnsupportedView` with a test, and the vacuous `Option` is gone. It is still not a view-creation refusal, which is what the finding asked for. It is held per slot (`ReaderSlot::Unsupported`/`ReaderSlot::Fault`) and returned at evaluation time. See FND-011. Close it by fixing FND-011, or move it to `deferred` once FND-011 has a ticket. |
| FND-007 | fixed | The package is projected once per view (checked_v2.rs `evaluation_view` -> `project_package`), over an id/key index. Identities and the wire graph are built once. A smaller residue remains (FND-015). |
| FND-008 | fixed | Root-context case binders start at `root_slots` (the slot's binder count) and grow `root_slots`. The reader frame is sized from `projection.frame_slots()` (reader_protocol_projection.rs:1130-1142,1192-1201; mod.rs:1993). |
| FND-009 | fixed | The stale comment was removed along with the code it described. |
| FND-010 | deferred | QSL-1660 (Backlog) exists and owns it. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | The per-slot refusal is a deviation from FR-390, and no ticket records it. FR-390 says view creation is package-wide, that an unsupported projection leaves no view, and that once a view exists `read_node` never returns a late unsupported projection. FR-390 also says every operation identity QSL emits SHALL project, and that the identity map is total, so `MissingOperationProjection` is unreachable (AC-41, AC-51). This PR leaves `quire.op.control.if`/`let`, the `collection.*` and `option.*` operations, `enum.eq`/`ne`, `reference.eq`, `text.eq`, integer `div`/`mod`/`rem`/`negate`, rational and decimal arithmetic, `model.navigate`/`lookup`/`all_instances` and `numeric.convert` unprojected. It holds each such slot as `ReaderSlot::Unsupported`, which evaluation returns as `unknown_required_feature`. Nothing yields a wrong value and nothing is charged, so this is acceptable only as a recorded interim step. Fix: file a ticket that owns projecting the remaining FR-400 kinds (AC-41..49, 51) and then switching the refusal to package-wide `ViewFailure::Unsupported`. Reference the ticket in the PR body and in the `evaluation_view` doc comment. | qsl-package/src/reader_protocol_projection.rs:262-279,1008-1011; qsl-package/src/checked_v2.rs:999-1004; qsl-eval/src/value/expression/mod.rs:1997-2002 |
| FND-012 | medium | Record-field presence for `NodeKind::Field` is read from the source package's type scope (`source_graph.scope().types().composite(..)`), not from the admitted wire. FR-390 says the source package never supplies a node's operation, children, type or location. The model-object branch already reads presence from the admitted wire (`model_object_fields`). Fix: read the record member's Option-ness from the admitted record declaration node's members. If IR cannot express it yet, return a typed unsupported result. | qsl-package/src/reader_protocol_projection.rs:1303-1321 |
| FND-013 | medium | When more than one projection is unsupported, FR-390 requires the reported refusal to be the first by ascending node key, then reason order. `project_package` returns the first `MissingLocationProjection` it meets in slot-request order and walk order instead. No test covers a package with two defects. Fix: collect the refusals, then report the minimum by (locus, node key, reason order). | qsl-package/src/reader_protocol_projection.rs:261-280,487-514 |
| FND-014 | low | `ParitySlot::reader_backed` is now true for every slot, and the test asserts `all(reader_backed)`, so the field is vacuous. Delete the field and its filter (acceptance.rs:4072). | qsl-eval/tests/it/protocol_slots/acceptance.rs:2601,2901,4072 |
| FND-015 | low | Per-node linear work remains at view creation. `literal_value` (Text) and `project_literal_type` scan every admitted node and rebuild `wire_graph` for each literal. `admitted_equality` rebuilds every producer application row and fact row for each equality node. Fix: pass the `WireIndex` (nodes, graph) into these helpers, and build the equality row vectors once per view. | qsl-package/src/reader_protocol_projection.rs:1632-1637,1689-1714; qsl-package/src/emit/equality_reader.rs:22-176; qsl-package/src/checked_v2.rs:390-399 |
| FND-016 | low | The refusal record's `reason` field is filled with `format!("{:?}", ..)`, so the Rust `Debug` layout becomes record content. Fix: give `UnsupportedReason` a stable name (for example `MissingOperationProjection:<identity>`). | qsl-eval/src/value/expression/mod.rs:143-145 |
| FND-017 | low | The PR body is stale. It describes head 52012fa99 and a "narrowed CheckedBody location fallback" that no longer exists, says the focused run did not complete, and does not mention the per-slot deviation. Fix: rewrite it for 391cb15ed. | PR #2296 body |

## Dispositions (pass 2)

| ID | Disposition |
| --- | --- |
| FND-011 | Fixed in the PR. The deviation is ticketed as QSL-1666 (project the remaining FR-400 kinds, then package-wide refusal); the `evaluation_view` doc comment in `checked_v2.rs` and the PR body reference it. |
| FND-012 | Partly fixed. Presence now comes from the admitted record: IR's `project_exact_type` record view first, else the admitted `composite_type/record` wire declaration's own member (an option-typed member is an optional field). Residual: a model `record_value_type` whose fields IR's complete-or-refuse view cannot project (option or collection members) carries no members on the wire and no public IR accessor, so its presence still comes from the package type scope, joined by the exact record key (`source_member_presence`). Returning a typed unsupported there would regress the AC-18 nested option/collection test, so it needs an IR accessor for record field presence. |
| FND-013 | Fixed. `pair` collects every unpaired visit or row and `first_refusal` reports the minimum by (node key, visit-before-row, index) across all slots. Test `reader_reports_the_smallest_node_key_of_several_refusals` drops six rows in both orders and was checked to fail when the first-met refusal is kept. |
| FND-014 | Fixed. `ParitySlot::reader_backed` and its filter and assertion are deleted. |
| FND-015 | Fixed. `literal_value` and `project_literal_type` take the `WireIndex` (node map, key map) instead of scanning nodes and rebuilding `wire_graph`; the producer equality rows are rejoined once per view (`Emission::equality_rows`, `admitted_equality_from`). |
| FND-016 | Fixed. The record `reason` field is `UnsupportedReason::name()` (`missing-operation-projection:<identity>` and the like). |
| FND-017 | Fixed. The PR body describes the current head, the per-slot deviation and QSL-1660/QSL-1666. |
