---
id: SR-7817
title: "QSL-52 PR 746 manual acceptance-to-tests gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@99da36f7ac0ca455e5e3bc7bb8c46c6b37bd59b9 (base 423dddc9470cecdc6036f3dde4034af40e057a37); agent-ix/quire-specification@34384b8ebb917e3b9545b15f91704ad75152bf95; QSpec TC-196 D01; QSpec FR-151-AC-8; qsl-eval/tests/it/dispatch_calls.rs"
review_set: subset
---

## Summary

Requalified the test at `99da36f7ac0ca455e5e3bc7bb8c46c6b37bd59b9` against `423dddc9470cecdc6036f3dde4034af40e057a37`, and checked D01 and FR-151-AC-8 at QSpec main `34384b8ebb917e3b9545b15f91704ad75152bf95`. The trace binding remains correct, and the corrected D receiver case now covers the A-rooted runtime scenario and exact work boundaries.

## Verdict

**PASS.** FND-001 is fixed: the D receiver is evaluated against the A-context invariant and D01's exact work-6 completion and work-5/work-1 incomplete boundaries. No further acceptance-to-test gaps were identified in the scoped criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The D01 acceptance scenario is only partially bound: the D receiver uses the B-rooted graph and asserts work 5 with limits 4 and 0, leaving the required A-rooted work-6 completion and work-5/work-1 boundaries uncovered. | QSpec TC-196 D01; QSpec FR-151-AC-8; qsl-eval/tests/it/dispatch_calls.rs:1970 |

## Examined scope

```yaml
scope:
  - {id: QSL-52, path: "Linear QSL-52", role: examined, excerpt: "Add a test that exercises D01's runtime behaviour as TC-196 states it. Trace it to TC-196 and the FR-151 AC that D01 names. If the runtime half needs code that isn't there yet, record what's missing on this ticket."}
  - {id: QSpec-TC-196-D01, path: "../quire-specification/spec/test-cases/TC-196-conformance-redefinition-and-dispatch.md", role: examined, excerpt: "| D01 | G and S2; link; then under `ModelNormalizationLimitsV1` with `dispatch_candidates: 8`, then `7`; evaluate `I` with `self` = object `d1` of type `D` under `ScalarLimitsV1` with every counter unlimited except `work_units: 6`, then `5`, then `1`; evaluate `I` for `c1` of type `C` | for `(A, A/size)`, per subtype `C`, `A`, `D`, `B` in that order: one `dispatch.subtype`, then one `dispatch.candidate` for each of `A/size` and `B/size` (work `f` = 2 for `C`, 1 for `A`, 5 for `D`, 2 for `B`), then for `D` and `B`, which each have two applicable candidates, one `dispatch.dominance` of"}
  - {id: QSpec-FR-151-AC-8, path: "../quire-specification/spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md", role: examined, excerpt: "Dispatch charges at their exact bound complete and the one-less run is incomplete at the named charge point."}
  - {id: "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit", path: qsl-eval/tests/it/dispatch_calls.rs, role: examined, excerpt: "#[trace(\"QSpec-TC-196\", \"QSpec-FR-151-AC-8\")]\n#[test]\nfn d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit() {"}
  - {id: "qsl-eval/tests/it/main.rs#dispatch_calls", path: qsl-eval/tests/it/main.rs, role: context_only, excerpt: "mod dispatch_calls;"}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: QSpec-TC-196-D01
    related_ids: [QSpec-FR-151-AC-8, "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit"]
    path: qsl-eval/tests/it/dispatch_calls.rs
    lines: "1970-1979"
    excerpt: |-
      | D01 | G and S2; link; then under `ModelNormalizationLimitsV1` with `dispatch_candidates: 8`, then `7`; evaluate `I` with `self` = object `d1` of type `D` under `ScalarLimitsV1` with every counter unlimited except `work_units: 6`, then `5`, then `1`; evaluate `I` for `c1` of type `C` | for `(A, A/size)`, per subtype `C`, `A`, `D`, `B` in that order: one `dispatch.subtype`, then one `dispatch.candidate` for each of `A/size` and `B/size` (work `f` = 2 for `C`, 1 for `A`, 5 for `D`, 2 for `B`), then for `D` and `B`, which each have two applicable candidates, one `dispatch.dominance` of
    finding: "D01 specifies evaluation of invariant I for receiver D through the `(A, A/size)` family, with the body selected as B/size. The test checks D through `graph_b`, built from a B/size root (dispatch_calls.rs:1925-1936, 1970-1979), including its invariant (1995-2004) and work probes (2048-2084). It reports five units for D and exercises limits four and zero. The specified A-rooted run completes at six; the one-less limit five is incomplete at ordering.result-retain, and limit one is incomplete at dispatch.select with zero consumed and next charge two. The C case uses graph_a, but that does not establish the missing D-through-A-root scenario. The current trace therefore does not provide the required runtime evidence for D01/FR-151-AC-8. Run D and its three work-limit observations against graph_a, asserting the specified results and charge accounting."
bindings:
  - {test_id: "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit", ac_id: QSpec-TC-196, trace: correct}
  - {test_id: "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit", ac_id: QSpec-FR-151-AC-8, trace: correct}
```

## Evidence and limits

Trace binding is syntactically correct under the repository's QSpec alias convention (`spec/tests.md`); the local `spec/test-cases/TC-196-duplicate-backend-registration-refused.md` is a different QSL test case and is not the cited acceptance source. QSpec main at the cited commit retains the D01 A-rooted case, its work-6 completion and 5/1 incomplete runs, and the FR-151-AC-8 exact-bound/one-less contract. PR #747's five changed paths are disjoint from this PR's sole changed path. No tests, builds or source edits were performed. The strict matrix scan previously produced no output and was stopped; this requalification directly inspected the unchanged test tag and outcomes. No tracker comment was created, per instruction; there is no comment id or read-back. The `has-review` label was already present.
The fixed test declares all three invariants in `a_type` context, invokes the real checked-package `evaluate_clause` path with explicitly constructed `AdmittedObservations`, and asserts D's work-6 completion, work-5 `OrderingResultRetain` failure, and work-1 `DispatchSelect` failure with zero consumed and next charge 2. Its comment explicitly limits the claim to runtime dispatch, not FR-106 document admission. The supplied receipt reports the focused test passed at this SHA; no tests/builds were run in this requalification. Tracker comment was withheld because the QSL-52 issue explicitly says replies sync to GitHub #175.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | Requalified at 5253c4fcdd54db296eeb390ea5fd9311d0703a17 against 362a7b3398a79e86a1fe0af22960afc93f120363: the D receiver still uses graph_b rooted at B.size and the assertions remain work 5 with limits 4 and 0. D01's A-rooted work-6 completion and work-5/work-1 incomplete boundaries are still absent. |
| FND-001 | fixed 99da36f7ac0ca455e5e3bc7bb8c46c6b37bd59b9 | The test sets invariant context to `a_type`, evaluates D through `evaluate_clause`, and asserts D01's exact 6/5/1 results and charge points. |
