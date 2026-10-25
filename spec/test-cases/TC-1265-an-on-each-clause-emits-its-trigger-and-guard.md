---
id: TC-1265
title: "An on each clause emits its trigger and guard, and S3 checks the guard"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: verifies
---
# TC-1265: An on each clause emits its trigger and guard, and S3 checks the guard

## Description

Verify that S3 checks an `on each` activation's `when` guard with the
existing causes, and that S4 emits an `on each` clause, guarded or not, as
QSpec FR-370's `temporal`/`temporal_clause` node, whose activation argument
carries the trigger binder's own parameter node and, only when written, the
guard term, so a guard-less clause and a `when (true)` clause have
different keys.

Scope: FR-337-AC-6 and FR-337-AC-7.

## Test Procedure

Use FR-124-AC-1's `Counter` unit with `over (c: M::Counter)` under
infinite-trace and the formula `always holds(c.value <= 1)`.

1. Compile the clause `on each (q: M::Counter) when (c.value >= 3)` and read
   the clause node's activation argument, the `q` parameter node and the
   guard term; read the package back through the I2 reader.
2. Compile the same clause `on each (q: M::Counter)`, `on each (q:
   M::Counter) when (true)` and `on origin`; compare the four clause keys and
   `package_id`s. Compile step 1's clause with `when (c.value >= 2)`, with
   the binder renamed `r`, and reformatted; compare the keys.
3. Compile the guards `c.value`, `q.value = 1`, `c.value = c.value + 1` and
   `6 mod (2 - c.value) = 0`; the binder type `M::Absent`; a clause whose
   guard is `c.value` and whose `holds` condition is `c.value`.
4. Compile an `on each (q: M::Counter)` clause whose body begins
   `capture n: Int[0, 3] = c.value;`.

Tag the tests `#[trace("TC-1265", "FR-337-AC-n")]`.

## Expected Results

- Step 1: no `NotEmitted` refusal; the activation binds `kind` `each`,
  `trigger` a reference to the `value`/`parameter` node named `q` at level 1
  typed by `Counter`'s type node, and `guard` the term of `c.value >= 3`,
  whose field read references `c`'s level-0 parameter node; the I2 reader
  admits the package and returns the clause's triple and `over` node.
- Step 2: the guard-less clause binds `kind` and `trigger` and no `guard`;
  `when (true)` binds `guard`; `on origin` binds `kind` `origin` alone; four
  different clause keys and `package_id`s. The changed guard and the renamed
  binder each change the clause key; reformatting changes neither.
- Step 3: `ill_typed`/`non-boolean-root` at the guard;
  `missing_declaration`/`missing-name` at the read of `q`;
  `undefined_expression`/`unproved-range` at `c.value + 1`; the `mod` guard
  checks; `missing_declaration`/`missing-name` at `Absent`; one refusal, at
  the guard. No refused clause gives a package.
- Step 4: `unsupported_construct`/`not-yet-implemented` (`NotEmitted`) at
  the clause.
