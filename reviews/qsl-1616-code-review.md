---
id: SR-20261104091641
title: "Code review — QSL-1616 PR #2274"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2274 head a8345af21; qsl-package/src/emit/protocol_locations.rs; qsl-package/src/emit.rs; qsl-package/src/emit/tests/protocol_locations.rs; qsl-package/src/emit/tests/protocol_recursion.rs; qsl-package/src/emit/tests/protocol_controls.rs; qsl-package/src/evaluation_view.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: reviews
---

## Summary

Ticket: QSL-1616. Code review with the Rust lane (rust-review) of PR #2274 at head a8345af21 against origin/main 58d79f4ea. The PR replaces the empty `protocol_locations` vector with rows computed by a new `emit/protocol_locations.rs`, adds `EmitRefusal::MissingProtocolLocationContext`, and splits the 100k-member cycle test so its I2 read is `#[ignore]`d.

Checked against FR-383 "Protocol term locations" and the locked IR reader `quire-contract-model` 64de9ec `protocol_locations::validate_rows`:

- Ordering: rows are grouped in a `BTreeMap<(CheckedNodeId, CheckedOccurrence), _>` and emitted in that key order with `term_ordinal` 0.. per group, which is the same derived `Ord` the reader's `key < prior` tuple compare uses, so ordering, duplicate and missing-predecessor checks are satisfied by construction.
- Occurrence keying: a position's row takes the `expression` occurrence recorded at that exact `Location`; `graph.occurrences()` is documented ascending by node, role, ordinal, so the j-th visit at one location takes the j-th recorded occurrence. Every row's application is an `expression`/`application` node (filtered in `applications_at`), and every occurrence comes from the checker's occurrence table, which the source map carries, so the reader's application, occurrence and source-map joins hold.
- Every slot kind: `slot_bodies` lists activation guard, capture initializers (registration and activation captures included), compensation guard/retry/recovery, control bodies, choice/repeat visible and guards, related ends, terminal, role until, channel key; the repeat loop proof is excluded and lives under its own decision span origin, so it cannot collide with a walked slot's candidates. All slot sites are built by `protocol_location`, a `type_declaration` origin, so `wire_location` cannot refuse a slot that the checker located.
- Several applications/origins: because each row's occurrence is recorded at the row's own location and each location belongs to one origin, every emitted group is single-origin; the cross-origin numbering in `ordered` is spec-conformant and never produces a gap.
- False-positive refusals: `MissingProtocolLocationContext` fires only for two unrelated outermost applications at one location, a missing occurrence role mapping, or an application with no emitted node id. None is reachable for a checked, emitted protocol under current lowering, and `emit_checked_never_returns_incomplete_across_the_fixture_set` panics on the new variant, so the full fixture set guards against it.
- `#[ignore]`: justified. 100k visits of one application give 100k occurrences; the reader charges a node scan, an occurrence scan and a source-map scan per row (~4e5 units), so 1e8 exhausts near row 250, matching the stated row 241. The emitted rows are what FR-383 requires; the cost is the IR reader's (IR-1321).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A term-kind position whose `Location` has no recorded application gets no row and no refusal (`applications.get(at)` is `None` → `continue`). FR-383 requires one row per term and states a general rule for visits with no occurrence recorded at their position; the emitter neither applies it nor refuses, so a future lowering that records an occurrence elsewhere silently drops rows and violates FR-383-AC-32 with an admitted package. Fix: refuse `MissingProtocolLocationContext` when a position whose `NodeKind` always lowers to an application (comparison, arithmetic, connective, not, attribute, field, call, coerce) has no candidate, or add the AC-32 total-count test (see gap analysis). | qsl-package/src/emit/protocol_locations.rs:93-100 |
| FND-002 | low | The `#[ignore]` reason names no ticket although IR-1321 now exists (the PR body says filing was blocked), and the non-ignored `a_100000_member_function_protocol_cycle_is_budgeted_and_order_invariant` keeps the FR-383-AC-25 trace while no longer doing AC-25's read-back. Fix: cite IR-1321 in the ignore reason. | qsl-package/src/emit/tests/protocol_recursion.rs:490-505 |

## Verdict

CONDITIONAL — emitted rows satisfy FR-383 keying, ordering and consecutive ordinals and the reader's `validate_rows` for every slot kind; no admitted package becomes refused under current lowering; the `#[ignore]` hides an IR reader cost, not a QSL defect. FND-001 (silent drop path) and FND-002 (ticket in ignore reason) should be fixed in this PR.

## Scope and method

Read the full diff, the locked reader `validate_rows`, `CheckedGraph::occurrences`, `CheckedBody`/`Node::location`, the slot site construction in `qsl-semantics/src/check/protocol_slots.rs` and `lowering/protocol.rs`, and FR-383 lines 195-322 and AC-25, AC-32..61. Focused tests were run (results in the reviewer report).

## Finding data

```yaml
clean: false
scope:
  - id: FR-383-AC-32
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "For every slot of **W**, of TC-1070 fixture E and of protocol L (fixture L), the number of `protocol_locations` rows under the slot's site name for an application equals the number of that application's visits in the slot's walk."
  - id: FR-383-AC-25
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "A 100,000-node cycle group emits and reads back within the supplied budget without native stack overflow; the budget refuses at zero and one unit short and succeeds at the exact measured work; permuting authored node order leaves group identity unchanged."
  - id: FR-383-AC-58
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "For every application of **W** and fixtures E, L and E-C, the (occurrence ordinal, `term_ordinal`) pairs of a slot's rows of that application, taken along the slot walk's visits, strictly increase."
findings:
  - fnd: FND-001
    severity: medium
    confidence: medium
    method: code-review
    check: code-bug
    artifact_id: FR-383-AC-32
    related_ids: [FR-383]
    path: qsl-package/src/emit/protocol_locations.rs
    lines: "93-100"
    excerpt: "For every slot of **W**, of TC-1070 fixture E and of protocol L (fixture L), the number of `protocol_locations` rows under the slot's site name for an application equals the number of that application's visits in the slot's walk."
    finding: "A term-kind position whose Location has no recorded application gets no row and no refusal; FR-383's general rule for visits without a recorded occurrence is neither applied nor refused, so a lowering change silently drops rows."
  - fnd: FND-002
    severity: low
    confidence: high
    method: code-review
    check: trace
    artifact_id: FR-383-AC-25
    related_ids: []
    path: qsl-package/src/emit/tests/protocol_recursion.rs
    lines: "490-505"
    excerpt: "A 100,000-node cycle group emits and reads back within the supplied budget without native stack overflow; the budget refuses at zero and one unit short and succeeds at the exact measured work; permuting authored node order leaves group identity unchanged."
    test_id: a_100000_member_function_protocol_cycle_is_budgeted_and_order_invariant
    claimed_ac: FR-383-AC-25
    correct_ac: FR-383-AC-25
    finding: "The #[ignore] reason names no ticket although IR-1321 exists, and the remaining traced test no longer performs AC-25's read-back."
bindings:
  - {test_id: a_100000_member_function_protocol_cycle_reads_back_at_i2, ac_id: FR-383-AC-25, trace: correct}
```
