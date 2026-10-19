---
id: SR-13359
title: 'Spec review of PR #1069: ADR-011 T-12 decode carve-out, failure domain'
type: SpecReview
analysis: failure-domain
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

Ticket: QSL-530. Whether a reader set that includes emitted-oracle and generated modules keeps the invariant 'a NodeKey names a node some check minted or an admitted package carries', and whether the backend's own run of the check can enforce the stated precondition. Evidence: CG writes node keys into generated oracle source as byte-array literals (quire-contract-codegen src/oracle/equality/mod.rs:2082-2089, render_key -> `rt::NodeKey::from_bytes([...])`); CG#89 and RT#56 are 'adopt quire-exact' tickets.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ADR-011-T-12-decode-item-2 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-decode-item-3 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-decode-item-4 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-T-12-decode-item-1 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | context_only |
| FR-060-T12-F | spec/functional/FR-060-check-qsl-api-surface-boundary.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Allowing emitted-oracle and generated modules weakens the invariant. A generated call decodes a byte literal that CG wrote into source when it emitted the oracle, not bytes of a package read and checked at that call. So (3) 'the reader calls it only on the bytes of a package that already passed the admitted-package identity check' and (4) 'carries that package check's provenance' do not hold at the generated call site. The spec states no link between the generated literal and the package the oracle runs against. A stale harness, one regenerated against another `package_id`, or a hand-edited one can carry a literal that names no node in the admitted package, and it still passes a call-site allow-list. The text then asserts 'every call in the set decodes bytes of a package that passed the identity check' as fact. Either bind each generated module to the `package_id` it was emitted from, refused when the run's admitted package differs, or have generated code look keys up in the admitted node table. Otherwise keep generated paths out of the set. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |
| FND-002 | medium | The backend's own run cannot state or enforce this set. `arch-lint api-surface` has only `Role::Qsl` and `Role::Cg` (api_surface.rs:95-100), and allowed_callers are compile-time constants in RULES. The CLI takes only --qsl/--cg/--qsl-only, so there is no input for a backend-named allow-list and no RT tree. A Cg scan reads only the checkout's `src/` (api_surface.rs:1241), and CG's oracle code is generated output outside it, so the 'generated paths included' never reach the scan. A call-site allow-list also cannot check (3)'s argument provenance. The tickets named to run it, quire-contract-codegen#89 and quire-contract-runtime#56, are 'adopt quire-exact' tickets with no API-surface gate criterion. Name the mechanism (tool input, which trees, generated-output scan) and a ticket that carries it. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1488 |

## Verdict

Findings: 1 high, 1 medium. Not mergeable as written: the set's generated members break the provenance chain that (3) and (4) rely on, and nothing named can enforce the set.
