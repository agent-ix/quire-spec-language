---
id: TC-1267
title: "A temporal clause with a capture checks, emits and is observed at activation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: verifies
---
# TC-1267: A temporal clause with a capture checks, emits and is observed at activation

## Description

Verify that S3 checks a temporal clause's captures with the existing causes and
keeps them, that S4 emits the captures aggregate and the capture parameter
nodes so a clause with an `on each` activation and a capture is compiled with a
distinct key, and that the engine fixes a capture at the activation and ends
the run with `UndefinedEvaluation` located at a capture it cannot evaluate.

Scope: FR-337-AC-8 to FR-337-AC-14, with FR-218-AC-16 (TC-1056 row 23a) over
the emitted clause.

## Test Procedure

Use FR-124-AC-1's `Counter` unit with `over (c: M::Counter)` under
infinite-trace unless a step names another profile.

1. Compile `always holds(c.value <= 1)` `on each (q: M::Counter) when
   (c.value >= 1)` with the body captures `n: Int[0, 3] = c.value;` and `m:
   Int[0, 3] = q.value;` and a `holds(c.value >= n)` and a `holds(m = n)`;
   compile the same captures under `on origin`.
2. Compile the mutations of AC-8: a `Boolean` capture of `c.value`; the type
   `M::Absent`; the initializer `c.value + 1`; an initializer reading a later
   capture, itself, and the guard reading a capture; a capture named `c`, `q`
   and as an earlier capture; a clause with a defect in the guard and in a
   capture, and one in a capture and in a `holds` condition.
3. Read the emitted clause of step 1 and of step 1 under `on origin`: the
   captures argument, the capture nodes with their levels, types and
   occurrences, the `holds` reads of `n`, the I2 reader's triple and `over`
   node; compile the clause with no capture.
4. Compile step 1's clause with `n` renamed `p`, `n` initialized `q.value`,
   `n` declared `Int[0, 9]`, `n` and `m` swapped, a third capture added, no
   capture, and reformatted; compare the clause keys and `package_id`s;
   compile `requires temporal Due;` over the clause with and without the
   captures and compare the clause keys and the nodes the `requirements` set
   holds.
5. Run the production letter seam over the `Counter` subject: under
   event-position false-extension the clause `eventually[0,1] holds(c.value =
   3)` `on each (q: M::Counter) when (c.value = 1)` and `when (c.value >= 2)`
   each with an unread `capture n: Int[0, 3] = c.value;`; under
   infinite-trace `always holds(true)` `on each (q: M::Counter) when (true)`
   and `when (c.value = 7)` with `capture z: Boolean = (6 mod (2 - c.value) =
   0);`.
6. Run `eventually[0,1] holds(n < 3 and c.value = n + 1)` `on each (q: M::Counter) when
   (c.value = 1)` with `capture n: Int[0, 3] = c.value;` through the engine,
   and the same clause with the `holds` condition `c.value = 3`; the refusal
   cases of AC-12 (infinite-trace `always`, a bounded `on origin` monitor, a
   second capture, an unbounded root), checking that no state is charged.
7. Run FR-128 replay and the FR-338 and FR-339 checkers over AC-13's
   clauses, and read the footprint of AC-14's clause with and without the
   capture.

Tag the tests `#[trace("TC-1267", "FR-337-AC-n")]`.

## Expected Results

- Step 1: every clause checks and the checked clause keeps `n` then `m` with
  declared type, typed initializer and slot count; both activations check.
- Step 2: `ill_typed`/`type-mismatch` (`expected` `Boolean`);
  `missing_declaration`/`missing-name` at `Absent`;
  `undefined_expression`/`unproved-range` at `c.value + 1`;
  `missing_declaration`/`missing-name` at each out-of-scope read;
  `conflicting_declaration`/`duplicate-declaration` at the capture's name,
  naming both declarations; one refusal, at the guard, and one at the
  capture. No refused clause gives a package.
- Step 3: the captures aggregate binds `n` to the term of `c.value` (a
  reference to `c`'s level-0 node) and `m` to the term of `q.value` (a
  reference to the level-1 trigger node); the nodes `n` and `m` are `value`/
  `parameter` nodes at levels 2 and 3 (1 and 2 under `on origin`) typed by
  `Int[0, 3]`'s type node, located at the capture names; `holds(c.value >= n)`
  references `n`'s node; the no-capture clause binds the empty aggregate; the
  I2 reader admits each package, returns the triple and `over` node, never a
  capture node as `over`.
- Step 4: every variant has a different clause key and `package_id`;
  reformatting changes neither; the node the `requirements` set holds for
  `Due` is the clause node, its key equal to the clause key and different
  with and without the captures.
- Step 5: `Violated` with the prefix `0 -inc-> 1 -inc-> 2`, then
  `Holds{Exhaustive}`; `Violated` with `kind: UndefinedEvaluation` at position
  2, cause `division-by-zero`, the site naming `z`, then `Holds{Exhaustive}`.
- Step 6: `ModelCheckRefusal::CaptureRead` naming the clause and `n`, no
  verdict and no state explored, likewise for each listed case, `RequiresBound`
  for the unbounded root; the second clause returns step 5's first verdict.
- Step 7: replay `reproduced-with-evaluated-witness`; `Holds{Exhaustive}`
  accepted by FR-338; the undefined `z` replays to `UndefinedEvaluation` at
  `z` and its certificate is rejected `BadState` by FR-338 and FR-339; the
  footprint holds `Counter::value` with the `over` receiver only with the
  capture.
