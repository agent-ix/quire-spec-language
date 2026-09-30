---
id: SR-822
title: "QSL-331 gap analysis of PR 538 (frame envelope consistency, ADR-017 TK-2)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5aee4879c307df1e94dddbe85af5c474b176889d; qsl-replay/src/execute/frame.rs; qsl-replay/src/execute.rs; qsl-replay/src/spine/clause/tests/frame_replay.rs; spec/functional/FR-116-replay-a-frame-counterexample.md; spec/test-cases/TC-515-replay-a-frame-counterexample.md; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md (unchanged; PF-4, G-2, TK-2)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
## Summary

Ticket: QSL-331. PR: quire-spec-language#538 at 5aee4879. There is no plan
bundle. The work item is ADR-017 §6 TK-2, so this check compares TK-2's fix
and exit criteria with FR-116, TC-515 and the tests.

- TK-2 exit: "a frame envelope whose `clause_node` or `occurrence_key`
  differs from the payload refuses `stale_dependency`/`revision-mismatch`
  naming both, before recompiling; TC-515 extended." Met. `check_envelope`
  (frame.rs:390-407) runs at frame.rs:197, before `recompile`. The two new
  TC-515 tests prove the ordering with a source that does not compile and a
  consistent-envelope control that fails at the recompile. This reviewer
  ran two mutants (check moved after the recompile; occurrence branch
  disabled) and the tests killed both.
- TK-2 fix, "the frame envelope's members as PF-4 states; amend FR-116".
  Met. FR-116 Inputs now states PF-4's first bullet word for word (clause
  node, `generated` occurrence, O-09 obligation identity). FR-116 Behavior
  states the second bullet.
- `obligation_identity` is stated but not checked. This is not a gap.
  ADR-017 PF-4 says "TK-2 implements the first two bullets". The first
  bullet defines what the envelope members are (the FR-116 Inputs
  amendment carries it). The second bullet, the refusal, names only
  `clause_node` and `occurrence_key`. G-2 is about two carriers of one
  identity. The frame packet carries the obligation identity once, since
  the payload has no copy, so there is nothing to disagree with. QSL does
  not compute O-09 anywhere in qsl-replay (`obligation_identity` is only
  decoded and carried, witness.rs:903-906). The clause replay does not
  check it either.
- `selected_function` is still not read, as PF-4's third bullet requires.
- Bindings: `an_envelope_clause_node_other_than_the_frame_refuses_before_recompiling`
  and `an_envelope_occurrence_other_than_the_frame_occurrence_refuses_before_recompiling`
  both trace to TC-515 / FR-116-AC-6. Both are correct: each tests what
  its AC half states. The spec/tests.md TC-515 row lists AC-6.
- Every production line added traces to FR-116's new Behavior bullet. No
  underspecified code.

## Verdict

Clean. Every part of TK-2's fix and exit criteria is implemented and
backed by a test that fails when the behaviour is removed or reordered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
