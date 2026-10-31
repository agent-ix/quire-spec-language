---
id: SR-1918005
title: 'Gap analysis of PR #1927: admitted lookups by structural wire id (FR-396-AC-10)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@887f2f0ee; git diff origin/main...887f2f0ee (PR #1927, branch qsl-admitted-lookup-by-wire-id): qsl-replay/src/admitted.rs, qsl-replay/tests/admitted_wire_lookup.rs, spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md, spec/test-cases/TC-1090-the-replay-facade-returns-the-admitted-record-and-tuple-keys.md; ticket QSL-1346'
review_set: subset
date: '2026-10-31'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
---

## Summary

Ticket: QSL-1346. PR: quire-spec-language#1927, head 887f2f0ee. Manual criterion-to-test check of FR-396-AC-10 against qsl-replay/tests/admitted_wire_lookup.rs (9 tests, all tagged TC-1090 / FR-396-AC-10). Tests were read, not run (the brief allows skipping the build).

Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-396-AC-10 | spec/functional/FR-396-expose-the-admitted-record-and-tuple-keys-through-the-replay-facade.md:103 | examined |
| admitted_wire_lookup tests | qsl-replay/tests/admitted_wire_lookup.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean for major gaps. Each method has a test per outcome. enum_declaration_by_wire and enum_case_by_wire cover found, Foreign, Unknown (including the named missing case) and Unknown for a record id (no WrongKind). composite_declaration_by_wire covers found, Foreign, Unknown, WrongKind, a nested path and MalformedWire. The test oracle builds the wire id through qsl_foundation::node_wire::node_key_to_text, the emitted package's own spelling, so it is independent of the code under test. Only composite_declaration_by_wire, and only EmptyPath, exercises MalformedWire. That is non-blocking because all three methods share wire_key.
