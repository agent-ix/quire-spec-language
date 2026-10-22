---
id: SR-15461
title: "gap analysis of STD-245 ACs and TCs (QSL PR #1212)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4fabd79f984475fc42ba12336433f44d432d6b91; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-116-replay-a-frame-counterexample.md, spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md, spec/test-cases/TC-183-witness-envelope-missing-member-refusal.md, spec/test-cases/TC-185-replay-request-exact-members-round-trip.md, spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md, spec/test-cases/TC-515-replay-a-frame-counterexample.md, spec/test-cases/TC-516-locate-a-function-call-site.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-15461: gap analysis of STD-245 ACs and TCs (QSL PR #1212)

## Summary

Ticket: STD-245. This is a spec-only PR. Plan completion: not assessed. AC-to-TC trace: FR-070-AC-18 is
traced by TC-183 step 6, FR-071-AC-14 by TC-185 step 6, FR-098-AC-21 by TC-444 step 9, FR-116-AC-8 by TC-515
step 6, and FR-121-AC-23 by TC-516 step 23. Each TC scope line names its new AC. Code: QSL OperationSite
(qsl-replay/src/call_site.rs) has no `frame_operation` yet. That is the code ticket QSL-972, which the
STD-245 planner comment names. `make check-spec-validation` with QSPEC_DIR at #364's head: exit 0. The
redefinition case without an AC is recorded in SR-15460 FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean apart from SR-15460 FND-002.
