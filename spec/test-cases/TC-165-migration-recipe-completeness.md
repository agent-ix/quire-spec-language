---
id: TC-165
title: "The migration recipe names every required test, conversion, removal condition and remaining family"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-066
    type: verifies
---
# TC-165: The migration recipe names every required test, conversion, removal condition and remaining family

## Description

Verify that the checked-in family-migration recipe document names all five
required-test categories, all three required-conversion categories, the
ADR-011 §7.3 M-6e removal condition with an implementing ticket for each
remaining family, and cites a concrete artifact from the function-application
migration as its worked example. Scope: FR-066-AC-1 through FR-066-AC-4.

## Test Procedure

1. Locate the recipe document at its checked-in path and read its
   required-tests section.
2. Read its required-conversions section.
3. Read its removal-condition section and its per-family ticket references.
4. Read its worked-example section.

## Expected Results

- Step 1: the section names all five categories -- clause-level unit,
  builder-ordering, seam-probe, wire-totality, backend-absence corpus -- each
  with a one-sentence description of what it verifies.
- Step 2: the section names all three categories -- v2 emitter, evaluator,
  requirement derivation -- and states that a family whose forms cross into
  IR, RT or CG has its wire and IR-side conversions owned by those
  repositories' own tickets.
- Step 3: the section states the removal condition in ADR-011 §7.3 M-6e's own
  terms (old composed-checker path deleted in the PR landing the S3 family
  checker and S4 emission) and names at least one implementing ticket for
  each of `StateModel`, `SumCase`, `TemporalTrace`, `ProtocolClause` and
  `Relation`.
- Step 4: the section names at least one real test file and one real deleted
  symbol from the function-application migration's implementation, not a
  placeholder or hypothetical name.
