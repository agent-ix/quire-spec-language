---
id: SR-17010
title: "Gap analysis of quire-spec-language PR #1439: QSL-1038 absent-key reads and several covering populations"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@992556734f42370ee77d000f2fdac2152e55c725; PR #1439 (QSL-1038) diff origin/main...HEAD; criteria FR-125-AC-11, FR-125-AC-12, FR-336-AC-1"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1439

## Summary

Ticket: QSL-1038. This is a planless gap analysis against the computed matrix
(`quoin matrix --repo . --json` at the reviewed head) and the source.

Matrix rows:

- FR-125-AC-11 is tagged by
  `the_bound_parameter_reads_as_the_reference_of_the_object_held_under_its_key`.
- FR-125-AC-12 is tagged by
  `a_key_holding_no_object_of_the_type_reads_absent_key`. It was untagged on
  main.
- FR-336-AC-1 is tagged by `an_unbounded_population_stays_in_the_requirement_record`.

All three read `no run evidence`, which is normal for a static matrix.

AC-12, clause by clause, against the test:

| AC-12 clause | Status in the test |
| --- | --- |
| Instances `k`, `t`, `d` in that order | asserted |
| S3 admits the clause with both populations, no ambiguous refusal | admission asserted by `run_formula(...).expect("checked")`; the claim's population list for this setup is not asserted |
| `k` reads as the `Dog` reference | asserted through `exists(o in allInstances<Dog>(animals) : o = x)` being true at `k` |
| Unguarded `always holds(x.age >= 0)` refutes `undefined-evaluation` naming `t` | asserted (`over_binding` is `t`, cause is absent-key) |
| `always holds(true)` is true at `t` and settles | asserted (atom reading and `Holds`) |
| `G` false at `t`, true at `k` and `d`; guarded property `proved` | asserted (atom readings and `Holds`) |
| Animals-only guard false at `d`, vacuous there | asserted |
| Cat-only subject has instances, not `NoInstance` | weakened: only `instances()` is asserted (FND-002) |

AC-11 is complete. Both halves are asserted:

- `Switch` and `Lamp` under `PM`.
- Key `c` listed by `P1` and `P2`, both declaring `Thing`, read through `P2`.

FR-336-AC-1 is still covered, now asserting exactly one population. The new
`two_populations_of_the_over_type_are_both_domains_of_the_clause` carries
TC-846 only, which is right, since it asserts no whole AC.

Reverse gap: `populations_covering` and
`evaluate_state_predicate_at_absent_key` each have a production caller. No
stubs were found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The production record wiring for several populations is untested. `PackageDeclarations` maps each of `claimed.populations()` to its `DomainKey` and passes them to `requirements`. The new TC-846 test calls `requirements` with keys it builds itself and never reads the record the checked graph holds for the two-population clause. A reviewer mutation that keys every population as the first one (so the record has one population domain instead of two) passes all 1991 qsl-semantics tests. Fix: in `two_populations_of_the_over_type_are_both_domains_of_the_clause`, read the clause's record from the checked graph and assert both `config_history` and `archive` are `Population` domains. | qsl-semantics/src/check/mod.rs:3513-3525; qsl-semantics/tests/it/model_subject_bound.rs:137-194 |
| FND-002 | low | AC-12's last sentence ("still has instances ... and is not `NoInstance`") is now asserted only through `instances()`. The PR removed the run-level assertion (`cats.run_formula(..., "always holds(x.age >= 0)", ...)` being `Violated`), which was the evidence that the run itself does not report `NoInstance`. Fix: restore a `run_formula` assertion on the Cat-only subject, for example that it is `Violated` with an undefined evaluation and not `NoInstance`. | qsl-analyze/tests/it/model_instances.rs:922-928 |
| FND-003 | low | AC-12's "S3 admits the clause with both overlapping populations" is asserted for the dogs/animals setup only through admission and through instances from both universes. The claim's `populations()` there is not checked. TC-846 checks it for the ConfigVersion package. Fix: optional, assert `populations()` has two entries for the AC-12 package, or accept the cross-test evidence. | qsl-analyze/tests/it/model_instances.rs:822-895 |

## Verdict

**CONDITIONAL.** AC-11, AC-12 and FR-336-AC-1 are tagged by tests whose
assertions a mutation can fail. Two reviewer mutations (guard exemption
removed; every atom absent-key) were killed by the AC-12 test. One coverage
gap is medium: FND-001, a surviving mutation on the production record path.

## Coverage

- Criteria examined: 3. Tagged and complete: 2 (FR-125-AC-11,
  FR-336-AC-1). Tagged with low gaps: 1 (FR-125-AC-12).
- Mutations: 3 run by the reviewer, 2 killed and 1 survived (FND-001).
- Semantic review: the clause-by-clause test-to-AC check is in the Summary
  above.
- Plan completion: not assessed.

## Dispositions

Round 1. Reviewed at `9c2ae662e392b056a6d1d02acff55577ac1213f6`. The matrix
(`quoin matrix --repo . --json` at this head) reads:

- FR-125-AC-11 tagged by `the_bound_parameter_reads_as_the_reference_of_the_object_held_under_its_key`.
- FR-125-AC-12 tagged by `a_key_holding_no_object_of_the_type_reads_absent_key`.
- FR-336-AC-1 tagged by `an_unbounded_population_stays_in_the_requirement_record` and by qsl-package `the_emitted_record_carries_the_population_domain_and_no_universe`.

Mutation M3 (the production record keys every population as the first) is
now killed by `two_populations_of_the_over_type_are_both_domains_of_the_clause`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
