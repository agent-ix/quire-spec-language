---
id: SR-17205
title: "Gap analysis of QSL-902 observed temporal replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language PR #1531; git diff 9c7a006bd..qsl-902-code; QSL-902 changes in qsl-replay, qsl-eval and downstream envelope fixtures against FR-331-AC-1 through FR-331-AC-8, FR-071-AC-8 and FR-072-AC-6"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: reviews
---
## Summary

Ticket QSL-902. Plan completion: not assessed. The changed implementation is
owned by FR-331, with request and result wire changes owned by FR-071 and
FR-072. New tests carry tags for FR-331-AC-1, AC-2, AC-3, AC-4, AC-5, AC-6,
AC-8, FR-071-AC-8 and FR-072-AC-6. FR-331-AC-7 is also exercised in the
tagged refusal test. The implementation preserves the full typed evaluation
outcome and settles non-matching or non-decisive outcomes as inconclusive.

The exact `InsufficientData` replay requested by FR-331-AC-8 is not yet
proven end to end: the current test directly checks result settlement, while
the documented `count where` source fixture fails parsing at `where`. Other
AC-8 cases include end-to-end `Incomplete`, mismatched locus and mismatched
cause replays. No unowned changed production behavior or stub was identified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `InsufficientData` branch in FR-331-AC-8 has direct result-constructor evidence only, rather than replay-facade evidence. The wire trace, evaluator dispatch and replay plumbing are therefore not exercised for this outcome. | qsl-replay/src/spine/clause/tests/temporal.rs:343; spec/test-cases/TC-847-an-undefined-letter-fails-a-supplied-trace.md:60-62 |

## Verdict

CONDITIONAL. The traced implementation is requirement-owned and has tests for
the other observed replay cases in scope. Close FND-001 by adding an
end-to-end `InsufficientData` replay once a valid source fixture can compile.

## Coverage

`quire matrix --scope . --format tsv` was started but did not complete within
the review window; the matrix tool offers repository scope rather than a
file-level filter. Criterion bindings above were checked against source trace
tags. `cargo test -p qsl-replay -p qsl-eval` passed, including the new tagged
tests. Semantic review was limited to the changed replay path and criterion
clauses listed in scope. Plan completion: not assessed.
