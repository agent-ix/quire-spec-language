---
id: TC-770
title: "Library outcomes serialize to one outcome document, with the undefined label on non-proof outcomes only"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: verifies
---
# TC-770: Library outcomes serialize to one outcome document, with the undefined label on non-proof outcomes only

## Description

Verify the outcome document's members, its determinism, the `execute` result member, and where the label `undefined` may appear. The CLI half (`quire --format json` writes this document unchanged) is the driver repository's test (FR-286 Overlap).

Scope: FR-286-AC-1 to FR-286-AC-6.

## Test Procedure

1. Serialize the `check` outcome over `tests/fixtures/spine-compile.native`, and the `package` outcome over that `check` outcome.
2. Serialize the `check` outcome of FR-100-AC-5's `inv` source.
3. Serialize TC-762 step 3's `analyze` outcome twice.
4. Serialize TC-763 step 5's `analyze` outcome, the outcome of FR-283-AC-5's `monitor` request, and the `execute` outcome of FR-100-AC-10's empty `sum`.
5. Serialize the `execute` outcomes of FR-100-AC-1's `seven`, FR-100-AC-10's empty `sum` and FR-100-AC-6's `seven` with `accounting` `{"work_units": 0}`, and the outcome of a `check` called with a `Cancel` already cancelled.
6. Call `OutcomeDocument::from_run` over `qsl_replay::spine::run`'s result for: FR-100-AC-1's `seven`; FR-100-AC-5's `inv` source; FR-100-AC-5's `function` `nope`; `seven` with a `Cancel` cancelled with `CancelCause::Requested` before the run; and a constructed `RunRefusal::Fault`. Then build the driver's document for an engine it has not built: `OutcomeDocument::new(Execute, None, Unsupported)` with one diagnostic of code `unimplemented_capability`.

Tag the tests `#[trace("TC-770", "<AC id>")]`.

Until QSL-596 (FR-281) and QSL-597 (FR-283) land the `analyze` and `monitor` outcome types, steps 3 and 4 build their `analyze` and `monitor` items with the public outcome builders; those tickets re-run steps 3 and 4 over their real outcomes.

## Expected Results

- Step 1: the `check` document has `format` `quire-outcome/1`, `operation` `check`, `last_stage` S4, `category` success, empty `items`, `diagnostics` and `artifacts`, and `result` `null`. The `package` document has `operation` `package`, `last_stage` S4, `category` success and `artifacts` holding exactly the emitted `package_id`.
- Step 2: `category` refusal and one diagnostic whose cause, `ill_typed` code, `Locus` and message equal the failure's.
- Step 3: three `items` in request order with their records and categories, causes in QSpec FR-331's wire spelling; the two serializations are byte-equal.
- Step 4: the `analyze` item and the `monitor` clause record each have category violation and cause `undefined-evaluation`, and neither document holds the label `undefined`; the `execute` document has category undefined and the label `undefined`.
- Step 5: `seven` has `category` success and `result` `{"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}`; the empty `sum` has `category` undefined and `result` `{"kind": "undefined", "reason": "sum-out-of-domain"}`; `seven` with `accounting` `{"work_units": 0}` has `category` incomplete and `result` `{"kind": "incomplete", "limit": {"kind": "work_units", "bound": "0", "counter": "1", "field": "work_units", "setting": "accounting.work_units"}}`; the cancelled `check` has `category` incomplete, `last_stage` `null`, `items` `[]` and `result` `null`.
- Step 6: `seven` gives the call's document with exactly one `package_id` artifact; `inv` gives `category` refusal, `last_stage` S3 and one `ill_typed` diagnostic; `nope` gives `category` refusal, `last_stage` S6a and one `missing_declaration` diagnostic; the cancelled run gives `category` incomplete, `last_stage` `null` and one `cancelled` diagnostic with cause `requested`; the fault gives `category` internal failure, `last_stage` `null` and its catalog code. The driver document serializes its diagnostic code as `unimplemented_capability` (not `unsupported_construct`), has `category` unsupported and `last_stage` `null`, and its FR-285 exit code is 21.
