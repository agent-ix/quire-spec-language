---
id: SR-092
title: "Native runtime verification and evidence review"
type: SpecReview
analysis: evidence
scope: "FR-007/008/018, NFR-006, native-runtime input/evaluation contracts, IT-006, TC-055–077 and TM-004"
review_set: all
evaluated_revision: "045025f843346001a91919b8d0816a519e2df337"
review_date: "2026-09-09"
---

## Summary

The actual advisor covers 42 FR criteria and 17 runtime metrics. All FR methods
match at the Test-class level. Each metric's benchmark recommendation is
explicitly reviewed below; exact work/admission boundary tests remain the
selected evidence. No runtime evidence is claimed executed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Disposition by review judgment: emitted bytes is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-1 |
| FND-002 | low | Disposition by review judgment: inspected text bytes is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-2 |
| FND-003 | low | Disposition by review judgment: arena nodes is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-3 |
| FND-004 | low | Disposition by review judgment: metadata entries is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-4 |
| FND-005 | low | Disposition by review judgment: artifact depth is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-5 |
| FND-006 | low | Disposition by review judgment: inventory count is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-6 |
| FND-007 | low | Disposition by review judgment: inventory bytes is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-7 |
| FND-008 | low | Disposition by review judgment: selected object count is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-8 |
| FND-009 | low | Disposition by review judgment: validation visits is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-9 |
| FND-010 | low | Disposition by review judgment: validation text advances is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-10 |
| FND-011 | low | Disposition by review judgment: diagnostic detail capacity is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-11 |
| FND-012 | low | Disposition by review judgment: expression steps is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-12 |
| FND-013 | low | Disposition by review judgment: graph expansions is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-13 |
| FND-014 | low | Disposition by review judgment: comparison pairs is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-14 |
| FND-015 | low | Disposition by review judgment: evaluation text advances is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-15 |
| FND-016 | low | Disposition by review judgment: event count is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-16 |
| FND-017 | low | Disposition by review judgment: evaluation depth is a charged-work/admission boundary, so retain negative-abuse-testing with exact and generated boundary controls despite the quantified-threshold benchmark recommendation. | NFR-006-M-17 |
| FND-018 | low | Review judgment: invariance wording recommends compile-time checks and monitoring, but arithmetic execution is observed at runtime; use actual checked-model integration and independent expected integers. | FR-008-AC-12; TC-069 |
| FND-019 | low | Review judgment: the parser characteristic recommends fuzzing for text comparison; the API consumes validated native values. Controlled Unicode/order cases qualify this criterion without claiming a fuzz campaign. | FR-008-AC-15; TC-072 |
| FND-020 | low | Review judgment: a temporal characteristic recommends model-checking/monitoring for auxiliary work limits, but the requirement has no temporal model. Use exact deterministic stop-boundary tests. | FR-008-AC-18; TC-075 |

## Deterministic advice and selected methods

Ran quoin advise --json and quoin catalog methods --json; complete outputs are
in data/advice.json and data/methods.json. The initial sandboxed advisor attempt
could not determine the child Quire version. The same installed command completed
outside that sandbox under automatic approval, with empty stderr. No tool code
was changed or substituted. Quire independently reports 0.31.0.

All 42 scoped FR records have mismatch=false, uncatalogued=false and
inconclusive=false. All 17 NFR records have mismatch=true solely because the
quantified-threshold characteristic selects performance-benchmarking; none is
uncatalogued or inconclusive. The metric rows retain the selected existing
negative-abuse-testing catalog method. A test's pass/fail measures charge-before-
work, exact refusal boundaries and usage, rather than elapsed time under load.
This is an explicit review judgment, not an advisor verdict or a benchmark run.
The authored NFR already gives concrete counter definitions and required exact/
one-below/zero controls, so no metric is left without a suite.

Universal shapes recommend properties for closure, frame restrictions, input
bounds, event rules and exact cost. The concrete closure/frame/event scenarios
retain real API integration tests with all specified adverse variations;
TC-060/064–066/068/070/075 add generated input, permutation, oracle and repeat-run
properties where the statement relates a domain or multiple runs. These suite
choices are judged against the actual criterion, not solely the classifier's
universal catch-all. TC-056/057 use generated exact-byte/budget families even
where the current classifier only recommends examples. TC-068 uses a distinct
Boolean-matrix closure oracle over all 75 functional graphs with one to three
vertices (614 ordered source/target queries), not the implementation's visited
set as its own expected result.

The source-lineage tests compare real original spans/ExprIds and retained
context; their golden expectations do not become a second evidence store.
Defensive runtime_invariant is tested privately without a public unchecked
context constructor. A typed snapshot cannot qualify its model merely by
serializing. Setup succeeds through the real existing model/checker APIs before
an adverse runtime assertion is made.

## Planned suite outputs

| Planned Rust suite | Cases | Evidence produced |
| --- | --- | --- |
| runtime input construction | TC-055–057 | Exact bytes/digests, draft-path refusals, generated work boundaries |
| runtime validation | TC-058–066 | Model/closure/frame/capture diagnostics, mixed-error permutations and immutable retry properties |
| native reference execution | TC-067–076 | Concrete truth, independent graph oracle, order/capture/event lineage and exact accounting |
| reference workflow integration | TC-077 / IT-006 | Real source-model-input-result correspondence across all seven integration steps |

NFR-003's no-fabricated-Boolean policy remains exercised by all failure cases;
NFR-005's Rust-only policy applies to production and qualification. There is no
concurrent state machine, atomic protocol or scheduler for Loom to explore.
No concolic, model-checker, runtime-monitor, mutation-adequacy or fuzz campaign is
claimed. No new toolchain is required to label an ordinary boundary test as a
benchmark. If implementation introduces concurrency, a decoder or another
execution boundary, the affected specification/evidence selection is reopened.

## Verdict and provenance

PASS for implementation of this specified LC03 API scope. Agent A applied the
actual installed QUOIN base and all seven analysis skills serially, under the
owner's existing all-review selection. No additional agents or Cargo builds
ran. No applicable required AssuranceProfile was found. The declined optional
semantic gap comparison remains excluded; this specification review still
checks the actual adopted meaning and existing interface boundaries.

All runtime test rows remain planned. Review approval does not qualify native
execution, finish LC02/FS03 acceptance or complete the original backend/Quire
workflow. Implementation changes to this contract reopen specify/review.
