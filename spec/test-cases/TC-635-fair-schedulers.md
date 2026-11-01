---
id: TC-635
title: "Every-scheduler claims with a fairness set are decided over fair schedulers through fair end components"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: verifies
---
# TC-635: Every-scheduler claims with a fairness set are decided over fair schedulers through fair end components

## Description

Verify fair end-component refinement, fair reachability infima including terminal stutter, `each`-granularity expansion, fair witness validation across multiple bottom SCCs, the fair long-run infimum with its randomized witness, and that bounded forms are unchanged.

Scope: FR-200-AC-1 to FR-200-AC-7.

## Test Procedure

Fixtures: §15.6's `Coin`; `Coin2` of FR-200-AC-2; `Avail2`; `Deliver`; `TerminalFair`; `Pair`; and `Split`. `TerminalFair` has `live`, target `done`, and non-target terminal `stopped`, reached by `abandon`; the terminal stutter at `stopped` enables no constraint. `Pair` has one state and `tick(a)` / `tick(b)` self-loops for two receivers. `Split` has an initial 1/2–1/2 transition to `left` and `right`, each with a `step` self-loop.

1. Compute the maximal fair end components and the infimum for `Coin` under strong and under weak fairness to `flip`.
2. Compute them for `Coin2` under strong fairness to `flip`; check the witness's induced bottom component.
3. Compute `Avail2`'s fair infimum under strong fairness to `repair_fast`; refute `>= 0.9995` and read the witness.
4. Decide `Deliver` with `fair weak Msg::send_a`.
5. Compute the minimum reachability probability of `done` in `TerminalFair` under `fair strong TerminalFair::proceed`; inspect `U` and assert that the returned fair witness selects `abandon` for `probability >= 1/2 [eventually holds(done)]`.
6. Compute the fair end component in `Pair` under `fair weak each Pair::tick`; inspect the expanded scheduled identities and the component's action set.
7. Compute the reachability result for `probability >= 1/2 [eventually holds(done)]` under `fair strong Split::step`; inspect the returned witness and its induced chain's bottom SCCs.

Tag the tests `#[trace("TC-635", "FR-200-AC-n")]`.

## Expected Results

- Step 1: none; infimum 1 in both cases.
- Step 2: `{stuck}`; infimum 0; the witness takes `stall` and its bottom component is fair.
- Step 3: `1000/1001`; a witness with `k = 1` and induced value `1400/1401`.
- Step 4: `24/25`, with the fairness set recorded.
- Step 5: `{stopped}` is a fair end component in `U` by terminal stuttering; the minimum reachability probability is 0, and the `abandon` witness replays `refuted` with a fair bottom component.
- Step 6: `fair weak each Pair::tick` expands to distinct scheduled identities `tick(a)` and `tick(b)`; both appear in the fair end component's action set.
- Step 7: the returned witness refutes the claim and is accepted as fair after validation checks both reachable bottom SCCs, `{left}` and `{right}`.
