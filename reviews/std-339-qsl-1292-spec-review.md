---
id: SR-16199
title: "spec-review of the contract-admission Begin without a caller run (quire-spec-language PR 1292)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@fa028512; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md; spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md; spec/test-cases/TC-1191-begin-cancellation-through-contract-admission-and-the-pre-admitted-entries.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1191
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: references
---

# Spec review of the contract-admission Begin without a caller run (quire-spec-language PR 1292)

## Summary

Ticket: STD-339. This is one spec-review pass over
`git diff origin/main...fa028512`. The diff rewrites FR-262's Begin text so
that `quire-eval`'s Begin with contract admission takes no caller run, says
the pre-admitted Begin is the only path on which a caller's run takes part,
removes FR-262-AC-12 runs 3 and 4, fixes FR-403's Argument admission
wording, adds FR-401-AC-10 and adds TC-1191. The oracles were merged #1280
and #1290 text (FR-262, FR-277, FR-401, FR-403, FR-405, FR-406) on
origin/main d7b13448 and the tests in code PR #1269
(`qsl-eval/tests/it/reference_adapter.rs`:
`begin_of_a_call_and_of_an_expression_observes_the_meters_handle`,
`call_supplied_polls_the_callers_handle_and_the_meters_at_the_first_charge`).
The code-review and gap-analysis methods did not run because the diff has
no code.

Gates: origin/main is an ancestor of the head. `make check-spec-validation`
exits 0 with no warning on a touched file.

Examined, clean:

- **No stale caller-run wording remains.** A grep of FR-262, FR-277,
  FR-401, FR-403, FR-405 and FR-406 (and all of `spec/` and `plan/`) for
  "caller-supplied run", "differs", "both handles", "different handle" and
  "passes its own run" finds only unrelated uses (type and record
  comparisons) and FR-401-AC-10's own wording.
- **FR-262, FR-401 and FR-403 agree.** All three say Begin with contract
  admission admits through its own run at the default limits, over the
  meter's installed handle or a handle no caller holds. FR-401's existing
  paragraph (`call_supplied` admits under the caller's run alone, then
  begins pre-admitted) matches the new FR-262 sentence that the pre-admitted
  Begin is the only path a caller's run takes part in.
- **FR-262-AC-12 is now expressible.** Runs 1 and 2 and both session
  outcomes use only Begin and the meter. #1269's test covers run 1 and both
  session outcomes; run 2 (trip at the second argument's first poll point)
  still needs a test in #1269, which is that PR's open SR-12354 FND-008, not
  a spec defect.
- **FR-401-AC-10 matches the behaviour #1269 implements.** #1269's
  `call_supplied` test already asserts its result for an uncancelled run and
  a cancelled meter handle.
- **Ids are unique.** TC-1191 and FR-401-AC-10 appear in no other open QSL
  PR diff and nowhere on main.
- **Wording is behaviour-only.** No status, changelog or "previously"
  language.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | FR-401-AC-10 and TC-1191 step 3 name the TC-918 `Compare` package, but the only `call_supplied` vector of that shape (#1269, `reference_adapter.rs`) runs over the two-argument `g(x: Int[0, 9], y: Int[0, 9])` package. As written, #1269 needs a new `Compare` test for `call_supplied`, and the existing test does not bind AC-10. Fix: either name the `g` package in AC-10 and TC-1191 step 3, or have #1269 add a `Compare` `call_supplied` vector. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:291; spec/test-cases/TC-1191-begin-cancellation-through-contract-admission-and-the-pre-admitted-entries.md:37-39 | missing-requirement |
| FND-002 | low | The caller-run path's own cancellation has no criterion. FR-262 says the pre-admitted path is the only one where a caller's run takes part, and FR-401 says `call_supplied` polls only that run's handle. No AC in FR-262 or FR-401 states that a cancelled caller run gives `CallFailure::Cancelled` with zero consumption whether or not the meter has a handle. #1269 asserts exactly this, tagged FR-401-AC-9, which covers only `call` and `evaluate`. FR-277-AC-5 / TC-906 may cover it in general terms. Fix: add that run to FR-401-AC-10 (or name the FR-277 criterion that binds it). | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:228-231, :291 | missing-requirement |

## Verdict

Mergeable. The PR does what STD-339 asks. The contract-admission Begin
takes no caller run, the paragraph about passing a run and polling both
handles is gone, FR-262-AC-12 keeps only the runs Begin can express, and
FR-403 no longer refers to a caller-supplied run. FR-262, FR-401 and FR-403
now agree. The two low findings concern how the new criterion lines up with
#1269's tests. Neither blocks the merge, and both can be resolved in #1269
or in a one-line follow-up.
