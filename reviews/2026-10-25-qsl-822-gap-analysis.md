---
id: SR-17210
title: "Scoped gap analysis — QSL-822 invariant helper object reads"
type: SpecReview
analysis: gap-analysis
scope: "QSL-822; FR-366; qsl-semantics/src/check/state_clause.rs; qsl-semantics/src/check/mod.rs; qsl-eval/tests/it/type_invariants.rs"
review_set: subset
review_date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: references
---

## Summary

The touched-scope review checked FR-366-AC-10 against the checked call-graph
walk, model-family typing, and the tagged regression. The test exercises the
direct and transitive read refusals, named predicate refusal, and non-reading
reference forwarding and identity cases.

## Verdict

**PASS** — no in-scope traceability or implementation gap was found.

## Coverage

- FR-366-AC-10 is tagged by
  `a_record_invariant_follows_only_object_reading_helpers` in
  `qsl-eval/tests/it/type_invariants.rs`.
- The scoped matrix included FR-366 and its TC-1050/TC-1051 records with the
  touched test file. Every FR-366 criterion is tagged; the new AC-10 binding
  appears at line 1540.
- Changed checker behavior is owned by FR-366-AC-10. No changed behavior in
  the three-file diff was found without an owning requirement, and no stub or
  coverage inflation was found.
- No repository-wide reverse-gap sweep was run; scope is limited to the ticket,
  FR-366, and the three changed files.
- No `AssuranceProfile` applies in `spec/assurance/`.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found | - |
