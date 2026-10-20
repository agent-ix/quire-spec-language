---
id: SR-13358
title: 'Spec review of PR #1069: ADR-011 T-12 decode carve-out, integrity'
type: SpecReview
analysis: integrity
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

Ticket: QSL-530. Consistency of T-12 items (1)-(4) with the edited item (2), with the rest of the T-12 cell, with ADR-013 O-04 and ADR-012, and with the arch-lint tool the text describes. Verified independently: NodeKey lives in quire-exact src/node.rs with two pub constructors, from_digest and decode_admitted; QSL's T12-B pattern is NodeKey::from_digest with check as allowed caller and T12-F has an empty QSL allow-list.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-011-T-12-decode-item-1 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-decode-item-2 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-decode-item-3 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-decode-item-4 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-scan-scope | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| FR-060-T12-F | spec/functional/FR-060-check-qsl-api-surface-boundary.md | examined |
| ADR-013-O-04 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md | examined |
| ADR-012-RT-NodeKey-rows | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Item (2) now names CG's emitter and emitted-oracle modules as `NodeKey::decode_admitted` callers, which contradicts ADR-013 O-04 (ADR-013:185): 'the kernel `NodeKey` has one public constructor', 'RT and CG hold no `NodeKey` ... only QSL (E4 and the layer-6 `replay` facade) converts a `WireNodeId` to a `NodeKey`', and 'a node id read from any wire is a `WireNodeId` ... never a `NodeKey`'. ADR-012:875 and :1099 say the same for RT. The PR body names only the ADR-012 conflict, and only on GitHub; ADR-013 O-04 is not flagged, and neither conflict is recorded in the spec or a tracked item. Amend O-04 (and ADR-012's two rows) to the carve-out, or record each conflict in the spec with its owner, before this lands. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |
| FND-002 | medium | The new reason 'since QSL's rule table scans only QSL crates' is false: rule T12-A has `Role::Cg` and scans the CG checkout passed with `--cg` (tools/arch-lint/api_surface.rs:44-53, 198-230; main.rs usage `api-surface --qsl <path> [--cg <path> \| --qsl-only]`). It also sits in the same cell as 'The API-surface check scans every crate that depends on `quire-exact`, `quire-semantic-value` included', while api_surface.rs:1252-1255 says quire-semantic-value is not scanned. Say which run is meant: T12-F is a `Role::Qsl` rule that scans only QSL's tree. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |

## Verdict

Findings: 1 high, 1 medium. Items (1), (3) and (4) are unchanged and consistent with each other; item (2) conflicts with ADR-013 O-04 and ADR-012 and gives a false reason for where the allow-list lives.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The new sentence says emitted code and RT resolve a key literal 'to a `NodeKey` only by lookup in the admitted node table', and item (2) still gives 'a backend' a module that decodes `NodeKey`s. ADR-013 O-04 (ADR-013:185), which the PR's decision says stands, says 'RT and CG hold no `NodeKey`: they handle only `WireNodeId`s, and only QSL (E4 and the layer-6 `replay` facade) converts a `WireNodeId` to a `NodeKey`'. ADR-012:875 and :1099 say the same for RT. So the sentence written to follow O-04 still contradicts it. The RT side is the IR-882 conflict that the PR body leaves with RT's owner, but that is recorded only on GitHub. Either drop 'to a `NodeKey`' and say the emitted code resolves the `WireNodeId` by lookup without holding a key, or record the O-04/ADR-012 conflict and its owner in the spec. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |

## Dispositions

Round 1, head fd0e59362 (fix commit fd0e59362 over merge e8907f963).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
