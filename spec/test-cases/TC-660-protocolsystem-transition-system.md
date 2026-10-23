---
id: TC-660
title: "ProtocolSystem implements TransitionSystem with canonical identities and the shared application rule"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: verifies
---
# TC-660: ProtocolSystem implements TransitionSystem with canonical identities and the shared application rule

## Description

Verify `ProtocolSystem<Sc>`'s initial states, keys and canonically ordered successors, its transition identity form, the shared application function, `Sc` as the seam's identity, the pre-check and model checking over a protocol subject.

Scope: FR-215-AC-1 to FR-215-AC-5.

## Test Procedure

1. Explore `Fill` with FR-101's engine; read s1's successors and their
   serialized identities.
2. Compare `bump()`'s successors with `ModelSystem`'s; run the undecided
   postcondition variant through `model_check`.
3. Compare `Fill`'s states under `Sc` and under an independent test model
   implementing `Sc`'s members.
4. Run the unbounded event record protocol; model-check `Fill`'s and the
   repaired `Fill`'s deadlock-freedom items twice each.
5. Build the protocol system for FR-215-AC-5's `event` binder of type `Msg`
   and list the `event` steps at the state where its thread rests there; run
   the `Wide` variant.

Tag the tests `#[trace("TC-660", "FR-215-AC-n")]`.

## Expected Results

- Step 1: seven states, six transitions; `attempt(A)` before `attempt(B)`;
  identities in the protocol-transition form.
- Step 2: equal successors; `ContractUndetermined` and V-6
  `UndecidedSuccessor`.
- Step 3: equal state sets; empty memory member; no internal step.
- Step 4: `RequiresBound` before exploring; `Violated` (`Deadlock`) and
  `Holds{Exhaustive}` over ten states; byte-equal reruns.
- Step 5: eight `event` steps in the stated record order and no refusal when
  the system is built; `RequiresBound` naming the binder, `Wide` and `t`
  before exploring.
