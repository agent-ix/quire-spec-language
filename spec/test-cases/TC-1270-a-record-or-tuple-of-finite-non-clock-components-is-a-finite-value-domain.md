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
that an enum is finite, in order, as a bare parameter, result or granted field and inside a record; that a clock component is a clock place and adds no factor and no
unboundedness; and that a record or tuple with an unbounded non-clock component
stays unbounded and returns `NotSimulated::RequiresBound`.

Scope: FR-120-AC-22 to FR-120-AC-25.

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
   initial state. In a separate model, grant `oc: Option<Cfg>` and
   `sq: Sequence<Cfg>[0, 1]`, and declare `R1 { c: Cfg, a: Boolean }`.
2. Enum `Color` (`red`, `blue`, `green`): a parameter `c: Color`, a result of
   type `Color` and a granted field `col: Color`; then `Pick { col: Color, on:
   Boolean }` and `Note { note?: Boolean, n: Int[0, 1] }` as granted fields.
3. Declare `time dense unit ms`. Add `ck: Timer` (`Timer { x: Clock, n: Int[0, 3] }`),
   `sw: Stopwatch` (`Stopwatch { x: Clock, y: Clock }`) and `mx: Mix`
   (`Mix { t: Timer, b: Boolean }`) on `Cell`, and `retime()` with frame
   `modifies ck, sw, mx` and
   postcondition `self.ck = Timer { x: 0 ms, n: 3 }`. Read each domain and
   expand the initial state; repeat with no postcondition. In another model,
   grant `Sequence<Timer>[0, 1]` and `Sequence<Clock>[0, 1]` fields.
4. Add `op: Open` (`Open { n: Integer, b: Boolean }`), `tm: Tm`
   (`Tm { x: Clock, k: Integer }`), `wr: Wrap2` (`Wrap2 { o: Open }`) and
   `u: U` (union `U { a, b(Boolean) }`) as fields, each granted by a frame, one
   at a time. Call `explore_model` and `sample_model`.

Tag the tests `#[trace("TC-1270", "FR-120-AC-n")]`.

## Expected Results

- Step 1: no unbounded root. `cfg` has 6 values in order (`false`, 0), (`false`,
  1), (`false`, 2), (`true`, 0), (`true`, 1), (`true`, 2); `pr` has 4, ordered
  `false` 0, `false` 1, `true` 0, `true` 1; `w` has 18, `c` in `Cfg` order and
  then `o` absent, `false`, `true`. `put` generates 6 x 4 x 18 = 432 candidates,
  each counted against `max_candidates`. In the separate model, `oc` and `sq`
  have 7 values each and `R1` has 12.
- Step 2: `c`, the result and `col` are finite, 3 values in declaration order,
  no unbounded root ; `Pick` has 6 values, `col` most significant; `Note` has 6, `note`
  absent first.
- Step 3: `ck`, `sw` and `mx` are roots and none is unbounded. `Timer` has 4
  values (`n` 0 to 3 ascending), no factor from `x`; `Stopwatch` has exactly
  one value; `mx` has 8, `t` most significant. The expansion has a successor
  with `ck.n` 3 and `x` reset, `sw` unchanged, and no candidate ranges over a
  value of `x`. With no postcondition each clock keeps its pre-state value. The
  `Sequence<Timer>` and `Sequence<Clock>` fields are unbounded roots.
- Step 4: each of `op`, `tm`, `wr` and `u` is an unbounded root.
  `explore_model` returns `NotSimulated::RequiresBound` and `sample_model`
  `ModelSampleError::NotSimulated(RequiresBound)`, each naming that field's
  `WireNodeId`, with no state expanded. `tm`'s clock does not make it bounded.
