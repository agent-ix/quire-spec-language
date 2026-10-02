---
id: TC-597
title: "S3 binds trace variables to models, types indexed atoms and step labels, splits the match and checks align skip"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: verifies
---
# TC-597: S3 binds trace variables to models, types indexed atoms and step labels, splits the match and checks align skip

## Description

Verify S3's checks of hyper clauses over behaviours and relations over model executions: alias binding, object parameters, indexed atoms, the `μ` split, per-variable fairness, `align skip`, execution bindings, refusals at their spans, and identity.

Scope: FR-172-AC-1 to FR-172-AC-4.

## Test Procedure

Fixtures: ADR-023 §1, §8.2 and §13's clauses over the vault and device models; FR-172-AC-2 and AC-3's ill-formed variants; FR-171-AC-3's `Det`.

1. Check `NonInterference`, `SavesPower` and `Opaque`.
2. Check each FR-172-AC-2 variant.
3. Check §13's clause and each FR-172-AC-3 variant.
4. Check `Det`; compare node identities of `NonInterference`, its fairness-swapped variant and its variant with an extra `match` conjunct.

Tag the tests `#[trace("TC-597", "FR-172-AC-n")]`.

## Expected Results

- Step 1: alias `V` with two universal quantifiers, one instance, both conjuncts in `mu_universal`; aliases `S` and `N`; `Opaque`'s conjunct in `mu_existential`.
- Step 2: each refuses with the code and span FR-172-AC-2 names.
- Step 3: `align_skip` holding `mix`; each variant refuses `unsupported_construct`/`expression-form`.
- Step 4: two model execution bindings; three distinct identities.
