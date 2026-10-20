---
id: SR-13366
title: 'Spec review of PR #1070: ADR-012 RT holds shared NodeKeys, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@e1b5ec811; git diff refs/remotes/origin/main...e1b5ec811 (PR #1070): spec/decisions/ADR-012-semantic-family-extension-contracts.md rows at 875 and 1099; read against main 891825c45 merged with the PR head'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-012
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-013
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-011
  type: reviews
---

## Summary

Ticket: QSL-530. Consistency of the two amended ADR-012 rows with ADR-013 O-04, with ADR-011 T-12 and FR-060 T12-F as merged in #1069 (the PR branch predates #1069, so the check ran on main 891825c45 merged with head e1b5ec811), and a sweep of spec/ for NodeKey hold and construct statements. Sweep result: the only live statements that contradict hold-only are ADR-013 O-04 (:185) and ADR-011 T-12 (:1488). ADR-013 OQ-I (:1247) and ADR-012 SC-Q1 (:1865) already assume `ValueType::Composite(NodeKey)` in RT. FR-060, FR-087, FR-096 and ADR-014 have no RT NodeKey statement. TC-252 and FR-088 'no `NodeKey`' concern the sum shape only, and spec/reviews/ entries are historical records. The rows are accurate about the kernel: quire-exact `Value` does carry `NodeKey`, as RT PR #138's review records.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-012-875-RT-lookup-row | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-012-1099-RT-NodeKey-row | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-013-O-04-public-type | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | examined |
| ADR-013-O-04-enforcement | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | examined |
| ADR-013-O-05-enforcement | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | examined |
| ADR-011-T-12-scan-scope | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-rt-from-bytes | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-emitted-code | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-inspection | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| FR-060-T12-F | spec/functional/FR-060-check-qsl-api-surface-boundary.md | examined |
| arch-lint-Role | tools/arch-lint/api_surface.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-013 O-04 still says 'RT and CG hold no `NodeKey`: they handle only `WireNodeId`s'. Both amended ADR-012 rows now say 'RT may hold shared `NodeKey`s inside `quire_exact::Value`'. Hold-only is not compatible with 'hold none', so the two decision records contradict each other, and ADR-012's rows cite O-04 as their authority. The PR body names this conflict but leaves it to a later change. Amend O-04 in this PR: RT holds shared keys only inside `quire_exact::Value` and constructs none; CG holds none. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:185 |
| FND-002 | medium | ADR-011 T-12, as merged in #1069, still says 'RT decodes them with its own `NodeKey::from_bytes`'. That is an RT-local key type with an RT library constructor. ADR-012 :1099 now says RT keeps 'one kernel with no RT copy' and constructs none. The same cell then lists RT among emitted code that resolves a `WireNodeId` 'without holding a key (ADR-013 O-04 ...)', which reads as RT holding no key. Restate both sentences for the hold-only boundary: RT holds kernel keys inside `Value`, has no local `NodeKey`, and decodes none. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |
| FND-003 | medium | The row is titled 'how RT obtains `NodeKey`s', but the new text says only that RT may hold keys, constructs none, and that only QSL converts a `WireNodeId`. It never says where a key held in `Value` comes from. The other rules leave no path: no backend has a `decode_admitted` caller (FR-060 T12-F), emitted code resolves without holding a key (ADR-011 T-12), RT constructs none, and RT has no QSL edge. Two implementers could fill this gap differently, for example RT minting keys in harness code, or CG passing keys from the `replay` facade. Name the source, for example values that the QSL `replay` facade or E4 produces and CG passes to RT, or say RT holds keys only in test code until a source exists. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1099 |
| FND-004 | medium | Nothing enforces 'RT constructs none in library code', and O-04 says something does. O-04 (and O-05 at :197) says the T-12 API-surface check 'scans every crate that depends on `quire-exact` and fails any other caller, in RT and CG as in QSL'. Merged T-12 says the QSL-role rules scan only the QSL workspace, quire-contract-runtime#56 'is not an API-surface gate', and 'Until a backend gate exists, #216 and #219 check all four by inspection'. arch-lint has only `Role::Qsl` and `Role::Cg`, and `--qsl` refuses any tree that is not QSL. When O-04 is amended (FND-001), state the true enforcement: by inspection under #216/#219 until a backend gate exists. The ADR-012 rows can cite that. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:185 |

## Verdict

Findings: 1 high, 3 medium. Not mergeable: ADR-013 O-04 must be amended in this PR. The ADR-011 T-12 leftovers, the missing source for held keys, and the enforcement claim also need fixing.
