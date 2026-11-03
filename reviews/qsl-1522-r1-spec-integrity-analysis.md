---
id: SR-20261103154604
title: "QSL-1522 r1 integrity analysis of PR #2202 (citations, traces and cross-artifact consistency)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #2202 head 8bf8bbec7; FR-497, FR-505, FR-509, TC-1287, TC-1300, TC-1304; grep of spec/, docs/, plan/ for stale delivery and Rational text"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-497
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-509
    type: references
---

# QSL-1522 r1 integrity analysis of PR #2202

## Summary

Tickets: QSL-1522, QSL-1526.

- `make check-spec-validation` (quire 0.36.2, engine 0.50.2) exits 0 and
  raises no warning on the six changed files.
- Every cited QSpec vector id exists on QSpec origin/main: `CQ-W16` to
  `CQ-W19`, `CQ-E21` to `CQ-E26`, TC-981 `RV-01` to `RV-12`, `TK-36`, and
  TC-947 `CH13a`, `CH13b` and `CH14a`.
- The FR-509 claim that Contract IR's strict reader refuses a non-canonical
  delivery text holds: quire-contract-ir main `ChannelDelivery::parse`
  admits only canonical naturals with `lo <= hi`.
- The FR-509 claim that the legacy `qsl-package/src/protocol_view.rs`
  `delivery` parser trims and accepts non-canonical text also holds, as does
  the claim that `emit/tests/protocol_view.rs:879` admits `[ 2 , 2 ]`.
- Stale-text grep of spec/, docs/ and plan/ found no FR-464 delivery citation
  and no "Ratio type node" text. No `operator-ineligible` wording for a
  bounded Rational survives outside the intended bare-native and unbounded
  case.
- FR-383 and FR-218 still list the four policy words. That is consistent,
  because FR-509 extends them.

## Verdict

**MINOR GAPS.** There are three trace or citation defects and no
contradiction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-497-AC-3 cites `CQ-W17` to `CQ-W19` as conversion-lowering vectors, and TC-1287 step 2 says to "lower the conversions of ... `CQ-W17` to `CQ-W19` and compare the `result_type` node and `mode`". Only `CQ-W18` is an admitted conversion. `CQ-W17` is a node-shape refusal and `CQ-W19` a mode-mismatch refusal, both strict-reader (IR leg) cases like `CQ-W11` and `CQ-W12`, which TC-1287 step 1 already marks out of scope. Step 2 cannot be run on them as written. Cite `CQ-W18` in AC-3 and step 2, and add `CQ-W17` and `CQ-W19` to step 1's out-of-scope IR-leg list. | spec/functional/FR-497-check-and-emit-carried-quantities-and-their-conversion.md:142; spec/test-cases/TC-1287-carried-quantities-check-lower-and-convert.md:22-28 |
| FND-002 | low | FR-497-AC-1's setup declares only `dimension Length;` and `unit m`, yet it asserts `Quantity<m, Rational[-3, 7; 1, 12]>` is distinct from `Quantity<cm, Rational[-3, 7; 1, 12]>`. `cm` is undeclared in that fixture, so the `cm` form refuses `missing-name` instead of checking as a distinct type. Declare `unit cm : Length = rational(1, 100);` in AC-1, as AC-8 does. | spec/functional/FR-497-check-and-emit-carried-quantities-and-their-conversion.md:140 |
| FND-003 | low | FR-509 Dependencies "Upstream: ... TC-383 (queue semantics)" (a line this PR edits) cites a test case that is not the queue semantics. QSL has no TC-383, and QSpec TC-383 covers terminal members and wait causes. Queue semantics are QSpec TC-378, which the Contract bullet already cites. Drop TC-383 or replace it with TC-378. | spec/functional/FR-509-declare-a-channel-s-delivery-count-interval.md:160-161 |
