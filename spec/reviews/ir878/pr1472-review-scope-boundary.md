---
id: SR-17052
title: "IR-878 PR #1472 scope-boundary review of FR-486"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-486
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (scope-boundary) over the spec-only re-derivation of closed #995 as FR-486. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-486 (examined), FR-486-AC-1 (examined), FR-486-AC-2 (examined), FR-486-AC-3 (examined), FR-486 (examined).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-2 says 'a direct QSV caller with no bound retains the behavior specified by QSV FR-122-AC-3'. That is QSV-owned behaviour (QSV FR-122-AC-3 owns the absent-B UnsupportedHelper result) inside a QSL Test criterion, and a QSL test cannot verify it. #995's round 2 removed exactly this kind of cross-owner clause. Move it to Dependencies. | FR-486-AC-2:41 |
| FND-002 | low | AC-3's second sentence ('The evaluator-effect allocation follows quire-exact FR-372-AC-5 and QSV FR-109-AC-8; supplied-admission failures remain attributed to their initiating phase.') points at another owner without adding a QSL check, and restates the AC-1 ownership claim. Drop it or move it to Description. | FR-486-AC-3:42 |

## Verdict

QSL ownership is mostly well bounded. One QSV-owned clause sits inside a QSL Test criterion.
