---
id: SR-13365
title: 'Spec review of PR #1070: ADR-012 RT holds shared NodeKeys, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@e1b5ec811; git diff refs/remotes/origin/main...e1b5ec811 (PR #1070): spec/decisions/ADR-012-semantic-family-extension-contracts.md rows at 875 and 1099; read against main 891825c45 merged with the PR head'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-012
  type: reviews
---

## Summary

Ticket: QSL-530. Wording, behaviour-not-changelog and EARS for the two edited ADR-012 rows. Both rows state the boundary as behaviour with a reason ('preserving one kernel with no RT copy'); there is no build-log wording. Neither row is an FR or NFR requirement statement and no EARS statement changed, so EARS does not apply. No AC or TC is needed in QSL: QSL's own constructor rules are unchanged. make check-spec-validation exits 0 (17 warnings, none in ADR-011/012/013) and tools/check-index-completeness.sh passes on main 891825c45 merged with the head; git diff --check is clean.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-012-875-RT-lookup-row | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-012-1099-RT-NodeKey-row | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| FR-060-T12-F | spec/functional/FR-060-check-qsl-api-surface-boundary.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | 'constructs none in library code' uses a term no other QSL rule uses. The QSL constructor rules (FR-060 T12-B/T12-F) exempt test code (`#[cfg(test)]`), and RT also runs CG-generated harness code, which is not RT library code. A reader cannot tell whether RT binaries, benches or generated harnesses may construct a key. Say 'non-test code' (the T12-B exemption), and say which code the rule covers. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:875 |

## Verdict

Findings: 1 low. Wording is behaviour, not changelog; the blocking findings are in SR-13366.

## Dispositions

Round 1, head c6ac33eb9 (fix commit c6ac33eb9 over merge b4c476a28 of main 891825c45).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
