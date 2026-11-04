---
id: SR-20261104091642
title: "Gap analysis — QSL-1616 PR #2274"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2274 head a8345af21; qsl-package/src/emit/protocol_locations.rs; qsl-package/src/emit/tests/protocol_locations.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: reviews
---

## Summary

Ticket: QSL-1616. Planless gap analysis of the FR-383 "Protocol term locations" criteria (AC-32..61) against the tests the PR adds. Plan completion: not assessed.

Covered by tagged tests that assert the behaviour: AC-34, AC-43..47 (if/let parts), AC-48, AC-49 (if arm), AC-50..59, AC-61 (pre-existing). Correctly bound: every trace on the six new tests matches what the test asserts. Untagged or untested: AC-35, AC-36, AC-37, AC-38, AC-42 (inspection), AC-60 (case, needs a model type fixture E lacks — stated in the PR). The FR-390 reader pairing (FR-390-AC-35/37/38/57/58) is deferred by the PR to QSL-1625.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-383-AC-32 is traced on `fixture_e_keys_each_visit_to_the_occurrence_recorded_at_its_position`, which spot-checks three applications of fixture E; nothing compares, for every slot and application, the row count with the visit count of the slot walk, and fixtures W and L are not exercised. This is the test that would catch the silent drop path (code-review FND-001). Fix: add a generic assertion over every emitted slot (rows per origin/application vs term positions of the checked body) for W, E and E-C. | qsl-package/src/emit/tests/protocol_locations.rs:134-193 |
| FND-002 | medium | FR-383-AC-35/36 (the row read back through QSL's I2 reader carries the term's checked `Location`) have no tagged test although the locked reader already exposes `CheckedPackageV2::protocol_location`; AC-36's two distinct `earlier.tally` locations are asserted on the wire but not tagged. The PR defers FR-390 pairing to QSL-1625, whose description covers computed-root parity only and does not mention `protocol_locations` pairing, so that deferred work has no owning ticket. Fix: tag AC-36, add an I2 `protocol_location` read-back for AC-35, and add the pairing scope to QSL-1625 or file a ticket. | qsl-package/src/emit/tests/protocol_locations.rs:146-193 |

## Verdict

CONDITIONAL — the new tests are real behavioural tests with correct traces, but AC-32 is only spot-checked and AC-35/36 are testable now and untested; the FR-390 pairing deferral needs a ticket that actually covers it.

## Finding data

```yaml
clean: false
scope:
  - id: FR-383-AC-32
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "For every slot of **W**, of TC-1070 fixture E and of protocol L (fixture L), the number of `protocol_locations` rows under the slot's site name for an application equals the number of that application's visits in the slot's walk."
  - id: FR-383-AC-35
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "Each row, read back through QSL's I2 reader, has a `Location` equal to the `Location` of the term at the position it numbers in the checked clause held in memory."
  - id: FR-383-AC-36
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "The two retry-origin `earlier.tally` rows of `Refund` carry different `Location` values, the first that of the term in `earlier.tally < 3` and the second that in `earlier.tally + 1`."
  - id: FR-383-AC-43
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "Removing the retry-origin `earlier.tally` row of `Refund` keyed to occurrence 1 from fixture E's emitted package (leaving the occurrence-0 row) leaves a package QSL's I2 reader admits."
  - id: FR-383-AC-44
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "Changing the `term_ordinal` of the retry-origin `earlier.tally` row of `Refund` keyed to occurrence 1 from 0 to 1 (a group with no ordinal-zero row) is refused by QSL's I2 reader with `invalid_package`/`invalid-value` at `/checked_term_data/protocol_locations`."
  - id: FR-383-AC-46
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "Duplicating a retry-origin `earlier.tally` row (same key, written next to the original) is refused by QSL's I2 reader with `invalid_package`/`conflicting-definition` at `/checked_term_data/protocol_locations/<index of the duplicate>`."
  - id: FR-383-AC-52
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "In fixture E `target >= 0` has one row of the trigger-guard origin of `Refund`, keyed to its `expression` occurrence 0, and one row of the retry origin, keyed to occurrence 1; each group holds exactly one row."
  - id: FR-383-AC-56
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "In fixture E-C `order >= 0` has one row of the `Tried` constraint's origin, keyed to its `expression` occurrence 0, and one of the `Picked` constraint's origin, keyed to occurrence 1; each group holds exactly one row."
  - id: FR-383-AC-59
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "In fixture E-C no `protocol_locations` row names a `Local` read (`x`, `b`, `c`), a literal, a `let`, a conditional or a case node."
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-383-AC-32
    related_ids: []
    path: qsl-package/src/emit/tests/protocol_locations.rs
    lines: "134-193"
    excerpt: "For every slot of **W**, of TC-1070 fixture E and of protocol L (fixture L), the number of `protocol_locations` rows under the slot's site name for an application equals the number of that application's visits in the slot's walk."
    finding: "AC-32 is only spot-checked on three fixture E applications; no test compares row count with walk visits for every slot and application, and W and L are not exercised."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-383-AC-35
    related_ids: [FR-383-AC-36]
    path: qsl-package/src/emit/tests/protocol_locations.rs
    lines: "146-193"
    excerpt: "Each row, read back through QSL's I2 reader, has a `Location` equal to the `Location` of the term at the position it numbers in the checked clause held in memory."
    finding: "AC-35/36 are untested though the reader exposes protocol_location; AC-36 is asserted but untagged; the FR-390 pairing deferral points at QSL-1625, which does not cover it."
bindings:
  - {test_id: fixture_e_keys_each_visit_to_the_occurrence_recorded_at_its_position, ac_id: FR-383-AC-34, trace: correct}
  - {test_id: fixture_e_keys_each_visit_to_the_occurrence_recorded_at_its_position, ac_id: FR-383-AC-52, trace: correct}
  - {test_id: every_fixture_e_row_is_under_its_slots_site_name, ac_id: FR-383-AC-39, trace: correct}
  - {test_id: the_reader_admits_the_rows_and_a_package_missing_one_whole_group, ac_id: FR-383-AC-43, trace: correct}
  - {test_id: a_group_with_no_ordinal_zero_row_is_refused_at_i2, ac_id: FR-383-AC-44, trace: correct}
  - {test_id: a_group_that_skips_an_ordinal_is_refused_at_i2, ac_id: FR-383-AC-45, trace: correct}
  - {test_id: a_duplicated_row_is_refused_at_i2_at_the_duplicate, ac_id: FR-383-AC-46, trace: correct}
  - {test_id: e_c_shared_constraint_rows_are_keyed_tried_zero_picked_one, ac_id: FR-383-AC-56, trace: correct}
  - {test_id: e_c_let_and_if_rows_cover_every_term_and_no_non_term, ac_id: FR-383-AC-59, trace: correct}
```
