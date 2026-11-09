---
id: SR-971
title: "integrity of the L5 phase-4 matrix amendments"
type: SpecReview
analysis: integrity
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
# SR-971: integrity of the L5 phase-4 matrix amendments

## Summary

Ticket: QSL-12. Spec-only review of `git diff origin/main...HEAD` (9 files). Method: spec-review/spec-integrity-analysis.

Examined: FR-281, FR-281-AC-3, FR-281-AC-10, FR-294, FR-560, FR-560-AC-5, FR-560-AC-6, FR-564, FR-564-AC-4, FR-564-AC-6, FR-564-AC-8, FR-565, FR-565-AC-1, FR-565-AC-2, FR-566-AC-1, FR-567-AC-5, FR-568, FR-568-AC-1, FR-568-AC-4, ADR-035.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-294's Behavior bullet still says a backend that cannot run a selected entry lists it "with an `Untranslatable` cause". The amended `UnsupportedCause` row and FR-560 add `EnvironmentRefused(ExecutableMemoryRefused)` for exactly that situation, so the two SHALLs contradict. Fix: "with its `UnsupportedCause`" (or name both causes). | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:135-137 |
| FND-002 | medium | FR-294 defines `ExpressionForm(name)` by "its ADR-012 form name". `self.field` and `clause-trail` are not ADR-012 form names: field reads are StateModel forms and `self` is a ProtocolClause form (ADR-012 §15), and a clause trail is not a form at all. Either name the real forms (for example the field-read form and `self`) or add a non-form arm to `UntranslatableConstruct` in FR-294 for the trail. | spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md:80-82 |
| FND-003 | medium | Ceremony: `PendingEngine { verb, kind, ticket: &'static str }` bakes Linear ticket ids into the public qsl-conformance API, and FR-565-AC-1 and FR-564-AC-8 assert those ids (`QSL-1819`, `QSL-1820`). That is tracker metadata under test, not behaviour; the ticket closing would require a code change only to delete a string. The pending/applicable/n-a distinction is the real feature. Fix: carry a `reason: &'static str` as `NotApplicable` does and drop the ticket-id assertions from both ACs. | spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md:38 |
| FND-004 | low | Status prose in a target-design spec: "the operations that run today" (FR-564) and "grow into the coverage by phase" / "joins the coverage in the phase that translates it" (FR-560:70-71, 96-98). State the rule (an applicable cell exists in check, package, monitor or execute for every kind; a construct joins the coverage only with the parity gate holding it) without time words. | spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md:102-106 |

## Verdict

Not mergeable until the high and medium findings are fixed.
