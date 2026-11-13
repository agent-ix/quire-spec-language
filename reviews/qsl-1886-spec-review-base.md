---
id: SR-2026111305294500000003
title: "Spec review (base) of QSL-1886 added criteria and test steps"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR 2695 head e7b13c3125a89d3045ce10dc5a0f0dc9a94ac080; FR-207-AC-7, FR-209-AC-9, TC-652 step 7, TC-654 step 9"
review_set: subset
---

## Summary

Ticket: QSL-1886. Base checklist review of the criteria and test-case steps the
PR adds. The ids are well formed and sequential (FR-207-AC-7 follows AC-6, and
FR-209-AC-9 follows AC-8). The TC scopes were widened to match, and each new
step has an expected result. Both ACs are testable and each can fail.

## Verdict

**CONDITIONAL**: two low wording and completeness findings on FR-207-AC-7.
FR-209-AC-9, TC-652 step 7 and TC-654 step 9 are clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-207-AC-7's long subject ("a variant of §7.1's `Refund` with ... the recovery predicate `seen and armed` reads ...") reads as if only the recovery predicate reads the capture and binder. Splitting the setup sentence from the claim would make the subject unambiguous | spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md:89 |
| FND-002 | low | FR-207-AC-7 states outcomes for triggers 3, 2 and 1 but not 0, although `Level` is `Int[0, 3]` and `activate(Level)` is enabled for 0 too | spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md:89 |
