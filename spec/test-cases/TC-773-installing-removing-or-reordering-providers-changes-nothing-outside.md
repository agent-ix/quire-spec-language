---
id: TC-773
title: "Installing, removing or reordering providers changes nothing outside the touched candidate sets"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: verifies
---
# TC-773: Installing, removing or reordering providers changes nothing outside the touched candidate sets

## Description

Verify FR-289's front-end, locality and meaning invariants. Order invariance is TC-194's.

Scope: FR-289-AC-1 to FR-289-AC-14.

## Test Procedure

1. Compile `tests/fixtures/spine-compile.native` in one test that builds an empty registry and in one that builds a registry from three manifests.
2. Property-test: for generated manifests and items, add a manifest advertising none of an item's capability kinds, then one advertising its kind, and compute the item's candidate set each time. Include probabilistic items with `statistical` and `exact` evidence and manifests that add only the other evidence kind.
3. Compile the `compile_fail` doctests that build a `refuted` terminal record with no reproducing replay result and a `proved` record with no `BackendId`, beside compiling controls; build a `refuted` record from a reproducing replay result and read it back.
4. Settle a process provider's counterexample from an input-arm replay that reproduces the violation, one that does not, one that reproduced a non-violation, a witness-arm result, a refusal and a fault; settle a plugin `proved` with an exhaustive basis and with each of the check-count, closed-form and witness bases (the three core-only bases with a `ProofBasis` variant; `exact-value`, `value-bounds` and `zone-certified` have none yet); read each QSL-only cause kind in its QSpec FR-331 plugin-wire spelling (`replay-parity`, `replay-refused`, `kani_vacuous_proof`) and the hyphenated `kani-vacuous-proof`, build an `inconclusive` result with each QSL-only typed cause, and read every other cause kind.
5. Build the diagnostic of a refused QSL-only cause, of a refused core-only basis and of a `refuted` with nothing to replay; settle a plugin `proved` with each of the five plugin bases, `bounded-solver` included.

Tag the tests `#[trace("TC-773", "<AC id>")]`.

## Expected Results

- Step 1: the `package` bytes and `check` outcomes are equal.
- Step 2: the first addition leaves the candidate set equal; the second adds exactly that backend. For probabilistic items, adding a manifest that advertises only the other evidence kind leaves the set equal; only a manifest advertising the item's exact evidence kind adds that backend.
- Step 3: the doctests fail to compile and the controls compile; the record reads back `refuted` with that replay result.
- Step 4: `refuted` naming the plugin and keeping the replay result; `replay-parity` with no replay result; `failed` with no replay result for the reproduced non-violation and for the witness arm; the refusal's own settlement and `failed` for the fault; `proved` with `trusted` and the plugin's identity for the exhaustive basis; the check-count, closed-form and witness bases refused (ADR-029 PL-7); each QSL-only kind refused with a `failed` record; each QSL-only typed cause refused; every other kind, the hyphenated `kani-vacuous-proof` included, not refused.
- Step 5: `invalid_result`/`unknown-cause` naming member `cause`, then `basis`; `invalid_result`/`missing-member` naming `counterexample`; each plugin basis `proved` with `trusted` and the plugin's identity.
