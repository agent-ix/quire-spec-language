---
id: TC-523
title: "The replay facade replays a model counterexample through ModelSystem"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: verifies
---
# TC-523: The replay facade replays a model counterexample through ModelSystem

## Description

Verify `replay_model_trace`: lassos, prefixes, stutter loops and deadlock
counterexamples reproduce; the recorded digest selects among several
post-states of one transition identity; an engine disagreement settles
`inconclusive`; a loop that does not close, an unfair lasso, a digest that
no successor has, a disabled step, a bad initial index and a misplaced
stutter marker refuse.

Scope: FR-128-AC-1 to FR-128-AC-4.

## Test Procedure

Take the counterexamples of TC-521 steps 1 to 3, with the subject's initial
snapshot and source in the byte provision and its universe in the request.

1. Replay the lasso under the constraint with no granularity, the bounded
   prefix and the stutter lasso. Over the `Branch` subject of FR-128-AC-1,
   replay a one-step prefix recording the digest of `v = 2`, and one
   recording a digest no successor of `step` has.
2. Replay the deadlock counterexample; the same payload in an envelope
   carrying the identities of the `Counter` unit whose `When` member covers
   value 3; the original payload truncated to end at value 2.
3. Replay each defective envelope of FR-128-AC-3.
4. Replay a lasso of `upd(a)` steps bound to `c = a`; replay after a source
   edit that changes the `package_id`; with `clause_node` naming another
   clause; step 1's lasso twice.

Tag the tests `#[trace("TC-523", "FR-128-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness` three times, the lasso with
  `trace_position` 0; the `Branch` prefix reproduces with `trace_position`
  1; the other refuses `stale_dependency`/`revision-mismatch` naming the
  step and the recorded digest.
- Step 2: `reproduced-with-evaluated-witness`; `inconclusive`, `Verdicts`;
  `inconclusive`, `Verdicts`.
- Step 3: each refuses as FR-128-AC-3 states, with no result, the
  `Observed` envelope before recompiling.
- Step 4: `inconclusive`, `Verdicts`; FR-098's stale `package_id` refusal;
  `stale_dependency`/`revision-mismatch`; equal results.
