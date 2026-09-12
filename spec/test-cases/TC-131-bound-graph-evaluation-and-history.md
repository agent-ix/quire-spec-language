---
id: TC-131
title: "Bound graph evaluation and preserve historical profiles"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: verifies }
---
# TC-131: Bound graph evaluation and preserve historical profiles

## Description

Verify graph resource accounting, immutable retries and historical
ConfigVersion compatibility.

## Test Procedure

Measure successful true and false depth-first traversals under
`quire.state.evaluation-work/1`, then rerun each with zero, exact and one-step-
insufficient graph-expansion, graph-edge, comparison and active-depth limits
while keeping other dimensions sufficient. Request each graph limit above its
NFR-009 hard ceiling and verify clamping. Exercise a self-loop at depth one, a
chain at the exact active-depth limit, a duplicate edge, and an exhaustion that
occurs immediately before the next expansion and immediately before the next
edge inspection. Retry from the unchanged package and environment.

Using otherwise identical offered objects, compare a target absent from a
declared-complete domain with the same absence under unavailable membership or
closure authority. Keep a known foreign target in both cases as the structural-
invalidity precedence control.

Run the frozen ConfigVersion source/model/runtime fixtures before and after the
new graph profile without adding a finite integer maximum or replacement parent
relation.

## Expected Results

Exact capacities complete; one-short and clamped-insufficient capacities return
typed exhaustion before the next operation and no Boolean. Retrying starts fresh
and repeats the same Boolean and deterministic usage; no run reports a path
witness. The complete-domain absence refuses as dangling, unavailable membership
or closure remains incomplete, and the independently known foreign target
refuses in either case. Historical fixtures retain their original selections
and outcomes; the new graph profile neither widens nor silently upgrades them.
