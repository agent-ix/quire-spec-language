---
id: TC-1201
title: "The SMT encoding reads every TP-2 formula, guarded clauses and several instances through per-instance monitors"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
---
# TC-1201: The SMT encoding reads every TP-2 formula, guarded clauses and several instances through per-instance monitors

## Description

Verify that `encode_smt_query` encodes monitor items: TP-2 `on each`
items, `on origin` TP-2 items over a nested bounded formula, guarded
(`when`) items under both profiles, and TP-3 items over more than one
instance, with one monitor per instance, the false-extension end letter,
the past-the-end distinctness form and the reading of satisfiable models
by rules 4, 4b and 4c.

Scope: FR-315-AC-33 to FR-315-AC-39.

Steps 2, 3, 5 and 7 read scripts with a `QF_LIA` solver, and read their
models by FR-315's rules with FR-128's replay. Steps 1, 4 and 6 read the
printed script.

## Test Procedure

1. Over FR-315-AC-1's `Counter` under event-position false-extension,
   encode `Unrolling{depth: 2}` for `eventually[0,1] holds(c.value = 3)`
   `on each` twice, then `InductionBase{depth: 1}` and
   `InductionStep{depth: 1}` for it.
2. With a solver, check `Unrolling{depth: 2}` and `Unrolling{depth: 1}` for
   that item, read each model by rule 4b and replay the prefix it gives;
   then check `InductionStep{depth: 1}`, `InductionBase{depth: 2}` and
   `InductionStep{depth: 2}` for `eventually[0,1] holds(c.value >= 1)`
   `on each`.
3. Encode and check `Unrolling{depth: 4}` and `Unrolling{depth: 1}` for
   `always[0,3] eventually[0,1] holds(c.value >= 1)` and
   `always[0,3] eventually[0,1] holds(c.value = 3)` `on origin`, read each
   satisfiable model by rule 4b and replay its prefix; then encode
   `InductionBase{depth: 4}` for each.
4. Encode `Unrolling{depth: 1}` for the guarded
   `always holds(c.value <= 1)` `on each` `when c.value >= 3` under
   infinite-trace, and `Unrolling{depth: 2}` for
   `eventually[0,3] holds(c.value = 3)` `on each`
   `when 6 mod (2 - c.value) >= 0` under event-position false-extension.
5. With a solver, check `Unrolling{depth: 3}` for the guarded TP-1 item of
   step 4, then, with the condition `c.value = 7`, `InductionStep{depth: 2}`,
   `InductionBase{depth: 3}` and `InductionStep{depth: 3}`; then check step
   4's false-extension query. Read each satisfiable `Unrolling` model and
   replay its prefix.
6. Over FR-315-AC-38's two-object `Counter`, encode `Unrolling{depth: 2}`
   and `InductionStep{depth: 1}` for
   `always (holds(x.value = 1) implies eventually[0,1] holds(x.value = 2))`,
   and `Unrolling{depth: 2}` for FR-315-AC-23's TP-3 item.
7. With a solver, take FR-315-AC-39's two models of step 6's
   `Unrolling{depth: 2}` (by asserting each model's steps), read each by
   rules 4 and 4c, and replay the prefix with its `over_binding`.

Tag the tests `#[trace("TC-1201", "FR-315-AC-n")]`.

## Expected Results

- Step 1: the declaration order `s0.c.value`, `m0.0`, `h0`, `s1.c.value`,
  `m1.0`, `h1`, `s2.c.value`, `m2.0`, `h2`; the range assertions,
  `(= s0.c.value 0)`, `(not h0)`, two false-extension step assertions with
  no stutter; position-0 implications for the letters 0, 1 and the end
  letter 2 with the terms `(and (not h0) (not A0))`, `(and (not h0) A0)`
  and `h0`; `(= m<p>.0 r)` then `inc`'s postcondition failure at each
  position; byte-equal scripts. Both induction kinds encode; the step has
  no initial predicate, no `(not h0)`, no position-0 implication and the
  distinctness `(not (and (= h1 h0) (or h1 (= s1.c.value s0.c.value))
  (= m1.0 m0.0)))`.
- Step 2: depth 2 satisfiable, read as the prefix `0, 1, 2`, replayed to
  `refuted`; depth 1 satisfiable and no outcome. For the `>= 1` item, the
  step at depth 1 satisfiable, and base and step at depth 2 unsatisfiable,
  `Inductive{depth: 2}`.
- Step 3: the `>= 1` query at depth 4 unsatisfiable, `BoundedComplete{depth:
  4}`; the `= 3` query at depth 4 read as the prefix `0, 1, 2, 3`,
  replayed to `refuted`; at depth 1 no outcome; both `InductionBase`
  requests refused `query-kind`.
- Step 4: `m0.0`, `m1.0` and no `h<i>`; letter 2's term
  `(and (not C<i>) A<i>)`; the stutter disjunct in each step assertion. The
  false-extension query's condition failures `(and (not h<p>) F)` at
  positions 0 to 2, after each position's violation and before `inc`'s
  postcondition failure, and no atom-failure disjunct.
- Step 5: depth 3 satisfiable, read as the prefix `0, 1, 2, 3` and
  replayed to `refuted`; with `c.value = 7`, the step at depth 2
  satisfiable and base and step at depth 3 unsatisfiable,
  `Inductive{depth: 3}`; the false-extension query satisfiable by the
  condition failure at position 2 only, read as the prefix `0, 1, 2` and
  replayed to `refuted`, `UndefinedEvaluation` at position 2, its site
  naming the activation.
- Step 6: the per-position order `s<i>.c.value`, `s<i>.d.value`, `m<i>.0`,
  `m<i>.1`; instance 0's implications before instance 1's at each position,
  over `c` and `d` respectively; `(= m<p>.0 r)` before `(= m<p>.1 r)`; the
  four-equality distinctness assertion; FR-315-AC-23's item encoded, not
  refused.
- Step 7: the `c.inc`, `d.inc` model fails at position 2 with only
  instance 0 rejecting and is replayed to `refuted` with `over_binding`
  `c`; the `d.inc`, `c.inc` model names `d`.
