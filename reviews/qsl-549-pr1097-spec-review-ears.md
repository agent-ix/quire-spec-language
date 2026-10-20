---
id: SR-13541
title: 'Spec review of PR #1097: FR-149 meter stops and expansion stop, EARS'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@100f4dc1d; git diff refs/remotes/origin/main...100f4dc1d (PR #1097, base main 7e28a47ae): spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md; ticket QSL-549'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-149
  type: reviews
---

## Summary

Ticket: QSL-549. PR: quire-spec-language#1097, head 100f4dc1d, scoped to `git diff refs/remotes/origin/main...100f4dc1d` (one commit, one file, +9/-5). Spec-only. EARS review of the changed Meter stops and Expansion stop bullets and of FR-149-AC-5. The grammar lint (make check-spec-validation, exit 0, 17 warnings, as on main) flags nothing in FR-149. Methods: spec-review/base (SR-13540) and spec-review/ears (SR-13541).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-149-meter-stops | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-expansion-stop | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-149-AC-5 | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Expansion stop bullet is compound again. It has two SHALLs (a named-limit stop gives `LimitReached`; an unnamed, non-internal stop gives `ResourceExhausted`), each separately testable, with a descriptive middle sentence ('a concrete contract ... stops the expansion this way ... because FR-120 ...') between them. Round 1 of SR-13481 (FND-002) split the Limits bullet to one SHALL per bullet, and this brings the pattern back. The lint did not flag it. Fix: split it into 'Named expansion stop' and 'Unnamed expansion stop' bullets, keeping the FR-120 sentence as the first bullet's rationale. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:196-202 |

## Verdict

One low finding: the Expansion stop bullet holds two separately testable statements. Meter stops is now a well-formed event-driven statement: 'When the verdict ... is an `Undetermined` that is meter-only, the checker SHALL stop ... and not reject', which is one behaviour and its negation. Both new When-clauses have the checker as subject. AC-5 (c)'s added 'which stops the position's successor expansion' says where the stop happens and still asserts a falsifiable outcome.
