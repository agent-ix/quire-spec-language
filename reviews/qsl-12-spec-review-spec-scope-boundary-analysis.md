---
id: SR-975
title: "scope boundary of the L5 phase-4 matrix amendments"
type: SpecReview
analysis: scope-boundary
date: 2026-11-09
scope: "agent-ix/quire-spec-language; branch spec/l5-phase4-matrix-amendments 0c98234bb vs origin/main 52c0c7239; spec/decisions/ADR-035-jit-code-generator-selection-criteria.md, spec/functional/FR-281-analyze-claims-with-in-process-engines.md, spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md, spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md, spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md, spec/functional/FR-566-keep-qsl-outcomes-unchanged-by-a-registered-process-plugin.md, spec/functional/FR-567-qualify-qsl-on-the-final-path-the-driver-calls.md, spec/functional/FR-568-confine-unsafe-code-in-qsl-jit-to-one-module.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-035
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-560
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-566
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-567
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-568
    type: references
---
# SR-975: scope boundary of the L5 phase-4 matrix amendments

## Summary

Ticket: QSL-12. Spec-only review of `git diff origin/main...HEAD` (9 files). Method: spec-review/spec-scope-boundary-analysis.

Examined: FR-281, FR-281-AC-3, FR-281-AC-10, FR-294, FR-560, FR-560-AC-5, FR-560-AC-6, FR-564, FR-564-AC-4, FR-564-AC-6, FR-564-AC-8, FR-565, FR-565-AC-1, FR-565-AC-2, FR-566-AC-1, FR-567-AC-5, FR-568, FR-568-AC-1, FR-568-AC-4, ADR-035, FR-567.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-567's `lifecycle-matrix` part passes when every `Case` passes and says nothing about `PendingEngine` cells, so `qualify` reports `complete-v1: qualified` with seven cells pending. FR-564-AC-8 says no pending cell is reported as satisfied or waived, and the qualify verdict is the surface that would report it. FR-567 must state what pending cells do: name them in the `parts` member as pending, and either withhold `complete-v1: qualified` while any is pending or state that it does not cover them. | spec/functional/FR-567-qualify-qsl-on-the-final-path-the-driver-calls.md:37-40 |

## Verdict

Not mergeable until the high and medium findings are fixed.
