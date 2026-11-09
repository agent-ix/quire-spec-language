---
id: SR-970
title: "cross-file consistency of the L5 phase-4 matrix amendments"
type: SpecReview
analysis: base
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
# SR-970: cross-file consistency of the L5 phase-4 matrix amendments

## Summary

Ticket: QSL-12. Spec-only review of `git diff origin/main...HEAD` (9 files). Method: spec-review.

Examined: FR-281, FR-281-AC-3, FR-281-AC-10, FR-294, FR-560, FR-560-AC-5, FR-560-AC-6, FR-564, FR-564-AC-4, FR-564-AC-6, FR-564-AC-8, FR-565, FR-565-AC-1, FR-565-AC-2, FR-566-AC-1, FR-567-AC-5, FR-568, FR-568-AC-1, FR-568-AC-4, ADR-035.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The analyze timeout and cancellation cells (matrix row `analyze`) name FR-281-AC-1's request, the EN-5 item that reason 5 says waits for an engine that does not exist. Engines::qsl() holds no EN-5 (qsl-analyze/src/analyze.rs:405-415: EN-1, EN-4, EN-6, value, contract), and analyze settles invalid-request/unknown-backend before it reads the cancel (analyze.rs:1105-1131), so these two applicable cells cannot produce cancelled{source: deadline/requested}. The l5/p3-matrix code already uses an EN-4 statistical item for them. Fix: name an input whose engine Engines::qsl() holds (the EN-4 statistical item) in both cells and in FR-564-AC-4. | spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md:74 |
| FND-002 | high | The item cause `backend-absent` is outside QSpec FR-331's closed `unsupported` cause set (its wire-spelling table lists missing-confidence ... stopwatch-required; `backend-absent` appears nowhere in QSpec), and QSpec FR-331 states an `unsupported` disposition carries its FR-272 diagnostic. FR-281's own rationale ("FR-331's closed unsupported causes hold no claim-kind cause") applies equally to `backend-absent`. The code already emits it (qsl-replay/src/proof_result.rs:738), so either QSpec FR-331 gains `backend-absent` (cross-repo, owner decision) or the AC must not assert a cause outside the closed set. Surfaced against lead ruling (7). | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:372 |
| FND-003 | medium | FR-281 states the diagnostic "FR-286 places on the document's diagnostics member and not on the item (FR-286's item has no diagnostic member)". FR-286 says no such thing: its `items` member carries each item's QSpec FR-331 terminal record, and FR-331's `unsupported` record holds "exactly one of cause and diagnostic". FR-286 needs the matching rule (an analyze/prove disposition diagnostic is written to the document's `diagnostics`) amended in this PR, or FR-281-AC-3/AC-10 cite a rule that does not exist. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:75-82 |
| FND-004 | medium | Reasons 5 and 6 say "the engine that decides the cell's claim does not exist". For the EN-1 cells (analyze violation, analyze incomplete, prove violation/unsupported) the engine does exist: Engine::model_check is in Engines::qsl(). The gap QSL-1819 records is a byte-driven analyze entry for authored-clause items, and QSL-1820's is a byte-driven provider route (provider::run is EN-4 only). State the actual missing piece (no byte-driven input route to the engine), not a missing engine. | spec/functional/FR-564-produce-each-outcome-kind-of-the-lifecycle-conformance-matrix-through-the-library.md:94-100 |

## Verdict

Not mergeable until the high and medium findings are fixed.
