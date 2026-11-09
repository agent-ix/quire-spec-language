---
id: TC-1381
title: "The JIT names the first construct outside its coverage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-560
    type: verifies
---
# TC-1381: The JIT names the first construct outside its coverage

## Description

Verify the `Untranslatable` cause the JIT gives for each kind of construct
outside its coverage, and that covered entries beside them are prepared.

Scope: FR-560-AC-3, FR-560-AC-4, FR-560-AC-7 and FR-560-AC-8.

## Test Procedure

1. Pick an unlisted form (`case` if unlisted, otherwise the first unlisted
   ADR-012 form) and an unlisted value type (`Timestamp` if unlisted,
   otherwise the first unlisted K2 type) from `coverage()`.
2. Prepare the JIT over one unit with four entries: `e1` whose first outside
   construct is the unlisted form; `e2` with a parameter of the unlisted
   type; `e3`, covered, that calls `e1`; and `e4`, covered, that calls no
   other entry.
3. Over a property-generated set of entries built only from listed forms and
   types, prepare the JIT; then replace one construct of each entry with the
   unlisted form and prepare again.
4. Prepare a clause entry with no `self` and all constructs listed, and call
   `evaluate`.
5. Prepare an entry with a union-typed parameter and no `case`; read
   `coverage()`'s forms and value types.

Tag the tests `#[trace("TC-1381", "<AC id>")]`.

## Expected Results

- Step 2: `e1` is listed with `ExpressionForm(<form name>)` at the form's
  `Location`; `e2` with `ValueType(<type>)` at the parameter's `Location`;
  `e3` with `Operation(k)`, `k` the node key of its `call`; `e1` keeps its own
  cause; `e4` is prepared and its call returns the interpreter's result.
- Step 3: no generated entry is listed unsupported in the first run; each is
  listed with the form's `ExpressionForm` cause in the second.
- Step 4: the clause is translated, `evaluate` equals the interpreter's
  `Evaluation`, and `unsupported()` is empty.
- Step 5: the entry is listed `ValueType(<union type>)` at the parameter's
  `Location`; `coverage()` lists `name` and no union type.
