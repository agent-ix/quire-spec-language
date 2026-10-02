---
id: TC-770
title: "Library outcomes serialize to one outcome document, and the CLI writes it unchanged"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: verifies
---
# TC-770: Library outcomes serialize to one outcome document, and the CLI writes it unchanged

## Description

Verify the outcome document's members, its determinism, and where the label `undefined` may appear.

Scope: FR-286-AC-1 to FR-286-AC-4.

## Test Procedure

1. Serialize the `check` outcome over `tests/fixtures/spine-compile.native`.
2. Serialize the `check` outcome of FR-100-AC-5's `inv` source.
3. Serialize TC-762 step 3's `analyze` outcome twice.
4. Serialize TC-763 step 5's `analyze` outcome, the outcome of FR-283-AC-5's `monitor` request, and the `execute` outcome of FR-100-AC-10's empty `sum`.

Tag the tests `#[trace("TC-770", "<AC id>")]`.

## Expected Results

- Step 1: `format` `quire-outcome/1`, `operation` `check`, `last_stage` S4, `category` success, no diagnostics, `artifacts` holding the `package_id`.
- Step 2: `category` refusal and one diagnostic whose cause, `ill_typed` code, `Locus` and message equal the failure's.
- Step 3: three `items` in request order with their records and categories; the two serializations are byte-equal.
- Step 4: the `analyze` item and the `monitor` clause record each have category violation and cause `UndefinedEvaluation`, and neither document holds the label `undefined`; the `execute` document has category undefined and the label `undefined`.
