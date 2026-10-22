---
id: SR-12297
title: "Gap analysis of QSL-947 PR #1174 against FR-398 (admitted unit table through the replay facade)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7f746bf97; PR #1174, commit 7f746bf97 only (new against origin/qsl-946-impl): qsl-replay/src/admitted.rs, qsl-replay/tests/admitted_unit_facade.rs, qsl-replay/tests/admitted_unit_root_only.rs; spec/functional/FR-398, spec/test-cases/TC-1092; ticket QSL-947 acceptance"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-398
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1092
    type: reviews
---
# SR-12297: Gap analysis of QSL-947 PR #1174 against FR-398 (admitted unit table through the replay facade)

## Summary

Ticket: QSL-947. Plan completion: not assessed. Ticket text was read as data.

| Criterion | Test | State |
| --- | --- | --- |
| FR-398-AC-1 | `a_consumer_builds_a_quantity_conversion_from_admitted_units` (root-only) | covered; mutation-killed (wrong target unit fails) |
| FR-398-AC-2 | `unit_ids_are_the_spines_distinct_and_stable`, `the_unit_table_holds_every_admitted_unit_by_id` | covered; independent oracle (checked signature and semantic nodes); id mutant killed |
| FR-398-AC-3 | `unit_lookups_refuse_unknown_and_wrong_kind_ids`, `a_dependencys_unit_is_foreign` | covered (direct import and through an intermediate library) |
| FR-398-AC-4 | TC-1089, TC-1090 suites (`admitted_context_facade`, `admitted_root_only`, `admitted_composite_*`) | pass at 7f746bf97 |

Ticket acceptance:

- Check 1 (root-only `UnitId` and the conversion relation): met.
- Check 2 (distinct, stable, typed refusals): met.
- Check 3 (titan:kani:plan confirms CG can lower `Quantity<u>`): not in this PR's scope. The PR states it as remaining work owned by titan.

The code adds nothing that no requirement owns. I found no stubs and no tautological assertions: every expected value comes from the spine's graph or from a literal.

## Verdict

No major gaps. The acceptance criteria are covered by tests that fail under mutation. The gaps below are behaviour that FR-398 states, or relies on, with no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-398 Behavior states, and the PR body calls out as a public behaviour change, that a declared but unused unit is listed and found, and that its id looked up as a record or tuple is now `WrongKind`. No test declares an unused unit. All three units in the TC-1092 fixtures are parameters of `f`. A probe measured the behaviour: an unused unit is listed, `unit_declaration` finds it, and `composite_declaration` refuses it `WrongKind`. Nothing pins this. Add an unused unit to the AC-2/AC-3 fixture and assert it. | qsl-replay/tests/admitted_unit_facade.rs:27-35 |
| FND-002 | low | No facade test covers a derived dimension (`is_base() == false`, and the dimension walk through `terms`), a scaled chain that does not pass through the target (km to mm), or an affine unit (C to K). All three work by probe: 2 km is 2,000,000 mm, 0 C is 5463/20 K, and `Speed = Length / Time` is listed as not base. The dimension walk over `terms` in `unit_tables` has no test at all. Every fixture dimension is base, and the test asserts `all(is_base)`. | qsl-replay/src/admitted.rs:722-741 |
| FND-003 | low | The claim that a replayed quantity value carries `AdmittedUnit::id` holds by trace only. Replay types a quantity argument from the same checked signature that AC-2 compares against (execute.rs:1322, `WitnessValueType::Canonical`). No test replays a `Quantity<u>` argument and compares its unit with the facade's id. A one-step replay of `f` with a `5 m` argument would close this. | qsl-replay/src/execute.rs:1322 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: test an_unused_unit_is_found_and_a_dimension_no_unit_names_is_not_listed | 66a821ff2 |
| FND-002 | fixed: test derived_dimensions_chains_and_offsets_convert (derived parts, 2 km = 2,000,000 mm, 0 C = 5463/20 K) | 66a821ff2 |
| FND-003 | fixed: replay test admitted_unit_ids_are_the_units_replay_converts_against (FR-398-AC-5) | 66a821ff2 |
