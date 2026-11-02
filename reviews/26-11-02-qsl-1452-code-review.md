---
id: SR-202611021137
title: "code-review and rust-review of QSL-1452 priority slice (quire-spec-language PR 2107)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; Cargo.lock; qsl-foundation/src/diagnostic.rs; qsl-foundation/tests/kernel_refusal_record.rs; qsl-replay/src; qsl-semantics/src; qsl-eval/src"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-497
    type: references
---

# Code review of QSL-1452 priority slice (PR 2107)

## Summary

Ticket: QSL-1452. Head 707327014773bd0c696815f211b67a7a5a8f360c, base main. Diff only. Cargo.lock moves quire-exact
to 87e40238 and quire-semantic-value to 25535387 and nothing else; both reachable from upstream origin/main; Cargo.toml untouched.
The InexactConversion arm shares the InexactDecimal arm correctly (same InexactTarget, FR-096 and FR-100 spell both as
Decimal[lo, hi; smin, smax] or Int[lo, hi]). Scalar classification OtherRefusal matches InexactDecimal. The exhaustive-match arms
are consistent. No allow, unreachable, todo or panic added. The slice is intentionally partial.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | `inexact_conversion` is not added to the catalog code maps that every other kernel code is in: CATALOG_CATEGORIES (49 entries) has `inexact_decimal` but no `inexact_conversion`, so `category_of` of the new record's code is None; `kernel_fields` and `kernel_causes` in native_result.rs also lack it. Consequences: wire.rs `refusal()` rejects the code as an unknown spelling, native_result `catalog_category` fails, and explore.rs `stop_category` files it as InternalFailure. FR-497-AC-7 and FR-100-AC-9 require the record to render with exit 20 like the other value refusals; the new test checks only record.category(), which the constructor fixes, never `category_of`. | qsl-foundation/src/diagnostic.rs:707-755; qsl-replay/src/spine/native_result.rs:912-945; qsl-replay/src/result/wire.rs:1257 | implementation-bug-despite-evidence |
| FND-002 | medium | `value_type_source` spells a CarriedQuantity as `Quantity<m, Decimal[-100, 100; 2, 2]; exact>` (rounding appended after the carrier's closing bracket) and `Quantity<m, Int[-2, 2]; nearest-even>`; FR-497 writes the rounding inside the carrier, `Decimal[-100, 100; 2, 2; exact]` and `Int[-2, 2; nearest-even]`. The function is documented as source syntax and feeds the signature-mismatch text. No test covers the new arm, unlike the neighbouring BoundedQuantity and Map cases. | qsl-replay/src/spine.rs:1953-1969 | implementation-bug-despite-evidence |
| FND-003 | low | The second equality unit-fact loop ends in `_ => None`, so the new variant is hidden there while the first loop (check.rs:2957) was given an explicit CarriedQuantity arm; a carried unit would skip the UnheldUnit guard and push no unit fact. Latent while `=` refuses a carried operand, but it is the wildcard the exhaustiveness work should not leave. | qsl-semantics/src/check/check.rs:3012-3016 | implementation-bug-despite-evidence |
