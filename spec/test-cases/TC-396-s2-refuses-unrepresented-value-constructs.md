---
id: TC-396
title: "S2 refuses a declaration holding an expression construct with no parsed-form variant"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-396: S2 refuses a declaration holding an expression construct with no parsed-form variant

## Description

Verify that a construct absent from FR-091's mapping table refuses the whole
unit with `UnrepresentedConstruct`, naming the CST production and span, and
never yields a partial form (ADR-011 §2.3 E2).

Cases 1 to 5 depend on FR-091-OQ-5. Cases 6 to 8 depend on FR-091-OQ-1.

Scope: FR-091-AC-7.

## Test Procedure

1. For each body below, build an admissible unit whose one function body is
   that construct, and run S2:
   1. `"text"`; 2. `decimal(1, 2)`; 3. `a mod b`; 4. `xs[0]`; 5. `none`;
   6. `deref(r)`; 7. `pre(a)`; 8. `allInstances<Point>(p)`.
2. Repeat case 3 with the construct nested inside `if a then b else a mod b`.

Tag the test `#[trace("FR-091-AC-7", "TC-396")]`.

## Expected Results

- Every case refuses with cause `UnrepresentedConstruct`, returns no
  parsed unit, and names the construct's CST production and exact span.
- Step 2's span is the `a mod b` sub-construct, not the whole body.

## Status

Planned; no test backs this case.
