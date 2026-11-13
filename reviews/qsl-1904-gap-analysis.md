---
id: SR-2700002
title: "QSL-1904 PR 2700 gap-analysis"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1904; origin/main...3a4ac6d192303022c7695b1c61ed16a71adbd04f; FR-380-AC-8 to FR-380-AC-21, TC-1054 rows 15-28, FR-105 reaches_field note; changed Rust under qsl-semantics, qsl-package, qsl-replay, qsl-eval"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1054
    type: reviews
---
# SR-2700002: QSL-1904 PR 2700 gap-analysis

## Summary

Ticket: QSL-1904. Plan completion: not assessed. Scoped AC-to-test
reconciliation over the PR diff by trace-tag search (`#[trace("TC-1054",
"FR-380-AC-N")]`), not a full computed `quoin matrix` run over the whole
repository. Every criterion FR-380-AC-8 to AC-21 has at least one tagged test:
AC-8 to AC-18, AC-20 and AC-21 in
`qsl-package/src/emit/tests/population_field_edge.rs`, AC-19 in
`qsl-eval/tests/it/relationship_reachability.rs`. AC-12 is tagged but
`#[ignore]`d with the reason "QSpec STD-663 and IR-1533: IR's reader does not
yet admit `population` on a `field` member of `reaches_field`", which the
FR-380 Dependencies section records as PENDING; the criterion is not
evidenced until both land.

Every new production line has an owning requirement: the `walked_object`
change and link gate (FR-380 "Field-edge reachability", checker paragraph),
`Node.field_population` and `link_population` (same, link), the lowering of
the population over `edge.owner` and `Member::Field.population` (S4
paragraph), the emitter key (FR-105 note, absent not `null`). No stub, no
placeholder return, no `todo!`.

## Verdict

PASS with one open criterion (AC-12, externally blocked and correctly
ignored with named tickets). Test-strength notes on AC-13, AC-17/18/21 and
AC-19 are recorded in SR-2700001 and not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-380-AC-12 has no running evidence: its only test is `#[ignore]`d pending QSpec STD-663 and IR-1533. The ignore reason names both, and FR-380 records the dependency as PENDING, so this is a tracked open criterion, not a defect in the PR; it must be un-ignored when IR-1533 lands. | qsl-package/src/emit/tests/population_field_edge.rs:328-331 |

## Coverage

- FR-380-AC-8 (examined): tagged `a_reference_field_edge_links_the_bounded_population`.
- FR-380-AC-9 (examined): tagged `an_optional_reference_field_edge_links_the_bounded_population`.
- FR-380-AC-10 (examined): tagged `a_sequence_field_edge_links_the_bounded_population`.
- FR-380-AC-11 (examined): tagged `a_sequence_bounds_supply_no_population`.
- FR-380-AC-12 (examined): tagged `ir_reads_each_field_edge_package_and_selects_its_population`, ignored.
- FR-380-AC-13 (examined): tagged `an_unbounded_field_edge_keeps_its_member_and_node_identity`.
- FR-380-AC-14 (examined): tagged `two_populations_covering_the_edge_type_refuse_ambiguous_name`.
- FR-380-AC-15 (examined): tagged `a_subtype_operand_links_the_edge_types_node`.
- FR-380-AC-16 (examined): tagged `a_population_covering_only_the_operand_type_is_not_linked`.
- FR-380-AC-17 (examined): tagged `an_uncovered_edge_type_adds_no_domain_and_no_refusal`.
- FR-380-AC-18 (examined): tagged `a_walked_field_edge_population_is_a_bounded_domain_of_the_record`.
- FR-380-AC-19 (examined): tagged `a_walked_field_edge_population_refuses_above_its_bound`.
- FR-380-AC-20 (examined): tagged `a_field_edge_and_an_end_share_one_population_node`.
- FR-380-AC-21 (examined): tagged `the_edge_type_not_the_operand_type_names_the_walked_population`.

## Dispositions

Round 1, checked against fix commit fd31b2efc.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | AC-12 waits on QSpec STD-663 and IR-1533, as FR-380 Dependencies records; the ignore reason names both. Un-ignore when IR-1533 lands. |
