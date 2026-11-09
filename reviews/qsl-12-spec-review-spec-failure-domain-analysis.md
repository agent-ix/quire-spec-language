---
id: SR-974
title: "failure domains of the L5 phase-4 matrix amendments"
type: SpecReview
analysis: failure-domain
date: 2026-11-09
scope: "agent-ix/quire-spec-language; branch spec/l5-phase4-matrix-amendments 0c98234bb vs origin/main 52c0c7239; spec/decisions/ADR-035-jit-code-generator-selection-criteria.md, spec/functional/FR-265-derive-a-state-clause-separating-witness.md, spec/functional/FR-281-analyze-claims-with-in-process-engines.md, spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md, spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md, spec/functional/FR-565-publish-the-lifecycle-matrix-cases-the-driver-cli-half-runs.md, spec/functional/FR-566-keep-qsl-outcomes-unchanged-by-a-registered-process-plugin.md, spec/functional/FR-567-qualify-qsl-on-the-final-path-the-driver-calls.md, spec/functional/FR-568-confine-unsafe-code-in-qsl-jit-to-one-module.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-035
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-265
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
# SR-974: failure domains of the L5 phase-4 matrix amendments

## Summary

Ticket: QSL-12. Spec-only review of `git diff origin/main...HEAD` (9 files). Method: spec-review/spec-failure-domain-analysis.

Examined: FR-281, FR-281-AC-3, FR-281-AC-10, FR-294, FR-560, FR-560-AC-5, FR-560-AC-6, FR-564, FR-564-AC-4, FR-564-AC-6, FR-564-AC-8, FR-565, FR-565-AC-1, FR-565-AC-2, FR-566-AC-1, FR-567-AC-5, FR-568, FR-568-AC-1, FR-568-AC-4, ADR-035, FR-265.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The seventh entry, "a clause entry whose evaluation records the FR-265 trail", is undefined: FR-265 defines stop reports, not a trail; stop reports exist only when a quantifier stops early, which the `forall`/`exists` rows already cover; and FR-294's `evaluate` returns only the `evaluation` member, so a backend never produces a trail. The entry has no source construct, so `location` "that construct's" has no value. As written no test can build this case. Remove the row, or define the construct and its location. | spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md:145 |
| FND-002 | medium | "the interpreter, the parity reference (FR-563), then runs such an entry" reads as a runtime fallback, which lead ruling (2) also says ("fall back to the interpreter"). FR-296-AC-3 requires `execute` of a JIT-unsupported entry to return `CallFailure::Unsupported` (exit 21) with the interpreter never called, and QSpec FR-301 forbids a fallback target. Say that only the parity gate runs the interpreter for it, and that `execute` with `jit` returns unsupported. Surfaced: ruling (2) as worded conflicts with FR-296. | spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md:69-72 |
| FND-003 | medium | "The frame the generated code runs over is a typed structure of exec, not an opaque handle" has no AC (AC-1 adds only the shim fixture, AC-4 only the call-target location), and FR-568 does not name `EntryFrame` while ADR-035 does. FR-561 also says kernel entries take "handles to values", which a reader can take as the opaque handle this clause forbids. Name `EntryFrame` and add a checkable clause (for example an arch-lint or compile_fail check that each shim takes `&mut EntryFrame`/`*mut EntryFrame`, not `*mut u8` or `usize`). | spec/functional/FR-568-confine-unsafe-code-in-qsl-jit-to-one-module.md:38-41 |

## Verdict

Not mergeable until the high and medium findings are fixed.
