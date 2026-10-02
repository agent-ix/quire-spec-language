---
id: TC-570
title: "Weak each fairness is decided on the annotated quotient"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-153
    type: verifies
---
# TC-570: Weak each fairness is decided on the annotated quotient

## Description

Verify the spanning tree, cycle group, witness sets and the weak `each` test on ADR-021 §7.1's proof and refutation.

Scope: FR-153-AC-1 to FR-153-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.1's annotated subject, `[[a, b, c]]`, instance `c = a`.

1. Run `always eventually holds(c.versionNumber = 2)` under `fair weak each attemptUpdate`; inspect the SCC with `va = 0`.
2. Run `eventually always holds(c.versionNumber != 2)` under `fair weak each attemptUpdate`.

Tag the tests `#[trace("TC-570", "FR-153-AC-n")]`.

## Expected Results

- Step 1: six canonical states, entry `(0,0,0)`, tree word `(b c)` at `(0,0,1)`, generator `(b c)`, `W_taken` closing to `{upd(b), upd(c)}`, `W_disabled` empty; the SCC fails; `Holds` over 30 product states.
- Step 2: an 18-state SCC with all three labels, cycle group `{id, (b c)}`, the closure covering all three identities; the SCC passes.
