---
id: SR-20261103154603
title: "QSL-1522 r1 spec review of PR #2202 (FR-505, FR-497, FR-509 and TC-1300, TC-1287, TC-1304)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2202 head 8bf8bbec7; spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md; spec/functional/FR-497-check-and-emit-carried-quantities-and-their-conversion.md; spec/functional/FR-509-declare-a-channel-s-delivery-count-interval.md; spec/test-cases/TC-1300, TC-1287, TC-1304"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-505
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-497
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-509
    type: references
---

# QSL-1522 r1 spec review of PR #2202

## Summary

Tickets: QSL-1522 (FR-505, FR-497) and QSL-1526 (FR-509). The PR adopts the
QSpec contracts merged as quire-specification #609 and #610. Each changed
statement was checked by id against QSpec origin/main `9b267fba`:

- FR-505 against QSpec FR-154 ("Rational value types", FR-154-AC-42 to AC-47)
  and FR-322 (step 4 rows, FR-322-AC-180 to AC-183 and AC-380 to AC-382).
- FR-497 against QSpec FR-142 ("Carried quantity", FR-142-AC-27 and AC-31),
  FR-149 ("Bounded rationals") and FR-322-AC-382.
- FR-509 against QSpec FR-465 (FR-465-AC-3, AC-4 and AC-10), FR-426
  ("Interval sends", "Interval size", FR-426-AC-19 to AC-21), FR-170
  (FR-170-AC-7), FR-425 (`send-pending`) and TC-378.

What agrees:

- FR-505's six-row first-defect order matches QSpec FR-154 rows 1 to 6.
- An empty `constraints` is the unbounded `Rational`.
- The denominator range now lives in row 6 (bounds). The old contradiction
  is gone: the old text sent a zero denominator to `malformed-declaration`,
  while old AC-2 sent it to `declaration-form`.
- The unit spelling is `ix://quire/unit/<name>`. A bare `m`, `1` or native
  URI refuses `malformed-declaration` at `TypeDefinition.unit`. `nosuch`
  refuses `missing-name` at the node and `TypeDefinition.unit`.
- FR-505-AC-8 and AC-9 match FR-154-AC-44 and FR-142-AC-31.
- FR-497 admits `Rational[..]` with `rational_range` and `rounding` `exact`
  and no mode. FR-497-AC-8 places `rational_out_of_domain` after
  `unit.target-domain` and never `inexact_conversion`.
- FR-509's canonical text, strict reader, interval-step relation, send
  record, `send-pending` key and lazy ascending work charge each match the
  cited QSpec ids.
- The FR-464 delivery citation is replaced by FR-465, FR-426 and FR-170.

## Verdict

**MINOR GAPS.** The adopted contracts are stated correctly. One completeness
gap stands: FR-505 names tuple positions and variant payloads as inputs but
specifies nothing for them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-505 Inputs list "tuple positions and variant payloads" typed by a Rational value type, but no Behavior bullet or AC states their outcome. QSpec FR-322-AC-380 (`TK-36`) admits a bounded Rational value type as a tuple position and a variant payload. FR-322-AC-181 and AC-182 refuse an unconstrained one there at step 1, unlike a field. FR-505's Behavior sentence "a Rational value type with no constraints keeps FR-056's behavior (admitted with no checked type ...)" is unqualified, so it reads as admitting the unconstrained type at those positions too. Add the two position rules and an AC (and TC-1300 rows) for bounded-admitted and unconstrained-refused at a tuple position and a variant payload. | spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:58-61,166-171 |
