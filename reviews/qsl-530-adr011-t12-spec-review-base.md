---
id: SR-13357
title: 'Spec review of PR #1069: ADR-011 T-12 decode carve-out, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@44eca2272; git diff refs/remotes/origin/main...44eca2272 (PR #1069): spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md T-12; spec/functional/FR-060-check-qsl-api-surface-boundary.md T12-F'
review_set: subset
date: '2026-10-19'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-011
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-060
  type: reviews
---

## Summary

Ticket: QSL-530. Wording, behaviour-not-changelog and TC/AC need for the edited T-12 item (2), the new hex sentence and the FR-060 T12-F row. The text describes behaviour, not a build log (no 'now', 'instead of' or ticket narration). No QSL AC or TC is needed: QSL's T12-F behaviour (no QSL caller) is unchanged and tc_027/028 cover it.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-011-T-12-decode-item-2 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-hex-sentence | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| FR-060-T12-F | spec/functional/FR-060-check-qsl-api-surface-boundary.md | examined |
| TC-157-T12-F-tests | tools/arch-lint/api_surface.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The hex sentence describes 'a plain function in `quire-exact` from exactly 64 lowercase hex digits to `[u8; 32]`' in the present tense, but quire-exact main has no such function (src/ has only the private UUID `hex_digit` in value/native.rs), and no ticket or owner is named for it. A reader looking for the helper the backends should use finds nothing; CG today parses with RT's `NodeKey::from_hex` (quire-contract-codegen src/oracle/equality/mod.rs:1177-1178). State it as a planned quire-exact API with its ticket, or drop the function and keep only 'hex parsing is not a NodeKey operation'. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |
| FND-002 | low | 'the admitted-package reader set: the modules that decode an admitted package, which may be the backend's emitter module or modules and its emitted-oracle module or modules' chains two colons and a permissive 'may be', so it is unclear whether the set is limited to emitter and emitted-oracle modules. FR-060 T12-F's parenthetical '(emitter and emitted-oracle modules, generated paths included)' reads as the full list. Say once which module kinds the set may hold, in the same words in both places. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |

## Verdict

Findings: 1 medium, 1 low. Behaviour wording and the no-new-TC call are sound; the blocking issues are in SR-13358 and SR-13359.

## Dispositions

Round 1, head fd0e59362 (fix commit fd0e59362 over merge e8907f963).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
