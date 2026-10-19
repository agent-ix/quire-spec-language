---
id: TC-1075
title: "Footprints are state-free and exist for disabled transitions and objects not yet created"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: verifies
---
# TC-1075: Footprints are state-free and exist for disabled transitions and objects not yet created

## Description

Verify that `ModelSystem::footprint` takes no state, returns a footprint for a transition whose object no state holds and for a disabled transition, and keys type-derived locations on declared types related by conformance in both directions.

Scope: FR-155-AC-7, FR-162-AC-5.

Catches a footprint hook that fails with an unknown transition when `bump(d)` is asked before `spawn` creates `d`, and an inherited operation whose footprint differs by the receiver's most-specific type.

## Test Procedure

Fixtures: FR-155-AC-1's `Counter` unit with `spawn()` framed `creates Counter`, `counters` holding `a`, `b`, `c`; the `Base`/`Sub` subject of FR-155-AC-7 with `cancel()` framed `deletes self`.

1. Call `footprint` for `bump(d)` with no `d` in any state, then again after `spawn` creates `d`; test that `spawn` meets `Membership{counters, d}`.
2. Call `footprint` for `bump(a)` while its precondition is false.
3. Call `footprint` for `cancel(s)` with `s` typed `Base`, `s` typed `Sub`, and `s` absent; call the product system's hook for `bump(d)`.

Tag the tests `#[trace("TC-1075", "<AC id>")]`.

## Expected Results

- Step 1: `Some` both times, equal; `spawn` is in the candidate of `bump(d)`.
- Step 2: `Some`, equal to the enabled footprint.
- Step 3: the same footprint in all three cases, with `Membership{bases, s}`, `Membership{subs, s}` and `AnyField{Holder/r}`; the product's footprint equals the model's.
