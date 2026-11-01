---
id: TC-553
title: "Concrete steps are decided against an abstract protocol with internal closure and observation labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-148
    type: verifies
---
# TC-553: Concrete steps are decided against an abstract protocol with internal closure and observation labels

## Description

Verify the abstract protocol state sets with internal closure, visible and
internal concrete steps, refutations that replay, and QSpec FR-177's
divergence-freedom and environment-assumption implication checks on the same
product.

Scope: FR-148-AC-1 to FR-148-AC-10.

## Test Procedure

Fixtures: `Twice`, `Once` and `CasTwice` (FR-147, FR-148); the lost-update
model (ADR-020 §8) with `peek` removed; `CasTwice` with `peek` kept.

1. Run `check_initial` and the steps `beginA`, `commitA`, then a second
   commit, over `CasTwice`.
2. Check and settle `CasTwice` over universe `counters = {c}`, without
   applying the model-subject simulation-certificate checker to its
   abstract-protocol safety result; the protocol step decisions and further
   checks have no core certificate checker.
3. Check the lost-update model against `Twice` and replay its
   counterexample.
4. Check `CasTwice` against `Once` and replay its counterexample.
5. Check `CasTwice` with `peek` kept and mapped `-> stutter`, with no
   fairness rows.
6. Check a one-thread protocol whose only enabled node maps to `inc`, with
   weak whole scheduler fairness, against a concrete `inc` model with the
   corresponding weak whole `assume`; check the assumption implication over
   every reachable paired lasso.
7. Change the protocol to adversarial scheduling and remove its authored and
   memory fairness constraints. Keep concrete weak whole `assume inc` and add
   an infinite stutter loop while `inc` remains enabled; check, settle and
   replay the assumption-strengthening witness.
8. Use two concrete transition classes, `left` and `right`; constrain the
   abstract protocol only to weak fairness of `left`, and the concrete model
   to weak fairness of both classes. Check a reachable lasso that takes
   `left` fairly while `right` remains continuously enabled but is never
   taken; verify the implication counterexample. Repeat with a product limit
   below the complete exploration and verify `incomplete`, never `proved`.
9. Compare a strong each constraint over a finite single-operation domain to
   a weak whole constraint for that operation (which passes), then compare a
   weak abstract constraint to strong concrete fairness on a lasso where the
   operation is enabled infinitely often but not continuously and never
   taken (which refutes and replays).
10. Use an abstract protocol with a `tso` `parallel` and
    `scheduling adversarial`, so its scheduler constraints are absent but its
    derived memory constraint `fair weak each flush(writer)` remains in `S`.
    Make `tick` the writer's repeated buffered store and keep concrete `inc`
    continuously enabled but untaken on the lasso. Check that the lasso takes
    a `flush(writer)` each cycle, satisfies memory fairness, violates concrete
    weak whole `assume inc`, and is returned and replayed as the
    assumption-strengthening witness.

Tag the tests `#[trace("TC-553", "FR-148-AC-n")]`.

## Expected Results

- Step 1: one state at `a1`, `value` 0; the state at `a2`, `value` 1; the
  finished state with `value` 2.
- Step 2: `proved`, `closed-scope`, `Uncertified`; FR-149 does not certify
  protocol-subject step decisions, and the further checks have no core
  certificate checker.
- Step 3: `refuted`, `NoAbstractMatch` at the commit that leaves `value`
  unchanged; `reproduced-with-evaluated-witness`.
- Step 4: `refuted`, `NoAbstractMatch{position: 4}` at the second commit;
  reproduced.
- Step 5: `refuted` by the divergence-freedom check, a lasso whose loop is
  one `peek` step at the initial state.
- Step 6: the abstract scheduler constraint implies the matching concrete
  weak whole assumption over all reachable paired lassos; the item passes.
- Step 7: the empty specification assumption is true on the stutter loop and
  the concrete assumption is false; a typed assumption-strengthening witness
  is returned and replay reproduces it.
- Step 8: the `right` fairness constraint has a concrete counterexample
  lasso despite the abstract `left` constraint; a limited exploration settles
  incomplete without a proved result.
- Step 9: strong each implies weak whole for the finite class set; weak does
  not imply strong on the intermittent-enabledness lasso. Replay reproduces
  the latter assumption-strengthening witness.
- Step 10: `S` contains the memory-origin flush constraint despite adversarial
  scheduling; the lasso satisfies that constraint, while the concrete `inc`
  assumption is violated. The assumption-strengthening witness is replayed
  with the same paired path and fairness predicates.
