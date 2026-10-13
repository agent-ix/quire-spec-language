---
id: SR-7854
title: "QSL-729 PR 788 stale K2 test removal review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@908d4cadc37f9d8ee78f755af74612f996668466; qsl-semantics/tests/it/business_meanings.rs"
review_set: subset
---

## Summary

Ticket: QSL-729. Code review with the rust-review lane over PR 788 (base impl/business-model-meanings). The PR deletes the private `k2()` fixture and the test `full_k2_admission_preserves_twenty_one_declarations` from `qsl-semantics/tests/it/business_meanings.rs`. Read-only review; no builds run, per dispatch.

The deleted fixture was stale against QSpec TC-235 (scalar `datetime` Instant, `ix://quire/native/Int`). The shared fixture `qsl-semantics/tests/support/tc235_k2.rs::k2()` declares the same 21 nodes under `test/orders` with `Instant` targeting `ix://quire/native/Timestamp` and `Money/amount_minor` typed `ix://quire/native/Integer`. No `native/Int` IRI remains anywhere in the head tree.

## Verdict

PASS with findings. Deleting the stale copy is correct and leaves no unused helper: `identity`, `key`, `origin`, `construct`, `definition`, `field`, `operation`, `clause`, `document`, `read_authored`, `normalized`, `original_keys` and `expected_keys` all keep call sites, and every import is still used. The deletion does lose the exact 21-key assertion and the `Money` record-meaning check, because the surviving test `tc235_m01_literal_k2_is_admitted_as_declared` asserts counts only. The module doc comment still describes the removed K2.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | After the deletion no test asserts K2's 21 original declaration keys. `tc235_m01_literal_k2_is_admitted_as_declared` asserts `package.records.len() == 21` and 21 `NormalizeRecord` charges, plus type identities for Money, Fulfilment and OrderStore and none for OrderManagement. A reader that drops one member (for example `OrderLifecycle/NotYetPlaced`) and yields another key still passes. The deleted test also asserted that `Money` reads as `DomainPackageRecord::RecordValueType`; no test on unmodified K2 checks that now. Fix: in M01, assert the sorted `record.key()` list equals the 21 `DeclarationKey`s under `PACKAGE` (Order, Order/order_id, Order/total, Instant, Money, Money/amount_minor, Money/currency, OrderPlaced, OrderPlaced/occurred_at, OrderPlaced/order_id, OrderLifecycle, OrderLifecycle/placed, OrderLifecycle/place, OrderLifecycle/cancel, OrderLifecycle/NotYetPlaced, Fulfilment, Fulfilment/correlation_id, OrderStore, OrderStore/get, OrderStore/save, OrderManagement), and assert `Money`'s record matches `DomainPackageRecord::RecordValueType`. | qsl-semantics/tests/it/tc235_business_meanings.rs:251-320 |
| FND-002 | low | The module doc comment still says "K2 retains TC-235's natives and all declarations; the separately named fixtures isolate rules that need no native alignment", but this file no longer holds K2. Reword it to describe only the isolated fixtures, and point to tc235_business_meanings.rs for K2. | qsl-semantics/tests/it/business_meanings.rs:2-4 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| business_meanings.rs deleted k2() and full_k2_admission_preserves_twenty_one_declarations | examined | Asserted original_keys == the 21 expected keys under test/orders, Money as RecordValueType, normalized(&package, 21), and OrderManagement not a type identity. |
| tc235_business_meanings.rs tc235_m01_literal_k2_is_admitted_as_declared | examined | assert_eq!(package.records.len(), 21, "K2's declared source count"); 21 NormalizeRecord charges; type identities for Money, Fulfilment, OrderStore; no type identity for OrderManagement. |
| tests/support/tc235_k2.rs k2() | examined | Shared K2 with the same 21 nodes; Instant target ix://quire/native/Timestamp; Money/amount_minor ix://quire/native/Integer. |
| business_meanings.rs remaining helpers and imports | examined | All helpers and imports keep at least one call site after the deletion. |
| QSpec TC-235 K2 (qspec 1498c1e) | context_only | `UUID`, `Integer`, `String`, `Boolean` and `Timestamp` are native value types; Money's amount_minor typed `Integer`. |

## Coverage and limits

Trace tags for QSpec-TC-235 and QSpec-FR-154-AC-1 still appear on other tests in business_meanings.rs and tc235_record_event.rs, so no trace id loses all its tests. No production code changed, so gap-analysis does not apply. No builds or test runs, per dispatch.
