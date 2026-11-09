---
id: SR-20261108210000000003
title: 'QSL-1826 gap-analysis: checker cancellation and acceptance'
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826; six files in origin/main...7d7d31bc4b998a7598544fb621fffc07de731d97
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-282
  type: references
---

## Summary

Ticket: QSL-1826. Reviewed actual frozen head 7d7d31bc4b998a7598544fb621fffc07de731d97, independently confirmed against GitHub PR 2606. Scope is the six changed Rust files only.

## Verdict

CONDITIONAL: no concrete introduced defect found in this PR slice; computed matrix reconciliation could not be completed. This is not completion of QSL-1826 or full certification provenance. Opaque acceptance cannot be externally fabricated or mutated, but does not bind every public settlement argument.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | implementation-bug-despite-evidence |

## Examined scope

scope:
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: '- The checker''s limits and `&Cancel`.'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  `inconclusive`, `CertificateRejected`; `Stopped` SHALL settle V-7.'
- id: FR-203
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: '| Cancelled through the `Cancel` handle | `incomplete`, `cancelled{source}`
    | `unavailable` | `Incomplete(Cancelled{source})` (ADR-018 V-7) | incomplete |'
- id: FR-281
  path: spec/functional/FR-281-analyze-claims-with-in-process-engines.md
  role: examined
  excerpt: checker in the qualified core accepts its certificate (FR-282). A proof
    from

## Coverage

Reviewed graph reconstruction, claim binding, certificate row validation, topological ordering, Bellman validation, final acceptance poll, cancellation cause propagation and absent evidence on incomplete results. Genuine checker acceptance remains a positive control. Compile-fail examples address external construction and mutation. No source, manifest, CI or gate weakening changes. Existing unsupported shapes, witness replay, rejected vocabulary and byte codec remain outside this slice. No applicable AssuranceProfile found. New production behaviors inventoried: Cancel threading, typed cancellation cause, opaque checker acceptance, and adapter incomplete settlement (4); unowned behaviors: 0; new source/test stubs: 0.

Plan completion: not assessed

Optional gap semantic review: skipped; ordinary code-review behavior/test checks performed. Full repository assurance is not claimed. Reconciliation limitation: quoin matrix --repo . --json was stopped after more than seven minutes at continuous CPU activity with zero output bytes. A redundant static quire matrix fallback was cancelled earlier. Help exposes no criterion/path selector. No computed counts or complete repository coverage are claimed. Manual fallback checked the four governing requirement statements and their actual new test trace attributes (FR-201/FR-203/FR-282, TC-636/TC-763); these are statement-level cancellation tests, not evidence that every AC in those broad requirements is implemented.

Validation: inspected coder logs: Clippy exit 0, replay unit 7 pass, replay integration 5 pass, analyze item integration 9 pass, compile-fail doctests 2 pass. Reviewer diff whitespace check passed; focused independent replay probability-certificate unit rerun passed all 7 tests (exit 0). Aggregate gate intentionally not run by reviewer.

## Scoped mergeability

Mergeable for this cancellation and sealed-acceptance slice under the repository AGENTS owner update permitting engineering delivery while incomplete assurance is tracked separately. Code-review remains PASS; gap-analysis remains CONDITIONAL because the computed matrix did not complete. Manual scoped mapping is complete and no concrete introduced defect was found. This does not close QSL-1826, certify full repository coverage, or resolve terminal metadata provenance.

## Upstream compatibility

Upstream compatibility checked against origin/main 26b3c7c4f028b49e13df7c504620b5c67381cb84, comparing db995d817 to that head. FR-201/FR-282 now explicitly permit the checked-subject checker path; their new broader-form/fresh-process obligations remain the already-excluded follow-up scope. FR-282 still requires &Cancel, FR-281 still gates certified proof on checker acceptance, and its cancellation precedence AC-12 is unchanged. FR-281 adds provision, routing and counterexample obligations without changing this checker cancellation or opaque acceptance contract. The upstream analyze.rs change adds generic EN-1 clause routing and retains analyze_items(..., cancel); it does not alter the reviewed exact adapter/checker. No changed assumption or new in-scope defect found: this slice remains mergeable; code-review PASS and gap-analysis CONDITIONAL stand. This is a read-only compatibility assessment, not a new test run or full acceptance claim for upstream changes.
