---
id: TC-738
title: "A deep package emits, reads back and verifies, in the stratified grammar"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-264
    type: verifies
---
# TC-738: A deep package emits, reads back and verifies, in the stratified grammar

## Description

Verify emission and I2 read-back at 100,000 deep, and the stratified body
grammar of every emitted package.

Scope: FR-264-AC-1, FR-264-AC-2, FR-264-AC-6, FR-264-AC-7.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Compile a package holding a 100,000-term sum under limits raised to fit
   it, emit it, and read it back through QSL's I2 read with the `i2.*`
   settings raised to fit. Clone the `V2Read`, compare, format for debug and
   drop both.
2. Walk the emitted JSON of every package of the TC-415 corpus (each nested
   expression form at 2 and at 1,000 levels) and of step 1, classifying each body position as Leaf, Group, Member or Body.

3. Emit a checked package once to measure its canonical byte length `N`, then
   emit it at `emit.output_bytes` `N`, `N - 1` and `0` (FR-264-AC-6).
4. Offer the already emitted package to an emit entry at `N - 1` and at `N`,
   and run the compile lifecycle's E4 entry at both bounds (FR-264-AC-7).

Tag the tests `#[trace("TC-738", "FR-264-AC-6")]`, `#[trace("TC-738", "FR-264-AC-7")]`, `#[trace("TC-738", "FR-264-AC-1")]`, `#[trace("TC-738", "FR-264-AC-2")]`.

## Expected Results

- Step 1: the read verifies at the emitted `package_id`; the clone compares
  equal; formatting and drops complete.
- Step 2: every body is in FR-322's stratified grammar.
- Step 3: bound `N` emits bytes equal to the larger-bound emission; bounds `N - 1`
  and `0` each refuse with `OutputLimit` naming `emit.output_bytes`, the bound and
  an actual above it, and write no bytes.
- Step 4: the remembered emission is refused below `N` and returned at `N`; the
  lifecycle returns `CompileRefusal::Limit` at `N - 1` and emits at `N`.
