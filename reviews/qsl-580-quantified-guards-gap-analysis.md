---
id: SR-1957035
title: "Gap analysis — QSL-580 bounded quantified clock guards"
type: SpecReview
analysis: gap-analysis
scope: "QSL-580 bounded quantified clock guards; FR-230-AC-55; TC-685 step 33; qsl-semantics and qsl-package timed guard paths"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-685
    type: references
---

## Summary

The changed requirement has focused semantics and package-readback tests, both trace-bound to FR-230-AC-55 and QSpec FR-415-AC-24. The computed repository matrix did not finish within four minutes, so this review does not claim repository-wide trace coverage.

## Verdict

**CONDITIONAL** — the changed behavior is directly tested and manually traced, but the repository-wide computed matrix was unavailable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The computed matrix did not complete within four minutes; repository-wide tagged-criterion coverage remains unverified by this run. | `quoin matrix --repo . --json` |

## Coverage

- FR-230-AC-55 is directly tagged by `clock_places::a_quantified_guard_records_one_entry_per_size_or_slot` and `quantified_clock_guards_emit_ordered_slot_binders`; both exercise production behavior.
- The semantics test asserts each bounded size branch and exact slot constraints. The package test reads emitted typed integer binders and checks their order and use by the constraints, including universal and existential empty-collection behavior.
- `excluded_clause_forms_refuse_at_the_form` remains a focused control for arbitrary data-only alternatives.
- Reverse-gap and stub checks on the changed implementation found no added behavior without an owning requirement and no new stubs.
- Computed matrix: unavailable; the command was interrupted after four minutes of CPU time without output.
- Semantic review: skipped; it was optional and not requested.
- Plan completion: not assessed.
