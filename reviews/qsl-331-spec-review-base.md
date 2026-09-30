---
id: SR-823
title: "QSL-331 spec review of PR 538's spec edits (FR-116, TC-515, tests.md)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5aee4879c307df1e94dddbe85af5c474b176889d; spec/functional/FR-116-replay-a-frame-counterexample.md; spec/test-cases/TC-515-replay-a-frame-counterexample.md; spec/tests.md; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md (unchanged; PF-4, TK-2); spec/functional/FR-098-execute-a-replay-request.md (unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-515
    type: reviews
---
## Summary

Ticket: QSL-331. PR: quire-spec-language#538 at 5aee4879.

What the edits get right:

- The FR-116 Inputs bullet matches ADR-017 PF-4's first bullet.
- The new Behavior bullet is a well-formed EARS unwanted-behaviour
  statement, and it matches PF-4's second bullet and the code.
- The coder's rewording "before it reads the payload" to "before it
  resolves the payload's operation" is accurate and needed. The envelope
  check now reads the payload's `frame` and `occurrence` before the
  recompile, so the old wording would be false. The first thing read after
  the recompile is `operation_name(&payload.operation)` (frame.rs:209).
- AC-6 is atomic enough: two halves of one refusal rule, plus the ordering
  proof. It is testable, and it states its oracle: the source does not
  compile and a consistent-envelope control fails at the recompile.
- The TC-515 step 3 procedure and expected results match the tests. The
  tests.md row lists AC-6.
- The spec/tests.md edit merges cleanly with main bad4944c: different
  hunks.

## Verdict

Sound. Two low findings: which refusal wins between request decode and
the envelope check is not stated, and the status lines do not name the
ticket that added AC-6.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-116 Behavior orders the envelope check only "before it recompiles". It does not say whether a request that fails FR-098's decode rules outranks an inconsistent envelope. The code decodes first (frame.rs:196-197), and the `replay_frame` doc says so, but two implementers of the FR could order these refusals differently. Fix: add "after the request decodes" (or name the FR-098 rules that come first) to the bullet. | spec/functional/FR-116-replay-a-frame-counterexample.md:72-75 |
| FND-002 | low | FR-116 `## Status` still reads "Implemented by QSL-301", and the spec/tests.md TC-515 row status cites only QSL-301. AC-6 was specified and implemented by QSL-331, and neighbouring rows cite each contributing ticket (for example "QSL-279, QSL-312"). Fix: add QSL-331 to both. | spec/functional/FR-116-replay-a-frame-counterexample.md:135; spec/tests.md:267 |
