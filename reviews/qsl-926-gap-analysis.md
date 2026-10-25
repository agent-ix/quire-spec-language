---
id: SR-17196
title: "QSL-926 PR #1524 gap analysis of the on each criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/functional/FR-339-check-an-en-1-component-certificate.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
---

## Summary

Ticket: QSL-926. PR: quire-spec-language#1524 (head a8b073ce6). Plan completion: not assessed. This was run planless from `quoin matrix --repo . --json` (quoin 0.29.1) on the PR head.

The new FR-337-AC-6, FR-337-AC-7 and FR-125-AC-18 are `untagged`, as expected for a spec-only PR, since their tests land with the code. FR-338-AC-10 and FR-339-AC-5 were already untagged on main. FR-126-AC-16 is tagged by four qsl-analyze tests, which use test-built packages and test letters, as its amended text allows. No production code changed.

Against code: `is_emitted` (qsl-semantics/src/check/temporal_clause.rs:1520) still refuses every `on each` clause, and `ModelLetters::activation` returns `True` (qsl-eval/src/simulation/model/letters.rs:271-282). Both are the work QSL-1037 owns. EN-1 liveness still refuses a guarded TP-4 clause `GuardedActivation` (qsl-analyze/src/model_check/liveness.rs:176,283), which FR-126 no longer allows.

Examined units: FR-337-AC-5, FR-337-AC-6, FR-337-AC-7, FR-125-AC-18, FR-126-AC-16, FR-338-AC-10, FR-339-AC-5.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The owner is split. FR-337 Dependencies give "the `on each` activation rules" (S3 and S4) to QSL-926 and only the letter-seam read to QSL-1037. QSL-1037's ticket now scopes S3 keep/check, S4 emission and the letter seam together. So FR-337-AC-6 and FR-337-AC-7 have two candidate owning tickets. Name one owner for the S3/S4 code in FR-337 Dependencies, or split QSL-1037's scope to match. | FR-337:267-268 |

## Verdict

No coverage gap that this PR could close. The new ACs are untagged pending QSL-1037. One low ownership finding.
