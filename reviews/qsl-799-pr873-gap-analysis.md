---
id: SR-11271
title: "Gap analysis of quire-spec-language PR #873: TC-453 population variants"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7b0e0f7eeb288260e338f6e693b1bee18bea00e6; src/population_fixtures.rs, qsl-package/src/emit.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #873

## Summary

Ticket: QSL-799. Manual check of acceptance criteria against tests, scoped to
the PR diff. The oracle is the ticket's acceptance criteria and Contract IR
FR-053-AC-1, FR-053-AC-2 and TC-453 steps 1, 3 and 5. No `quoin matrix` run:
the PR adds no spec artifacts. Trace tags were resolved against this repo's
`spec/`.

Examined:
- QSL-799 AC "four current cases stay byte-for-byte": same `TypeForm` path;
  the existing tests assert kinds, roles and maxima per case (examined)
- QSL-799 AC "each variant emitted by the real emitter and admitted by the IR
  public reader, tests assert admission and node/type/bound":
  `two_same_type_bindings_and_a_second_object_type_are_admitted`,
  `a_changed_maximum_regenerates_an_admitted_package` and
  `a_retargeted_parameter_is_rederived_and_admitted` assert typed
  `CheckedPackageV2` fields, not constants (examined, not vacuous)
- QSL-799 AC "IR-760's TC-453 steps 1, 3, 5 can consume the files":
  `variants_are_deterministic_and_written_beside_the_cases` and the extended
  `writes_every_case_into_a_new_directory` (examined)
- FR-053-AC-1, FR-053-AC-2, TC-453 steps 1, 3, 5 (context_only)
- The new tests' `#[trace]` tags (examined, finding)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The four new tests carry `#[trace("FR-094", "TC-453")]`. In this repo `TC-453` is QSL's own `TC-453-exploration-orders-successors-canonically-and-keys-states-by-jcs-bytes` (FR-101). The tag binds these fixture tests to an unrelated test case and claims it verified. TC-453 here means Contract IR's TC-453, which is not a QSL artifact. Fix: drop `TC-453` from the tags (name Contract IR TC-453 in the doc comment if wanted) and tag the QSL criterion these fixtures serve, e.g. `FR-093-AC-19`/`TC-416` like the sibling tests, or the FR-094 AC that applies, instead of the bare FR. | src/population_fixtures.rs:792, src/population_fixtures.rs:814, src/population_fixtures.rs:838, src/population_fixtures.rs:874 |

## Verdict

One blocking finding: wrong trace tags. The tests themselves are real and
cover the ticket's ACs. No test was weakened.
