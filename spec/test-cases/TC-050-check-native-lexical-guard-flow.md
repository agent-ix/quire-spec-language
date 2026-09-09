---
id: TC-050
title: "Check lexical scope and guarded evaluation order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-050: Check lexical scope and guarded evaluation order

## Description

Integration, priority P1. Verifies FR-016-AC-7. Planned until real execution; setup
must use the qualified source-derived Rust producer and actual public IR APIs.

## Test Procedure

Check stable optional aliases, compound optional let initializers, repeated unbound compound optional expressions, quantifier element guards, duplicate/nested shadowing, own-initializer/domain name use and false/true unreachable branches. Compare an unsafe initializer followed by a body guard with a guard that precedes the initializer. Include unknown names and ill-typed expressions in skipped branches. Compare a true-join guard where both alternatives imply present(p), a guard where only one does, and a contradictory present(p)/not present(p) path. Reuse a guarded Boolean let value as another guard.

## Expected Results

Stable and let-bound optional keys admit their exact guards; repeated text alone does not. Right/body guards cannot justify earlier work. Explicitly unreachable unsafe branches can omit definedness goals while unknown names and type errors still refuse through their actual frontend phase. Local/domain scope is preserved and shadowing returns ill_typed. Successful results retain native let/quantifier/if/implication nodes rather than executable textual substitution.
