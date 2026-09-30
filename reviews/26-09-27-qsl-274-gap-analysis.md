---
id: SR-759
title: "QSL-274 gap analysis of PR 496 (FR-120, FR-101 amendment; TC-471..473)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md; spec/test-cases/TC-472-invariant-violations-are-recorded-and-undecided-expansions-stop.md; spec/test-cases/TC-473-model-effects-are-trace-data-and-ambient-reads-refuse.md; spec/tests.md; QSpec origin/main FR-181 (AC-3 to AC-6)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-471
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-472
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-473
    type: reviews
---
## Summary

Ticket: QSL-274. PR: quire-spec-language#496. Spec-only, so this
analysis traces behaviour to ACs and ACs to TCs. No code exists yet. All ten
ACs map to one TC step each. The `spec/tests.md` rows and the coverage
section match the TC scopes. Every TC step names its inputs and expected
outputs, except the two steps in FND-003. The gaps are in FR-120 behaviour
that no AC covers.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Several engine-amendment behaviours have no AC or TC: `Outcome::Stopped` with `runtime_invariant` and its InternalFailure category; `StopReason::Stopped` in sampling; `ReplayError::Stopped`; and the Stopped frontier order ("the state whose expansion stopped, then the queue"). TC-472 step 2 checks only a one-entry frontier `[s0]`, so it cannot tell that order from others. FR-101's TC-453 to TC-455 do not cover the amendment. Fix: add ACs, in FR-101 if FND-002 of SR-758 moves the amendment there. Use a test `TransitionSystem` that stops at depth 1 with each cause, samples into the stop, and replays it. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:138-153; spec/test-cases/TC-472-invariant-violations-are-recorded-and-undecided-expansions-stop.md:57-61 |
| FND-002 | medium | Contract findings and results are specified but never tested. No AC has a `pre` or `post` clause that is `Undefined` or `Refused`, so `ContractUndetermined`, its candidate-digest field and the finding order are untested. The finding order is invariants first, then transition bytes, then candidate key, then clause name. No fixture operation declares a result, so the result domain order, "the first such result" and `StepEffect`'s result are untested. Fix: add an operation with a `Boolean` result and a postcondition on it. Add a clause that is `Undefined` for one argument, for example an unguarded `deref(value(self.next))`. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:255-258,304-316,338-341,366 |
| FND-003 | low | Two TC-473 steps are not concrete. Step 1 samples "with seeds until the traces include" the three step kinds. It names no seed and no bound, so the test's loop is open. Step 2 needs a process whose working directory is an empty temporary directory. `std::env::set_current_dir` inside the threaded test harness is racy, and the TC names no subprocess mechanism. Fix: pin the seeds, or drive `successors` and `replay` directly. For step 2, name a child-process helper. | spec/test-cases/TC-473-model-effects-are-trace-data-and-ambient-reads-refuse.md:24-33 |
| FND-004 | low | Two admission branches are untested. A universe population that is no population of the package (`invalid_runtime_input`/`wrong-role-mapping`, FR-120:163-164) has no AC. The anchor check is tested only with `handler`. Fix: add one row each to AC-7 and TC-472 step 3. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:163-165,407 |

## Verdict

Request changes. Fix FND-001 and FND-002 in this PR. FND-003 and FND-004
are small TC edits. Every AC is testable once SR-758 FND-001 (the QSL-289
dependency) is resolved.

## Dispositions

Round 2.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | FR-101-AC-12 to AC-14 and TC-474 steps 1 to 6 cover `Stopped` with both causes and their categories, a two-entry frontier `[<1>, <2>]`, `StopReason::Stopped`, both `ReplayError::Stopped` directions and `FindingMismatch`. The frontier and depth expectations match `explore.rs` (depth check at :195). The `runtime_invariant` case is tested at engine level only. Reaching it through `ModelSystem` needs a fault the construction rules out, so an engine-level test is enough. |
| FND-002 | fixed | FR-120-AC-9 and TC-472 step 6 test `ContractUndetermined` for pre and post, the candidate digest and independence from clause names. The `SumOutOfDomain` undefined value exists in `quire-exact/src/outcome.rs:114`. FR-120-AC-10 and TC-473 step 1 test the result order, the first result, `StepEffect.result` and the result root. |
| FND-003 | fixed | TC-473 builds each effect by replaying hand-written one-step traces, with no seeds. Step 3 names the child process (`current_exe`, a filter for the ignored test, `current_dir` set to a `TempDir`). |
| FND-004 | fixed | the `wrong-role-mapping` row is added. The new anchor-kind `other` row expects the wrong code; SR-758 R2-FND-001 tracks that. |

Round 2 findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| R2-FND-001 | low | TC-474 step 6 samples with "a seed that takes `2` (found by the test from the pinned sampler's draws and fixed in the test)". The TC names no seed, so the expected result depends on a value the TC does not state. Fix: pin the seed. The pinned `n = 2` vectors FR-101 already carries give one, or the step can replay a hand-written trace `0 → 2`. | spec/test-cases/TC-474-engine-findings-and-stopped-expansions.md (step 6) |

Verdict at round 2: all four round-1 gaps are closed. TC-474 and
FR-101-AC-12 to AC-14 are concrete apart from R2-FND-001.

Round 3. This file was renumbered from SR-754 to SR-759, because #499
merged SR-754 on main.

| FND | outcome | sha/reason |
| --- | --- | --- |
| R2-FND-001 | fixed | TC-474 step 6 pins seed `424243`, trace `0`, `max_steps` 2, and expects the trace `0 → 2` and `FindingMismatch { step: 1 }`. Spot-checked with hashlib: the preimage for `424242` hashes to FR-101-AC-3's `cb7d4b3b…6682622`. The preimage for `424243` hashes to `cc49cc1a…0eecd395`, which is odd, so with `n = 2` it selects index 1. `n = 2` never rejects, since `2 * floor(2^256 / 2) = 2^256`. |

Verdict at round 3: no open gaps. Approve.
