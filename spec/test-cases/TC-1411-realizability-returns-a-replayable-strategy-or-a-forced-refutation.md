---
id: TC-1411
title: "Realizability returns a replayable strategy, a forced refutation or a non-conclusive result"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-571
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-175
    type: references
---
# TC-1411: Realizability returns a replayable strategy, a forced refutation or a non-conclusive result

## Description

Verify the strategy witness and its replay gate, both refutation forms,
the premise rules and the environment quantification.

Scope: FR-571-AC-1 to FR-571-AC-6; QSpec FR-175-AC-1 to AC-5.

## Test Procedure

1. Decide the ADR-036 worked example repaired by the `cancelled` message
   under `[1,1]` delivery; run the replay walk; then edit one table entry
   and run it again.
2. Decide the unrepaired worked example, and a protocol the environment can
   defeat under every owner branch by one admitted loss.
3. Decide the repaired example with the role-knowledge premise `None`
   (not stated), then `Some` of the empty set; and with `max_depth` too
   small.
4. Recompute `observation(role, history)` for every explored history of
   step 1 and compare actions of equal sequences; build a class with no
   common winning action.
5. Decide a protocol that every sampled run completes but one admitted
   outcome defeats; and one under an `at-least-once` channel.
6. Decide the step 2 protocol that one admitted loss defeats and read
   `defeats_per_branch`; and a protocol in which a role, in one branch,
   can send while its receive is not yet enabled but another role's step
   will enable it, and must receive in the other, both in one class.

Tag the tests `#[trace("TC-1411", "FR-571-AC-n")]`.

## Expected Results

- Step 1: `RealizabilityWitness`, the walk passes; with the edit the result
  is `inconclusive`, not realizable.
- Step 2: `IndistinguishableHistories` naming role, histories, actions and
  obligation; and `EnvironmentDefeat` with one defeating play per branch.
- Step 3: `inconclusive`; the witness; `inconclusive`.
- Step 4: equal sequences get equal actions; the empty-intersection class is
  refuted or `inconclusive`, never in a witness.
- Step 5: not realizable; a result depending on a cut count is
  `inconclusive`.
- Step 6: one entry; no owner choice exists to avoid a history or to be
  cut, so FR-571-AC-6's cases do not arise; the protocol whose role can wait
  is not refuted.
