---
id: SR-727
title: "QSL-245 failure-domain analysis of PR 487 (catalog 1-draft.8 adoption)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@913375e2255351626fdcb6f318b130938d351df9; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md (kernel value refusals, sum rule, OQ-2); spec/functional/FR-001-read-exact-source.md (blank label, OQ-1); code qsl-eval/src/value/expression/evaluate.rs, qsl-semantics/src/value/quantity.rs, qsl-cst/src/diagnostic.rs, qsl-foundation/src/diagnostic.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---

## Summary

Ticket: QSL-245 (PR agent-ix/quire-spec-language#487). This analysis looked
for unstated failure modes in the new `sum` outcome, the value-refusal
payloads and the blank-label check.

- The `sum` rule names the seed and addition failure sites, the location, and
  the charge cut-off.
- The rule "only `CheckMode::Kernel` meets this outcome" holds. Linked
  checking proves the prefix-sum interval (`qsl-semantics/src/check/facts.rs:860-883`).
  Production code never passes `CheckMode::Kernel`; only tests do.
- FR-096-OQ-2 is a real gap in the catalog. QSpec FR-044-AC-8 admits an
  unbounded `Integer`, and the catalog spells a domain only with both bounds.
  QSL cannot reach it today: `QuantityTarget::Integer` always carries a
  two-bounded `IntegerInterval` (`qsl-semantics/src/value/quantity.rs:340-349`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `Undefined::SumOutOfDomain` is a kernel reason. The catalog's "Undefined reasons" table is closed to `absent-key` and `precondition-false`, and so is QSL `UndefinedReason` (qsl-foundation/src/diagnostic.rs:1040). FR-096 says the outcome builds no refusal record, but not whether it names a catalog reason or builds an `UndefinedRecord`. State that it names none, as with `EmptyReduction`. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:267-278 |
| FND-002 | low | An integer-target conversion places into a synthetic `DecimalType` with scale 0..0 (qsl-semantics/src/value/quantity.rs:340-349). If the implementation renders that placement, `expected` comes out as `Decimal[0, 9; 0, 0]` instead of the declared `Int[0, 9]`. The key table and AC-8's `InexactDecimal` example are correct. OQ-2 should also say that QSL cannot reach the unbounded case today, so it does not hold up adoption. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:372-377 |

## Verdict

Approve. Both findings are low and clarify wording. None is a defect in the
adopted rules.
