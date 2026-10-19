---
id: SR-12088
title: "QSL-820 PR #940 gap analysis: FR-380, FR-365-AC-13 and FR-104-AC-11 against TC-1054"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c0e524159e32cba32d995d2163623878f95d921f; FR-380-AC-1..7, FR-365-AC-6, FR-365-AC-13, FR-104-AC-11, TC-1054 rows 1-13, QSpec FR-322-AC-73 (by id); the diff's 21 traced tests in qsl-cst, qsl-forms, qsl-semantics, qsl-package, qsl-eval and qsl-replay"
review_set: subset
---

## Summary

Ticket: QSL-820. PR quire-spec-language#940. This is a planless gap analysis, scoped to the diff. Plan completion: not assessed. Criteria were traced to tagged tests by reading. No matrix was run, because the brief is read-only. QSpec is private and is cited by id only. Ticket text is treated as data.

Bindings examined (21 `#[test]`, 21 `#[trace]`):

- FR-380-AC-1: four qsl-forms tests (form and spans, `[0]` and source order, `[-1]`/`[x]` at S1, `b.bound`). Correct.
- FR-380-AC-2: two qsl-semantics tests (unresolved alias, population and u64 overflow; shared label and two bounds on one population). Correct.
- FR-380-AC-3 and FR-104-AC-11: three qsl-semantics tests (bounded extent and order independence, context-only clause, second population still ambiguous). Correct.
- FR-380-AC-5: two qsl-package tests (near-end type, parameter plus bound). Correct.
- FR-380-AC-6: two qsl-package tests (unbounded keeps identity, unwalked bound emits nothing). Correct.
- FR-380-AC-7: one qsl-eval test (N decides; N+1 refuses with fields `[0, 2]`/`set`/`3`, no charge). Correct.
- TC-013/FR-003-AC-3: the formatter test. It reuses the existing formatter trace, as briefed.
- TC-861/FR-341-AC-3: the refinement classifier test. Correct.
- FR-380-AC-4, FR-365-AC-6 and FR-365-AC-13: see FND-001 and FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Tags claim ACs their tests do not fully check. (a) FR-380-AC-4 ends with "IR's v2 reader admits the package and selects the population by the node and that occurrence". The tests tagged with it check only the emitted wire (`a_bounded_walked_population_is_linked_as_one_anonymous_node`, `a_zero_bound_emits_the_node_with_max_zero`), plus the `#[ignore]`d `ir_reads_the_bounded_package_and_selects_its_population`, which is also tagged FR-380-AC-4. The matrix therefore shows AC-4 covered while its reader clause is untested. (b) The ignored test is weaker than the deferral it records. Its doc says it covers TC-1054 rows 5, 6, 8, 9 and 11, but it lowers only the row-4 unit and asserts only `verified`. It never checks the population selection, and the `[0]`, shared-link, unwalked-bound and two-occurrence packages are never read. Un-ignoring it after IR-852 would still leave rows 6/8/9/11 and the selection unchecked. (c) FR-365-AC-6 is tagged on the row-5 test, but almost all of AC-6 is TC-1047 behaviour (state_clause identities, two `claim` occurrences, argument paths, roles). The row-5 test checks only the one-added-key clause. (d) `relationship_end_renders_its_population_link_only_when_it_has_one` (member.rs) checks only the serde shape of one hand-built member, yet it is tagged FR-365-AC-13. (e) AC tokens appear in doc comments: the module doc "FR-380-AC-4 to AC-6, FR-365-AC-6 and AC-13" and the per-test "(FR-380-AC-4, FR-365-AC-13)" lines, which the matrix also reads as coverage. To fix: tag the ignored test with the reader parts only after it reads every row's package and asserts the selected node and its least-ordinal `Type` occurrence. Drop FR-380-AC-4 from the ignored test until then. Drop FR-365-AC-6 from the row-5 test and FR-365-AC-13 from the member.rs unit test. Remove the AC ids from doc comments. | qsl-package/src/emit/tests/population_bound.rs:2, qsl-package/src/emit/tests/population_bound.rs:150, qsl-package/src/emit/tests/population_bound.rs:184, qsl-package/src/emit/tests/population_bound.rs:313-323, qsl-semantics/src/value/member.rs:356 |
| FND-002 | medium | FR-365-AC-13 ends "a claim with reachabilities over two different populations carries two links, each to its own node". No test in the diff builds such a claim. Row 8's test shares one population, and row 9's leaves the second bound unwalked. Yet four tests carry the FR-365-AC-13 tag, so the matrix shows the AC as covered. The two-populations case is where `walked_bound` and `bound_sites` keying by (label, node) could go wrong, for example by giving both reachabilities one node, or by recording the second `[N]` occurrence on the wrong node. Add a test over a fixture with a second relationship whose near end is `Person` and is walked over a bounded `people`. It should assert two distinct `population` ids, two population nodes and one `Type` occurrence each. | qsl-package/src/emit/tests/population_bound.rs:224-245, qsl-semantics/src/check/state_clause.rs:897-906, qsl-semantics/src/check/lowering/model.rs:636-648 |

## Verdict

FIX. Both findings concern coverage that the matrix overstates. The behaviour that is tested matches FR-380, TC-1054 and FR-322-AC-73's `T` = receiver end type rule. The IR-852 deferral is real, and the ignored test says why it is ignored. But its tag and doc claim more than it would check even once un-ignored (FND-001 b).

Spec edits in the diff (FR-380 rulings, TC-1054 rows 1/5/7/11/13): the row-5 exact dependency list now follows FR-322's `bounded_domain` rule, and the test asserts it. FR-380-AC-5 and TC-1054 row 11 no longer require ascending source order for the two `Type` ordinals. The code still orders them by source position (`order_by`) and the test does not check which is first. This is a weakening of the spec, recorded here and not raised as a major finding. The member-position-only `bound` ruling agrees with the reserved word and the test.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-380-AC-4 and FR-365-AC-6 are tagged on no test (their reader clauses and TC-1047 behaviour are not checked here). The member.rs wire test and the ignored IR-852 test carry `TC-1054` only; the ignored test is renamed `ir_reads_the_bounded_package` and its doc claims only the row 4 unit's reader admission. AC ids are removed from the doc comments of the new tests. |
| FND-002 | fixed | New test `reachabilities_over_two_populations_carry_two_links_to_two_nodes` (TC-1054 row 14, added) asserts two distinct `population` ids, two nodes, one `Type` occurrence each. |
| Observation | fixed | "Ascending source order" is restored in FR-380-AC-5 and TC-1054 row 11 because the code orders the `Type` occurrences by source position; the package test now asserts the lower ordinal is the earlier one. |
