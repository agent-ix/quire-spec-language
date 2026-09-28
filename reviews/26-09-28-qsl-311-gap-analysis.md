---
id: SR-771
title: "QSL-311 gap analysis of PR 511 (TC-466 step 3 reaches over a probe invocation and charge denial)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6307caf29145a05eedf251a428f6e40b0a8b069d; qsl-replay/src/spine/clause/tests.rs; spec/test-cases/TC-466-s6a-evaluates-state-clauses.md (unchanged); spec/functional/FR-107-evaluate-state-clauses-at-s6a.md (unchanged); spec/functional/FR-106-admit-snapshots-and-invocations.md (checks 8 and 10, unchanged); spec/tests.md (unchanged); qsl-semantics/src/model/observation/document.rs (check_population_closure, admit_parameters_and_result, unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-466
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: references
---
## Summary

Ticket: QSL-311. PR: quire-spec-language#511 at 6307caf2, base f8b89a9b.
There is no plan bundle, so the scope is the ticket's asks, checked against
TC-466 step 3 as written (TC-466 lines 29-42 and 52-56) and FR-107-AC-3.

1. **Step 3 (a)-(d).** All four are delivered, each over a real
   FR-106-admitted `probe` invocation, with the outcomes TC-466 expects:
   `true`, `false`, `false`, `true`. Step (a)'s exact 5-charge `reaches` log
   is asserted separately. Before this PR, TC-466's Status called step 3(a)
   "built", but that referred to the `NoCycle` charge-log test: a different
   clause, with a 6-charge exhaustive walk. This PR supplies the real 3(a).
2. **Step 3(e).** All five denials are delivered through the existing
   `quire_exact::InjectedDenial` seam. TC-466 expects `Incomplete` with
   `resource_exhausted`. The tests reach `Incomplete` directly from
   `evaluate_clause`. The `resource_exhausted` code is attached later, in
   `run_clause`'s `convert_outcome`, which these tests bypass (FR-107 line
   98-100 fixes that mapping for any denial). The per-case discrimination
   gap is SR-770 FND-001.
3. **Trace tags.** All carry `#[trace("TC-466", "FR-107-AC-3")]`, which is
   correct: FR-107-AC-3 is the one AC row for all of step 3.

Silent-acceptance check (asked for in the review brief): I ran a temporary
probe test, reverted afterwards, on the step-3 fixture. `self` naming an
absent key is correctly refused (`invalid_runtime_input`/
`wrong-role-mapping`, check 9). The `target` parameter naming an absent key
in the complete `config_history` population is **admitted**, and
`ReachesTarget` returns `Completed(false)`. See FND-001.

Underspecified code: none added. The diff is test-only.

Semantic review: done inline with SR-770, because the diff is one test file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Pre-existing production defect, outside this PR's diff, exposed by its fixture. FR-106 check 8 requires refusing any `reference` value that names a key absent from its complete population (`dangling_reference`/`absent-target-in-complete-population`). `check_population_closure` walks only object fields, and `admit_parameters_and_result` admits reference-typed parameters and results through `admit_scalar` without that check. So a `probe` invocation with `target` `zzz` over the complete chain snapshot is admitted, and `ReachesTarget` silently evaluates `Completed(false)` instead of refusing. Measured with `run_config_version_probe(chain, "a", "zzz")`, which gave `Evaluate(Completed(Boolean(false)))`. No TC-465 row covers it. Fix: apply the closure check to parameter and result reference values, and add a TC-465/FR-106-AC-4 case. | qsl-semantics/src/model/observation/document.rs:1259-1292; qsl-semantics/src/model/observation/document.rs:1419-1481; spec/functional/FR-106-admit-snapshots-and-invocations.md:220-223 |
| FND-002 | low | TC-466's `## Status` is stale after this PR. It still says "Only step 3 (b)-(e) remains, tracked by QSL-311". It also calls step 3(a) built on the strength of the `NoCycle` test. With this PR, steps 1-3 are all built, so the Status and the `spec/tests.md` row (still "Planned") should say so. | spec/test-cases/TC-466-s6a-evaluates-state-clauses.md:58-67; spec/tests.md:248 |

## Verdict

The ticket's asks are all delivered with correct trace tags, so the PR
itself passes. FND-002 belongs in this PR's fix round. FND-001 is a
production admission defect outside the diff. It needs its own fix, either
folded into this PR (the fixture that exposes it is here) or as a separate
bug ticket, at the dispatching lead's choice.
