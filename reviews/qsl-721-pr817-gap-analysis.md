---
id: SR-9045
title: "QSL-721 gap analysis of PR #817"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a4ff9dd248c74995588593666070cb63727eed5b; QSL-721 AC-1..AC-4; QSpec TC-235 rows M01, M02, M03, M04, M13, M14, M17, M18; qsl-semantics/tests/it/tc235_business_meanings.rs; qsl-semantics/tests/it/tc235_record_event.rs; qsl-semantics/tests/support/tc235_k2.rs"
review_set: subset
---
## Summary

Ticket: QSL-721 (parent QSL-59). Plan completion: not assessed. This is a manual mapping of acceptance criteria and TC rows to tests. The TC rows live in the private QSpec repository and are cited here by id only.

AC mapping:

- AC-1 (K2 as declared, FCD-valid, Instant to native Timestamp): met. The fixture is shared in `tests/support/tc235_k2.rs` and is not changed here. M01 admits it after 21 charges and asserts that `Instant` is a `ScalarType` with domain `Native(Timestamp)`.
- AC-2 (each run asserts its stated outcome): met for M02, M03 and M18, and for M04 apart from the documented pointer locus in run 3. M13 and M14 assert binding only; their runtime halves are deferred to QSL-723 by ruling and are documented in the tests. The M02 runtime half is in `tc235_business_meaning_runtime::tc235_m02_record_values_compare_every_field_at_runtime`. M17 run 3 asserts two refusals per FR-082, with a planner ruling to amend the TC row. Gaps: M01 (FND-001), M17 run 1 (FND-002) and M17 run 2 (SR-9044 FND-001).
- AC-3 (mutation check): met through derived notes, per the planner ruling of no production edits. Note accuracy is covered in SR-9044 FND-002.
- AC-4 (no unbacked FR-208 rule claimed): no new rule claim in code. The PR body overclaims (FND-003).

Draft #770 (now CLOSED): its M01 payload block is ported verbatim, which I compared against #770's diff. The rest of #770 is the M10 matrix (`tc235_record_event.rs` +273) and its `intake.rs`/`normalize.rs` changes (field type-reference normalization, relationship owner and target checks). M10 is FR-208-AC-9, which is QSL-724's slice. Nothing QSL-721 needs is lost. The QSL-724 lane must recover the M10 matrix and those production changes from #770, since #770 is closed.

M18 run 1: the engine has no step-admission rule. Conformance of `ExpressOrderPlaced` to the event that `receive_order` consumes is the checkable form of "admitted wherever `receive_order` consumes `OrderPlaced`" at this layer. The new test asserts both halves. It looks up the consumed composite by name rather than through `steps[0].consumes`, which is acceptable.

## Verdict

**Findings.** Two medium gaps against TC rows the ticket lists (FND-001 and FND-002), and one PR-text overclaim (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The M01 payload is not the row's full outcome. The row requires `NotYetPlaced` to be bound "as the guard of `[0]` and as no invariant". Only the guard is asserted; nothing checks that `NotYetPlaced` is not an effective invariant of `OrderLifecycle`. Transitions `[1]` and `[2]` are asserted by key only. Their `from`/`to`/`trigger` (`Draft → Cancelled` on `cancel`, `Placed → Cancelled` on `cancel`) and empty `emits` are not checked, and `emits` is checked only for `[0]`. Step kinds (`event`, `command`, `wait`) are not asserted. Fix: add these to M01. | qsl-semantics/tests/it/tc235_business_meanings.rs:821-882 |
| FND-002 | medium | M17 run 1 does not use the row's input, and the test's "Deviations" note does not say so. The main M17 test retypes `Order/total` to `TaxedMoney`. The row keeps `Order/total` typed `Money` and reads a `TaxedMoney` value there. The new value test uses `k2_record_event_subtype_slice(true)`, which drops `Money/currency`, and it uses `tax_minor` 7 instead of `{amount_minor: 100, currency: "EUR", tax_minor: 20}`. It never involves `Order/total`, despite its name: it checks `admits(Composite(Money), t)`. The slice's reason for dropping currency ("QSL-781 owns native String admission") is stale: `tc235_k2_money_and_order_placed_composites_require_qsl_781` assembles full K2 `Money` with `currency` and passes in the gate. Fix: build `t` and the `Money` value with the row's fields over a slice that keeps currency, and assert admission at the type of `Order/total` as read from the package's field record. Otherwise, record every deviation in the doc. | qsl-semantics/tests/it/tc235_record_event.rs:1139-1151,1356-1403; qsl-semantics/tests/it/tc235_business_meanings.rs:1191-1199 |
| FND-003 | low | The PR body's first line says the PR "Brings the TC-235 runs M01-M04, M13, M14, M17 and M18 to their stated outcomes". It does not: the M13 and M14 runtime halves are deferred (as the body says further down), M17 runs 1 to 3 and M04 run 3 deviate, and M01 is partial (FND-001). Reword it to say which outcomes are asserted. | PR #817 body |

## Dispositions

Re-checked at `8d5b1e93a98bff82094736ca097beb3d16e3389d`: fix commit `8d5b1e93a`. The fix commit adds no new defect.

M01's "no invariant" check applies FR-208's own definition to the engine's data. A state machine's invariants are its owned `quire` clauses that no transition names as a guard. The check asserts that the guard set is exactly `[NotYetPlaced]` and that no other such clause exists. It is the same derivation M06 run 4 already uses. QSL has no other guard-versus-invariant classification at the model layer; state-machine clause lowering is #789's work. The check fails if `NotYetPlaced` loses its guard binding.

M17 run 1 now reads the type of `Order/total` from the assembled `Order` object type, which is `Composite(Money)`. It builds `t` and the Money value from the assembled record fields, using the row's values, including `currency`.

The PR body now names what is asserted and what remains with #789.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8d5b1e93a |
| FND-002 | fixed | 8d5b1e93a |
| FND-003 | fixed | PR #817 body edit (no commit; body read back at head 8d5b1e93a) |
