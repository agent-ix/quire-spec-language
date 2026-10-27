---
id: TC-1270
title: "A record or tuple of finite non-clock components is a finite value domain"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---
# TC-1270: A record or tuple of finite non-clock components is a finite value domain

## Description

Verify that a record or tuple whose non-clock components all have a finite
value domain is a finite value domain in `ModelSystem::domains()`, enumerated
with the first declared component most significant (ADR-027 ST-11, FR-215);
that a clock component is a clock place and adds no factor and no
unboundedness; and that a record or tuple with an unbounded non-clock component
stays unbounded and returns `NotSimulated::RequiresBound`.

Scope: FR-120-AC-22 to FR-120-AC-24.

## Test Procedure

Integration tests in `qsl-eval/tests/it/`. Build a hand-authored domain package
in TC-458's fixture shape (package `test/cells`): object type `Cell`, population
`cells` with universe `{k1}`, and the record, tuple and clock types each step
names, selected as model alias `Cells`. Compile through S1 to S4 in memory, with
no S5 emission. The initial snapshot holds `k1` with every field at its first
domain value. Read domains through `ModelSystem::domains`, successors through
`ModelSystem::successors`.

1. Fields `cfg: Cfg` (`Cfg { on: Boolean, n: Int[0, 2] }`), `pr: Pr`
   (`Pr(Boolean, Int[0, 1])`), `w: Wrap` (`Wrap { c: Cfg, o: Option<Boolean> }`);
   operation `put()` with frame `modifies cfg, pr, w` and no clause. Read the
   domain of each field and count the candidates `put` generates from the
   initial state. Add a field of `Option<Cfg>` and one of
   `Sequence<Cfg>[0, 1]`.
2. Add `ck: Timer` (`Timer { x: Clock, n: Int[0, 3] }`), `sw: Stopwatch`
   (`Stopwatch { x: Clock, y: Clock }`) and `Mix { t: Timer, b: Boolean }`.
   Read each domain. Add a postcondition `self.ck = Timer { x: 0 ms, n: 3 }`
   and expand the initial state.
3. Add `op: Open` (`Open { n: Integer, b: Boolean }`), `tm: Tm`
   (`Tm { x: Clock, label: Text }`) and `Wrap2 { o: Open }`, each in a frame
   that grants it, one at a time. Call `explore_model` and `sample_model`.

Tag the tests `#[trace("TC-1270", "FR-120-AC-n")]`.

## Expected Results

- Step 1: no unbounded root. `cfg` has 6 values in order (`false`, 0), (`false`,
  1), (`false`, 2), (`true`, 0), (`true`, 1), (`true`, 2); `pr` has 4, ordered
  `false` 0, `false` 1, `true` 0, `true` 1; `w` has 18, `c` in `Cfg` order and
  then `o` absent, `false`, `true`. `put` generates 6 x 4 x 18 = 432 candidates,
  each counted against `max_candidates`. The `Option<Cfg>` and
  `Sequence<Cfg>[0, 1]` fields each have 7 values.
- Step 2: `Timer` has 4 values (`n` 0 to 3 ascending), no factor from `x`, and
  `ck` is no unbounded root; `Stopwatch` has exactly one value; `Mix` has 8
  values, `t` most significant. The expansion has a successor with `ck.n` 3
  and `x` reset, and no candidate ranges over a value of `x`.
- Step 3: each of `op`, `tm` and `Wrap2` is an unbounded root.
  `explore_model` returns `NotSimulated::RequiresBound` and `sample_model`
  `ModelSampleError::NotSimulated(RequiresBound)`, each naming that root's
  `WireNodeId`, with no state expanded. `tm`'s clock does not make it bounded.
