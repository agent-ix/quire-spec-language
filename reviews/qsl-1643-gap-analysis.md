---
id: SR-20261104130233
title: 'Gap analysis of PR #2295: window aggregate emission, gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@1681cae5f; PR #2295 diff: FR-337-AC-15, FR-194, TC-1267 step 8 against qsl-package/src/emit/tests/temporal_aggregate.rs and qsl-semantics/src/check/temporal_clause/tests.rs; ticket QSL-1643'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1643. Planless, scoped to the PR diff. Plan completion: not assessed. Each clause of FR-337-AC-15 and of the new FR-194 family bullet was traced to a tagged test.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-337-AC-15 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:519 | examined |
| FR-194 value/threshold family bullet | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:94-99 | examined |
| TC-1267 step 8 | spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md:107-118 | examined |
| FR-194-AC-7 | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:183 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-194-AC-7's "threshold `5 m` against a time value, `ill_typed`/`unit-mismatch` at `5 m`" has no test that can fail on the tag: `wrong_threshold_dimension` asserts only `Code::IllTyped`, and the code now yields `type-mismatch` (code-review FND-001). Fix: assert the cause tag and the threshold locus. | qsl-semantics/src/check/temporal_clause/tests.rs:551-555 |
| FND-002 | low | AC-15's "for each kind its bindings and literals are those of the QSpec accepted vector over the same inputs" is not met for AG-05. The test uses a different input (`>= 1`) and rewrites the expected vector, so the AC holds only through that edit. Fix: once code-review FND-002 lands, test `>= 1/2` against the unmodified vector; otherwise state the AG-05 exception in AC-15 itself, not only in TC-1267. | qsl-package/src/emit/tests/temporal_aggregate.rs:170-175 |

## Verdict

Conditional. Every other clause of AC-15 has a tagged test that can fail: the vector shapes, the references reaching `over`, the spans, the omitted versus explicit `min_count` identity, the eight mutations, the `past[0 ms, 300 s]` lowering to 0 and 300000 of ms (the semantics test), and the duration refusal with no package. The family refusals (a Boolean value, a non-literal integer on `fraction`) are tested in `aggregate_atoms_are_checked_and_admitted`. No production code lacks an owning requirement.
