---
id: SR-12168
title: "Gap analysis of quire-spec-language PR #1008: FR-337, FR-123-AC-1 and FR-326's requirement record against QSpec FR-370"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8484189f18ba077ecf506179f72d56337f9c1289; PR #1008 diff against merge-base 9a8e7803c; against QSL FR-337 (AC-1 to AC-5), FR-123-AC-1, FR-326 Requirement record, TC-524, TC-1079, TC-518, and QSpec FR-370"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1008

## Summary

Ticket: QSL-511. Planless. Plan completion: not assessed. Read-only: the
criterion-to-test check was done by hand over the PR's tests and `#[trace]`
tags; nothing was built or run. QSpec read at its current main, cited by id.

Per criterion, each tagged test checked for whether it can fail on its AC:

- **FR-337-AC-1** (`reaches_two_emits_as_a_v2_temporal_clause_node_the_i2_read_admits`,
  TC-524): asserts the node tag and form, the `temporal` operator, the
  `quire.op.temporal.clause` identity, the one `temporal_profile` law equal
  to the lock row, the `null` member, all six arguments by position and
  shape, the one `fair` member (`weak`, `each`, `attemptUpdate`, declaring
  `model`/`object_type`), `always(eventually(holds(..)))` with `null`
  intervals, Boolean typing, `claim` occurrences, and the I2 read admitting
  it. Can fail on every clause of the AC.
- **FR-337-AC-2** (`the_resolved_fairness_set_and_the_intervals_decide_the_node_key`):
  byte-equal fairness bodies and equal keys for unmarked and `whole`;
  different key and `package_id` for `each` and for `eventually[0,5]`. Can
  fail. See FND-003 for the spelling of the operation.
- **FR-337-AC-3** (`each_interval_form_emits_its_temporal_interval_member`,
  TC-524 only): the three interval forms, identities, empty fairness
  argument and I2 admission. Strong previous is written as `once[1,1]`
  because the grammar has no strong previous (declared gap). See FND-002.
- **FR-337-AC-4** (`the_i2_read_refuses_each_fr370_defect_at_its_locus`):
  absent fairness operation (`missing_declaration`/`missing-name` at
  `/member/name`), a fairness node referenced from the formula
  (`ill_typed`/`operator-ineligible` at the referencing node), and a `null`
  interval under event-position (`invalid_package`/`operation-member-mismatch`
  at the formula body) each pin code, cause and locus. The reordered
  arguments case pins only the locus, since IR reports
  `ill_typed`/`operator-ineligible` there, not a schema failure (declared
  gap). Can fail on each pinned clause.
- **FR-337-AC-5** (three TC-1079 tests): triple and `over` node equal to the
  checked graph's, resolution in a recompiled package, two identical
  declarations sharing one node with two ordinals each resolving, and the
  four foreign triples returning `clause-root-unresolved`. The no-`over`
  half waits on STD-186 (declared gap). Can fail.
- **FR-123-AC-1** (two TC-518 tests, now also tagged `FR-123-AC-1`): the
  set half was already tested; `node_identity` now runs the whole package
  check and asserts unmarked equals `whole` and `whole` differs from `each`
  by clause node key. The node-identity half is now covered. Can fail.
- **FR-337 `holds` typing** (`a_holds_condition_is_typed_over_the_over_parameter`,
  tagged `FR-337`): an unbound name refuses at its own source region, and a
  non-Boolean condition refuses `NonBooleanRoot`.
- **FR-326 refusal of unemitted clauses**
  (`the_package_check_refuses_a_clause_no_emission_writes`): `on origin`
  compiles; `on each ... when` and a capture each refuse `NotEmitted` at the
  clause.
- **Declared gaps** (not blocking): strong previous; IR's reorder code;
  FR-337-AC-5's no-`over` clause (STD-186); FR-338-AC-8 / TC-1080 (QSL-880).

No production code without an owning requirement: the FR-326
`temporal-satisfaction` record is FR-326 "Requirement record"; the clause
root lookup is FR-337 "Outputs" and "Behavior".

## Verdict

PASS. No major gap. One medium coverage gap on the newly emitted FR-326
record and two lows, recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The PR now writes one FR-326 `temporal-satisfaction` record per `claim` occurrence into the checked graph, and from there into the package. No test asserts it at that level: not the key (clause node, `claim` occurrence), not the infinite-trace domain keyed by the real clause node, not the `over` parameter's domains, and not the model references read from the conditions. The FR-326 tests in `temporal_clause/tests.rs` call the family's `requirements` with stand-in node ids only. A wrong key or a dropped record would pass every test except the indirect I2 admission. | qsl-semantics/src/check/mod.rs:2276-2335; qsl-semantics/src/check/temporal_clause/tests.rs:133-160 |
| FND-002 | low | `each_interval_form_emits_its_temporal_interval_member` asserts FR-337-AC-3 but is tagged `TC-524` only, while the AC-1, AC-2 and AC-4 tests also carry their AC ids. No test in the workspace carries `FR-337-AC-3` (grep over `*.rs`; `quoin matrix` timed out at 300 s and was not used). Tag it `FR-337-AC-3` and say in the doc that strong previous is written as `once[1,1]`. | qsl-package/src/emit/tests/temporal_clause.rs:334-336 |
| FND-003 | low | FR-337-AC-2 names `fair weak attemptUpdate`, the operation written alone and read in the `over` type's view. The test writes `Config::ConfigVersion::attemptUpdate` in all three cases, so node-key equality of the alone form with the qualified form is not tested. FR-123's set-level test does cover the alone form. | qsl-package/src/emit/tests/temporal_clause.rs:298-303 |
