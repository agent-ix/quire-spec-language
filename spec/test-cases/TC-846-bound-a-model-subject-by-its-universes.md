---
id: TC-846
title: "A model subject is finite by its universes, apart from proof bounds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: verifies
---
# TC-846: A model subject is finite by its universes, apart from proof bounds

## Description

Verify that universes, not proof bounds, make a model subject's roots
finite, that the requirement record ignores universes, and that the
universe enters the obligation identity.

Scope: FR-336-AC-1 to FR-336-AC-4.

## Test Procedure

Use ADR-018 §6's ConfigVersion example with `config_history` declared
with no maximum.

1. Read `ReachesTwo`'s record for requests with universe `{a, b}` and
   `{a, b, c}`.
2. Model-check it with no universe, then with `{a, b}`.
3. Model-check a `Counter` variant with `step(n: Integer)`, without and with
   a `ProofBound{IntegerRange{0, 3}}`, then with `n: Int[0, 3]` declared.
4. Request `ReachesTwo` under `fair weak each` over `{a, b}` and
   `{a, b, c}`; compare obligation identities and the joins of the two
   results.

Tag the tests `#[trace("TC-846", "FR-336-AC-n")]`.

## Expected Results

- Step 1: (`temporal-satisfaction`, `Unbounded`) with the population's
  domain, both times.
- Step 2: `RequiresBound` naming the population root, no state explored;
  then FR-126-AC-1's outcome.
- Step 3: `RequiresBound` naming the parameter root, twice; then an explored
  run.
- Step 4: identities differ only in the universe; each `proved` joins its
  own request index.
