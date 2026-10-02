---
id: TC-546
title: "check_step decides initial states and steps by the stutter, abstract-operation and any rules"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: verifies
---
# TC-546: check_step decides initial states and steps by the stutter, abstract-operation and any rules

## Description

Verify `check_initial` and `check_step` over functional mappings, with `_`
arguments and bound results, with hidden fields tracked as a candidate set,
and with protocol step-class rows.

Scope: FR-141-AC-1 to FR-141-AC-5.

## Test Procedure

Fixtures: `CasRefinesCounter` and its lost-update model (ADR-020 §8);
`RingIsQueue`; `Coin` (FR-141-AC-4); FR-136-AC-4's protocol subject.

1. Run the initial check and the three steps of FR-141-AC-1, and the lost-update `commitB` step.
2. Run each variant of FR-141-AC-2.
3. Run the `take`, broken `take` and `put` steps of FR-141-AC-3.
4. Run `Coin`'s `toss` and `reveal` with `side` hidden; then with `side =
   self.face`.
5. Run the protocol attempt-node step and the `fork` step of FR-141-AC-5.

Tag the tests `#[trace("TC-546", "FR-141-AC-n")]`.

## Expected Results

- Step 1: pass; `Taken::Stutter`; `Taken::Abstract([inc(c)])`;
  `AbstractStepRejected{inc(c), Postcondition}`.
- Step 2: `InitialNotAbstract{initial: 0}`; `StutterChanged`;
  `Taken::Stutter`; `NoAbstractMatch`; `AbstractStepRejected{inc(c),
  Precondition{CanInc}}`.
- Step 3: pass as `deq(r)`; `AbstractStepRejected{deq(r), Postcondition}`;
  `Taken::Abstract([enq(r, 1)])`.
- Step 4: a two-state set, then the `side` 1 state;
  `AbstractStepRejected{show(c), Frame{…}}`.
- Step 5: `Taken::Stutter` through the node row; pass by RS-3.
