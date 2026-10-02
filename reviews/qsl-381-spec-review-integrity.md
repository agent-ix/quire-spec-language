---
id: SR-1071
title: "QSL-381 integrity review of PR 577: depth limits left outside the diff"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@9d240032be4eac1df486d3c224cbd48273053dc5; whole-repo grep for depth/nesting/clamp against ADR-030 D-1 and the amended FR-096, FR-047, NFR-001, NFR-006"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: reviews
---
## Summary

Ticket: QSL-381. Check 1 of the brief: no depth limit left in the PR's
scope. ADR-030 D-1 applies to every stage of every implementation, so the
scope is the whole spec. Grep at 9d240032 finds the artifacts below still
specifying a depth limit or clamp, several contradicting text this PR
amended.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-429 says its test "also covers IR's depth, edge, occurrence, diagnostic and work limits", but FR-096-AC-10 (amended here) drops IR's depth limit. Drop "depth". | spec/test-cases/TC-429-the-i2-reader-locates-its-refusals-and-limits-in-the-artifact.md:48 |
| FND-002 | medium | TC-012 still expects "A unit nested to exactly the nesting ceiling in bracket pairs parses, and one pair deeper returns resource_exhausted naming nesting depth" and cites NFR-001 "Nesting level", which this PR deleted. Rewrite to FR-256's deep-bracket criterion. | spec/test-cases/TC-012-native-readiness.md:22 |
| FND-003 | medium | ADR-012 still specifies a SumCase nesting-depth limit: §16.5 rows say a `case` nested past the checking depth bound returns `StageFailure::Limit` with the nesting-depth kind (FR-062-AC-7 pattern). FR-062-AC-7 is now a node-count test. Amend ADR-012 in place (ADR-030 should list it), and the ADR-011 SV/node_key rows that name `DepthAboveMaximum`/`MAX_CHECKING_DEPTH` as the body-depth bound. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:413,1626,1779; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:715,918,920 |
| FND-004 | medium | The native evaluator's execution contract still preflights "active-depth" and TC-130 still counts "active depth", while FR-008, FR-047 and NFR-006 (amended here) delete active depth and cite that contract for traversal rules. spec/native-runtime/tests.md:131 lists active depth too. | docs/native-runtime-evaluation.md:102,159; spec/test-cases/TC-130-evaluate-positive-length-reachability.md:20; spec/native-runtime/tests.md:131 |
| FND-005 | medium | NFR-008 keeps a temporal formula `depth` limit of 64 and clamps ceilings above the defaults (clamped ceilings even enter result identity, AC-4). Both contradict D-1 items 1 and 5. | spec/non-functional/NFR-008-bound-temporal-evaluation.md:39,79,97,104-105,159 |
| FND-006 | medium | Native-profile FRs keep fixed depth caps: FR-009 "depth 64 ... caller limits able only to lower" and "the native checker also caps ... depth at 64"; FR-002 "token/node/depth budgets" (contradicts FR-256-AC-1's 100,000-deep native parse); FR-016 "depth-64 expansion"; FR-033 node/depth/byte limits; FR-051 JSON and expression depth; FR-052 formula depth. | spec/functional/FR-009-lower-qualified-projections.md:56-63; spec/functional/FR-002-parse-native-units.md:17; spec/functional/FR-016-check-native-clauses.md:112; spec/functional/FR-033-lower-bounded-integer-ir.md:18,61; spec/functional/FR-051-publish-checked-native-handoffs.md:108-109; spec/functional/FR-052-publish-native-temporal-evaluation-owner.md:111 |
| FND-007 | low | Fixed size caps by clamping remain outside the diff: FR-003 ("clamped to the implementation ceiling"), FR-024 ("clamped"), FR-041 and FR-042 ("above-hard requests clamp"), TC-045 ("raise each caller value above its documented hard ceiling"). Same no-caps ruling. | spec/functional/FR-003-format-native-source.md:46; spec/functional/FR-024-read-native-runtime-artifacts.md:44; spec/functional/FR-041-admit-rational-native-model-profile.md:114; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:381; spec/test-cases/TC-045-bound-native-model-projection.md:19 |

## Verdict

Changes requested. Check 1 fails: beyond SR-1070's FR-092 and FR-099
findings, depth limits remain in ADR-012, the native-runtime contract and
TCs, NFR-008 and the native-profile FRs. Clean: FR-096, FR-062, FR-091,
FR-102, FR-106, FR-056, NFR-001, NFR-009, NFR-011, NFR-012 and FR-082/083
at this head carry no depth limit.

## Dispositions

Round 1, reviewed at 76e47d3a96aacde91f7964e872d50dcd87dfbc15.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 16a9154f: TC-429 drops QSL's claim on IR's depth limit. |
| FND-002 | fixed | 99139757: TC-012 uses FR-256's chain criterion. |
| FND-003 | fixed | 16a9154f: ADR-012 §16.5 uses the node-count limit. |
| FND-004 | fixed | 16a9154f: No active depth remains in the native runtime contract, TC-130 or native-runtime/tests.md. |
| FND-005 | fixed | 16a9154f: NFR-008 has no depth limit and no clamp. |
| FND-006 | fixed | 16a9154f: FR-002, FR-009, FR-016, FR-033 and FR-051 hold no depth-64 cap or lower-only limit. |
| FND-007 | fixed | 16a9154f: FR-003, FR-024, FR-041, FR-042 and TC-045 hold no clamp or hard ceiling. |
