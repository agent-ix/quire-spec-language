---
id: TC-548
title: "The liveness half reads abstract fairness through the mapping under concrete fairness"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: verifies
---
# TC-548: The liveness half reads abstract fairness through the mapping under concrete fairness

## Description

Verify the second phase: no liveness half without `ensure` rows,
`Divergence` and `AbstractUnfair` lassos, concrete halts read through the
terminal stutter independently of `terminal` members, `unsupported` with
hidden fields, and the scheduler constraints in the `F_C` filter.

Scope: FR-143-AC-1 to FR-143-AC-5.

## Test Procedure

Fixtures: `CasRefinesCounter` and its divergence variant (ADR-020 §8);
`Halting` (FR-143-AC-2); `Pair` (FR-143-AC-3); `RegisterHistory` without its
`prev` row; a concrete protocol subject with branches `A` and `B`.

1. Check `CasRefinesCounter`; the divergence variant; the divergence variant
   without its `ensure` row.
2. Check `Halting`; without its `ensure` row; with precondition `self.value
   < 2`; each under every combination of `terminal` members.
3. Check `Pair`; with `assume fair weak each Impl::Pair::flipB` added.
4. Check `RegisterHistory` without `prev`, with and without `ensure fair
   weak Spec::Register::write`.
5. Run the `F_C` filter on FR-143-AC-5's SCC with default scheduling and
   with `scheduling adversarial`.

Tag the tests `#[trace("TC-548", "FR-143-AC-n")]`.

## Expected Results

- Step 1: `Checked(Holds{Exhaustive})`; `Checked(Violated)`, empty stem, one
  `peek` loop step, `Divergence{inc}`; `liveness: None`, safety
  `Holds{Exhaustive}`.
- Step 2: `Divergence{inc}` with stem `inc` and the terminal stutter loop at
  `value` 1; `None`; `Checked(Holds{Exhaustive})`; the same three verdicts
  in every combination.
- Step 3: `AbstractUnfair{flipB}`, empty stem, two `flipA` loop steps;
  `Checked(Holds{Exhaustive})`.
- Step 4: safety `Holds{Exhaustive}`, liveness `Unsupported(unsupported-requested-capability)` naming hidden abstract fields; `None`.
- Step 5: the SCC fails the filter through `B`'s `Scheduler` constraint; it
  passes and is returned as the violation.
