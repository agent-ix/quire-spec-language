---
id: SR-15458
title: "integrity review of STD-245 FR-323 replay target in FR-070/071/098/116/121 (QSL PR #1212)"
type: SpecReview
analysis: integrity
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
# SR-15458: integrity review of STD-245 FR-323 replay target in FR-070/071/098/116/121 (QSL PR #1212)

## Summary

Ticket: STD-245. Diff reviewed: `git diff origin/main...4fabd79f9` (11 files), read together with
QSpec #364, QSL #1211 and the CG consumer, quire-contract-codegen PR #522 (spec/ir958-witness-selection).

Checks:

- FR-070 and FR-071 carry exactly one FR-323 target member, and FR-071's reader applies FR-323 decode
  rule 1 with its codes and pointers. This is consistent with FR-323.
- CG #522 FR-024-AC-41 and AC-42 require the declaring owner and operation from QSL's validated call
  site, refusal without it, and typed, distinguishable missing/conflicting target refusals. This matches
  FR-121's new OperationSite rule and FR-070's `MissingMember("replay_target")`/`ConflictingTarget`.
- Lead defaults: (1) the two refusal names are sound and distinct, and they match CG AC-42; (2) dropping
  the frame payload's `operation` is sound, because it leaves one carrier per ADR-017 G-2 and CG #522
  forbids an inferred payload operation; (3) the envelope/request mismatch refusal
  (`stale_dependency`/`content-mismatch`) is sound, following the PF-4 clause_node precedent; (4) the
  non-function target refusal (`invalid-value`) is sound but shares its pointer (SR-15460 FND-003).
- ADR-029 PL-7 (open #1210, read only): it replays plugin counterexamples as canonical assignments through
  `replay` (function path only). With FR-098's new non-function refusal and FR-323's "no `arguments` on a
  frame/state-clause request", a plugin counterexample on a frame or state-clause item has no replay path
  and settles `inconclusive`/`replay-refused`. This does not contradict PL-7. It is a scope note for the
  PL-7 owner, not a defect here.
- New ids FR-070-AC-18, FR-071-AC-14, FR-098-AC-21, FR-116-AC-8 and FR-121-AC-23 are unique on main and
  across the 17 other open QSL PRs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: ADR-017 PF-4 now says "The packet, the envelope and the request carry no `selected_function`", and FR-071 and FR-098 name the function selection as the `function` replay target. FR-357 (Inputs line 36, Behavior line 74: "It SHALL resolve `selected_function` in the recompiled package") and FR-358 (Inputs line 87, step 2 line 183: "Resolve `selected_function` ..."), with TC-907 steps 50-51, still name the FR-071 request's `selected_function`. That contradicts the new rule. Rename them to the request's `function` replay target, and state that a non-function target refuses as FR-098 does. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:183-185 |
| FND-002 | low | minor: PF-4's list is still introduced as "Decision, with no wire change:", but the new third bullet changes the wire: the packet drops `selected_function`, carries `frame_operation`, and the payload drops `operation`. "TK-2 implements the first two bullets" leaves the third bullet without a task row. Reword the lead-in and add or point to the task for the third bullet (the code ticket QSL-972). | spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:286-304 |

## Verdict

Not mergeable: FND-001 (major). Otherwise consistent with QSpec FR-323, QSpec #364 and CG #522.

## Dispositions

Round 1, reviewed QSL #1212 at 373ddf489.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | fixed at 373ddf489 (FR-357, FR-358, FR-358-AC-3 and TC-907 name the function replay target) |
| FND-002 | fixed | fixed at 373ddf489 (lead-in reworded; QSL-972 named for the third bullet) |
