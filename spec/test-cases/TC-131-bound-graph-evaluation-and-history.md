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
`quire.value.accounting/v1`, then rerun each with zero, exact and one-step-
insufficient `accounting.value_occurrences` (graph expansion),
`accounting.work_units` (edge inspection) and `accounting.result_units` (result)
while keeping other counters sufficient. Raise each setting above its FR-255
default and run work past the default. Exercise a self-loop, a chain of 10,000
objects on a thread with a 512 KiB stack, a duplicate edge, and exhaustion
immediately before the next expansion, edge inspection and result retention.
Retry from the unchanged package and environment.

Using otherwise identical offered objects, compare a target absent from a
declared-complete domain with the same absence under unavailable membership or
closure authority. Keep a known foreign target in both cases as the structural-
invalidity precedence control.

Run the frozen ConfigVersion source/model/runtime fixtures before and after the
new graph profile without adding a finite integer maximum or replacement parent
relation.

## Expected Results

Exact and raised capacities complete; one-short capacities return the owning
typed incomplete/resource-limit outcome before the next operation and no Boolean. Retrying starts fresh
and repeats the same Boolean and deterministic usage; no run reports a path
witness. The complete-domain absence refuses as dangling, unavailable membership
or closure remains incomplete, and the independently known foreign target
refuses in either case. Historical fixtures retain their original selections
and outcomes; the new graph profile neither widens nor silently upgrades them.
