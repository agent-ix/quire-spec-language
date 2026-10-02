---
id: SR-1208
title: "Gap analysis of quire-spec-language PR #594: FR-094 unit and dimension nodes to tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a9e332b892cdb40a8d6ec9e8e16f2770a410cb33; PR #594 diff against origin/main; FR-094 'Quantity type nodes' and key-fault list, FR-094-AC-6, FR-093-AC-19, FR-062-AC-9, FR-097-AC-6; tests in qsl-semantics/src/check/lowering/model/tests.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/admission_corpus.rs, qsl-package/src/emit/extent_agreement.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #594

## Summary

Ticket: QSL-247. The ACs the PR touches each have a traced, passing test:
- FR-094-AC-6 / TC-419 (`quantity_types_key_to_the_unit_node_or_a_compound_unit_node`)
  asserts the `metre` unit node, its `Length` dimension and their preimage
  hash.
- FR-093-AC-19 / TC-416 is the admission corpus, now with `dimension` and
  `unit` rows admitted by IR.
- FR-062-AC-9 / TC-160 step 8 (`a_compound_unit_is_omitted_only_for_its_omitted_unit`)
  asserts the exact omitted set.
- FR-097-AC-6 / TC-440 (`tc_440_a_quantity_record_is_emitted_and_reaches_ir`)
  emits, has IR read and lower. Its oracle is strong: IR re-hashes the
  preimages.

The `tc_440_quantity_extent_agrees_with_ir_requires_bound` ignore is ruled
and not raised.

Two branches the PR adds to FR-094's behaviour have no test, and every
fixture uses only root units of base dimensions.

## Verdict

Not mergeable until the two coverage gaps get tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-094's new key-fault bullet has no test: "a declared unit, or a dimension or unit a held unit names, whose nominal preimage the unit table does not hold". `KeyFault::UnheldNominal` is reached by a table built through `FromIterator`/`with_units`, not `UnitTable::declared`, holding a declared unit; no test builds one. | qsl-semantics/src/check/lowering/model.rs:472-474 |
| FND-002 | medium | FR-094 says check builds "every dimension and unit node it names (its dimension, that dimension's base dimensions, its target units)". Every fixture (`units()`, `metre_units_owned_by`) admits only root units over base dimensions. So none of these is exercised: the target-unit and derived-dimension arms of the walk, the emitter's `Dimension` terms and `Unit` target dependency arms, or IR's admission of a source-owned non-root unit or derived dimension preimage. | qsl-semantics/src/check/lowering/model.rs:483-512 |
