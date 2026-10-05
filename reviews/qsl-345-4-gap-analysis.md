---
id: SR-1311
title: "Gap analysis of quire-spec-language PR #637: FR-121-AC-18 to AC-22 and the DomainKey subject"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d764bb72d0cb411f62b266d8455aa8e84b4602a9; PR #637 diff against origin/main c8f0c2818; FR-121-AC-18 to AC-22, TC-516 steps 18-22, ADR-012 §15.4/§15.7, FR-097-AC-1 and TC-436 (context), trace tags in qsl-replay/src/spine/clause/tests/call_site.rs and qsl-foundation/src/bound.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #637

## Summary

Ticket: QSL-345 (item 4). PR: quire-spec-language#637.

AC to test trace:
- FR-121-AC-18 -> `call_site_keys_a_state_field_under_its_declaring_type_by_name_ordinal`.
  The ordinals are hard-coded (`[0]` parent, `[1]` versionNumber, `[0]` alpha,
  `[1]` zeta), against declaration orders `versionNumber, parent` and
  `zeta, alpha`. The nodes come from the graph's own lowering. All four refusals
  are checked for variant, selection, package and code. The oracles are
  independent of the implementation.
- FR-121-AC-19 -> `call_site_keys_a_field_of_a_type_no_clause_names`. It asserts
  that the graph holds no `Sub` node, then that the key equals the minted key.
- FR-121-AC-20 -> `a_field_key_and_a_population_key_with_one_ordinal_are_distinct`.
  The population oracle is a literal `Population{config_version, 0}`, checked
  against every recorded domain.
- FR-121-AC-21 -> `call_site_keys_a_population_as_its_requirement_records_do`.
  The ordinals are literals (`config_history` 1, `aaa_subs` 0, inverted against
  source order), and the record key set is compared for equality. `Sub`'s node
  is taken from `field_domain`, which AC-18 and AC-19 already pin to the
  lowering's node.
- FR-121-AC-22 -> `call_site_refuses_an_unknown_population_paired_with_its_package`.
- AC renumbering: AC-18..22 follow main's AC-17. TC-516 steps 18-22, its scope
  line and the tests.md row all agree, and no AC id is duplicated or skipped.
- Every new production item has an owning requirement: `FieldName`,
  `PopulationName`, `FieldSite`, `PopulationSite`, `UnknownField` and
  `UnknownPopulation` belong to FR-121; `CheckedGraph::field_domain` and
  `population_domain` to FR-121 via ADR-012 §15.4/§15.7; the `DomainKey` enum to
  ADR-014 §4.

Nested field paths: ADR-012 §15.4 and the `FieldSite` doc define the key of a
domain inside a field's type as `[ordinal, ...child-index path]`, as the
2026-10-01 ruling worded it. No FR-121 AC claims it, and FR-121 Behavior promises
only "the one-element path". That makes it an honest definitional statement, not
an unbacked AC, and the coder disclosed it on QSL-345 on 2026-10-01. It is still
a gap for the consumer, so FND-001 records it.

## Verdict

Every AC is traced and tested with independent oracles. There are two low gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | ADR-012 §15.4 and `FieldSite`'s doc tell a caller to key a domain inside a field's type by appending the child-index path itself. Nothing in QSL computes or tests such a key, and `FieldName` cannot select one, so CG would derive type positions on its own. That is the duplication the item-4 ruling ("so CG never computes ordinals") set out to avoid. It is honest as stated, but it has no owner: name it in the item-3 ticket (the replay domain check, the first consumer), or add a one-line note in FR-121 that nested field keys are not yet returned. | qsl-replay/src/call_site.rs:188-198; spec/decisions/ADR-012-semantic-family-extension-contracts.md:1299-1315 |
| FND-002 | low | `a_population_key_never_equals_a_node_key` is tagged `#[trace("TC-436", "FR-097-AC-1")]`, but FR-097-AC-1 says only "`DomainKey`s order by node, then path". It says nothing about a population key, distinctness from a node key, `Node < Population` order, or the Display form the test asserts. Retag it once FR-097-AC-1 is amended (SR-1312 FND-003), or tag it to FR-121-AC-20. | qsl-foundation/src/bound.rs:297-314 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bea72a04bef9bba0c8be31e03f2cee763bd04986 (the FR-121 Status and the spec.md row now name nested field keys `[ordinal, ...child path]` as remaining work under QSL-345, which gives the gap an owner) |
| FND-002 | fixed | bea72a04bef9bba0c8be31e03f2cee763bd04986 (FR-097-AC-1 now states the variant order, so the existing tag is correct and needed no retag; the new `node_keys_sort_before_population_keys_which_order_by_member_then_ordinal` is traced to the same AC) |
