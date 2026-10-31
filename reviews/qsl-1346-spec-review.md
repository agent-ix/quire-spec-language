---
id: SR-1918006
title: 'Spec review of PR #1927: FR-396-AC-10 admitted lookups by structural wire id'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@887f2f0ee; git diff origin/main...887f2f0ee (PR #1927): spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md, spec/test-cases/TC-1090-the-replay-facade-returns-the-admitted-record-and-tuple-keys.md; ticket QSL-1346'
review_set: subset
date: '2026-10-31'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1090
  type: references
---

## Summary

Ticket: QSL-1346. PR: quire-spec-language#1927, head 887f2f0ee. Method: spec-review (base). Checked the new row FR-396-AC-10 against FR-396's own body, the code and the tests, and checked the TC-1090 scope extension.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-396-AC-10 | spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md:103 | examined |
| FR-396 enum-lookup refusal rule | spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md:78-79 | context_only |
| TC-1090 scope | spec/test-cases/TC-1090-the-replay-facade-returns-the-admitted-record-and-tuple-keys.md:15 | examined |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-396-AC-10 says all three by-wire methods refuse WrongKind for another node of this package. That holds only for composite_declaration_by_wire. FR-396's body says the enum lookups never refuse WrongKind, the code delegates to enum_declaration and enum_case (which return Unknown), and the test enum_by_wire_never_refuses_wrong_kind asserts Unknown for a record id. A test tagged to AC-10 therefore contradicts AC-10's literal text. Fix: scope the clause, for example "and, for composite_declaration_by_wire, WrongKind for another node of this package (the enum lookups refuse Unknown for it)". | spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md:78-79, 103; qsl-replay/tests/admitted_wire_lookup.rs enum_by_wire_never_refuses_wrong_kind | wrong-requirement |

## Verdict

Not mergeable until FND-001 is fixed. It is a one-clause wording change in AC-10. The code and tests are correct. The TC-1090 scope extension to AC-10 is clean.
