---
id: TC-133
title: "Admit bounded choreography control and visible progress"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-048, type: verifies }
---
# TC-133: Admit bounded choreography control and visible progress

## Description

Verify exact sequence, choice, parallel, repeat and observed-guard admission,
including the repeat-progress soundness rule.

## Test Procedure

Compile closed and observed Boolean choices/repeats nested inside sequence and
parallel control with an all-branch join. Exercise receive, same-owner attempt
and domain-event atoms plus wrong-owner, branch-local, await-success-only and
unused unsupported operands. Use an inner observed repeat whose false guard is
feasible as the only proposed progress for an outer repeat; pair it with an
infeasible-false counterpart. Measure zero, exact and one-short proof budgets.
Exercise repeat maximum zero with both false and true guards, maximum one with
normal exit and exhaustion, the largest admitted finite maximum without eager
unfolding, and negative, unbounded and one-over-representation values.

## Expected Results

The positive graph retains authored operands and topology. Invalid visibility,
holes, overlaps and unproved progress refuse. A feasible repeat exit supplies
no enclosing progress; an infeasible exit preserves independently proven body
progress. Closed-guard behavior and charge order remain unchanged, and resource
exhaustion is not reported as a control violation. At maximum zero, false exits
normally and true selects the authored exhaustion branch once without entering
the body. Maximum one preserves one iteration identity; every invalid or
unrepresentable maximum refuses before an admitted control graph is emitted.
