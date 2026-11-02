---
id: SR-202611021138
title: "gap-analysis of QSL-1452 priority slice (quire-spec-language PR 2107)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-497-AC-2; FR-497-AC-7; FR-096-AC-8; TC-1287"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-497
    type: references
---

# Gap analysis of QSL-1452 priority slice (PR 2107)

## Summary

Ticket: QSL-1452. Head 707327014773bd0c696815f211b67a7a5a8f360c. FR-497-AC-7 and FR-096-AC-8 are covered for code, cause and
`expected` by `an_inexact_conversion_renders_its_carrier_type` (Decimal and Int carriers, tagged TC-1287, FR-497-AC-7,
FR-096-AC-8). AC-1, 3, 4, 5, 6 and the AC-2 source checks are deferred by the PR body, with nothing dropped.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The only new test covers the refusal record. The new CarriedQuantity behaviour in the other crates has none, although a value of the type is constructible without a source form: the family encoding `carried-quantity` (identity input: Decimal versus Int carrier and mode must differ), unary `-` refusing type-mismatch, and the typed refusals of lowering, witness, wire and SMT. The `=` type-mismatch the PR body cites rests on upstream `check_equality_in`; the in-repo arms in claims.rs and lowering.rs are defensive (lowering answers OperatorIneligible, not type-mismatch) and unexercised. | qsl-semantics/src/check/family.rs:344-364; qsl-semantics/src/check/check.rs:3310 | correct-requirement-no-evidence |
