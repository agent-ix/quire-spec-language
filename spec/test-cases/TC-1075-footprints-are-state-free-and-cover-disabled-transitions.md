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

Verify that `ModelSystem::footprint` takes no state, returns exact footprints for a transition whose object no state holds and for a disabled transition, and derives type-dependent locations from the `object_type` of the identity's triples exactly as QSpec FR-383 states.

Scope: FR-155-AC-7, FR-162-AC-5.

Catches a footprint hook that fails with an unknown transition when `bump(d)` is asked before `spawn` creates `d`, a footprint derived from the operation's or a parameter's declared type, and one that widens beyond FR-383.

## Test Procedure

Fixtures: FR-155-AC-1's `Counter` unit with `spawn()` framed `creates Counter`, `counters` holding `a`, `b`, `c`; the `Base`, `Sub`, `C extends A, B` and `K` subject of FR-155-AC-7.

1. Call `footprint` for `bump(d)` with no state, `d`'s triple carrying `Counter`; test that `spawn`, enabled at the initial state, meets `Membership{counters, d}`.
2. Call `footprint` for `bump(a)`, whose precondition is false at some state.
3. Call `footprint` for `cancel(s)` with the triple's `object_type` `Sub` and then `Base`, and for `cancel(c)` with `object_type` `C`; call `drop(k, c)`, `touch(k, c)` and `touch(k, x)` with `c` of `C` and `x` of `A`; call the product system's hook for `bump(d)`.

Tag the tests `#[trace("TC-1075", "<AC id>")]`.

## Expected Results

- Step 1: `Some`, reads exactly `{Field{d, v}, Membership{counters, d}}`, writes exactly `{Field{d, v}}`; the candidate seeded by `spawn` holds `bump(d)`.
- Step 2: `Some`, the footprint FR-155-AC-1 states for `bump(a)`.
- Step 3: the exact read and write sets of FR-155-AC-7 for `drop` and `touch`, whose argument memberships and referrer fields follow the argument triple's `object_type`, not the declared `A`; the exact read and write sets of FR-155-AC-7 for each of the three triples, which differ from one another where `object_type` differs; the product's footprint equals the model's.
