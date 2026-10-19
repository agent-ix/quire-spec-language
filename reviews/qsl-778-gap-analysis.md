---
id: SR-12117
title: "gap-analysis of quire-spec-language PR 950 (TC-830 step 4 against FR-321-AC-5)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c4e5206fb5f3c0f5df3fbe5081e7a67a592963ef; qsl-replay/src/execute/tests/supplied.rs; spec/functional/FR-321-admit-supplied-union-values.md (AC-4, AC-5); spec/test-cases/TC-830-union-values-round-trip-through-v2-literals-and-typed-values-at-any-de.md (step 4)"
review_set: subset
evaluated_revision: "c4e5206fb5f3c0f5df3fbe5081e7a67a592963ef"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-321
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-830
    type: reviews
---

## Summary

Ticket: QSL-778. Planless, diff-scoped gap analysis: the ticket's acceptance
criteria and FR-321-AC-5 mapped to the one new test. Plan completion: not
assessed.

- Ticket AC "the fixture from QSL-503 unchanged": met. `tree_witness` builds
  the same shape as `tree` in `qsl-replay/src/execute/tests/union.rs` (a
  `Leaf(1)` under 9,999 `Branch(1, Some(next))`), over a source-declared
  union so that public `replay` can run it.
- Ticket AC "occurrence bounds 29,998 deny and 29,999 admit": met, through
  public `replay` (29,999 = 10,000 unions + 10,000 integers + 9,999 options).
- Ticket AC "A-1 denies and A admits with the independently enumerated QSV
  event sequence": met. A = 79,996 recomputed from QSV FR-109's event table
  and the locked QSV source.
- Ticket AC "evaluation limits identical and sufficient": met; every run
  uses the same evaluation limits, and the admitting runs reach the call.
- Production code: none changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-321-AC-5 still has no tagged test in the repo. The new test is tagged `#[trace("TC-830")]` only, while TC-830 says "Tag each test `#[trace(\"TC-830\", \"<AC id>\")]`" and the ticket's outcome names FR-321-AC-5. Leaving the AC id off is correct: the test does not check AC-5's last sentence (multi-argument controls keep cumulative helper spend and the original Cancel over union arguments). But that clause has no union-argument test anywhere, so AC-5 stays uncovered in the matrix and nothing records that the clause is missing. Either add a multi-argument union control and tag the pair with FR-321-AC-5, or name the open clause on a ticket. | qsl-replay/src/execute/tests/supplied.rs:771; spec/functional/FR-321-admit-supplied-union-values.md:91 |

## Verdict

PASS on the ticket's acceptance criteria, with one medium recorded. Every
QSL-778 criterion is exercised by the new test with a failable oracle.
FR-321-AC-5 as a whole is not traced, because its multi-argument clause has
no union test (FND-001).
