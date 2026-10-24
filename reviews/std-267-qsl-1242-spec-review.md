---
id: SR-15810
title: "spec-review of S4 state-graph claim emission FR-410 and TC-1140 (quire-spec-language PR 1242)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c3ad3a367; spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md; spec/spec.md; spec/test-cases/TC-1140-a-state-graph-claim-emits-a-v2-claim-node.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-410
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1140
    type: references
---

# Spec review of state-graph claim emission (quire-spec-language PR 1242)

## Summary

Ticket: STD-267 (QSL half, QSL-987; paired with quire-specification PR 370,
SR-15800, which must merge first). One spec-review pass over the PR diff at
c3ad3a3. Oracles: QSL FR-092, FR-093, FR-165, FR-169, FR-170, FR-282,
FR-337, FR-338, FR-362, ADR-022, and QSpec FR-475 and FR-322 at the #370
head.

Gate: `QSPEC_DIR=<#370 head worktree> make check-spec-validation` exits 0
with no finding on FR-410 or TC-1140.

Examined, clean:

- The FR-165 relation. The kind mapping (`Possible`, `AlwaysPossible`,
  `UniquePath`), `over` as `Option`, the from-less constant `true`, and the
  form and predicates entering the node id realise FR-165-AC-1.
- FR-169 identity. "The obligation identity binds the subject, the claim
  and the instance" agrees with FR-475 and the subject-free body.
- The emission FR sits beside FR-337 rather than inside FR-165 or FR-169
  (lead default), which keeps S3 checking and S4 emission apart as for
  temporal clauses. Agreed.
- Reusing `clause-root-unresolved` (lead default) fits: FR-337 defines it
  as a QSL lookup result, not a package refusal. See FND-004 for the
  missing lookup split.
- FR-410 and TC-1140 are unique on main and across the 44 other open PRs.
- No status wording, no compat layer.

Mergeability: the head's merge base is 43f4e20. Main has since moved to
b2c306e (#1222, #1243), so the PR body's "origin/main is an ancestor"
claim is stale. The merge is clean. Open #1253 conflicts on `spec/spec.md`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [major] Outputs say "one `expression` node per distinct predicate, its body the predicate's Boolean term as FR-093 lowers it". FR-093 lowers a Boolean `Literal` (`true`) to a `value`/`literal` node. So the from-less `CanStillWin`, `from (true)` and `possible Anything using v { true }` (AC-2) cannot emit an `expression` node, and FR-475's step 4 refuses a `value` target. This mirrors SR-15800 FND-002 and should follow its fix. | spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md:53-54, :69-72, :92 | wrong-requirement |
| FND-002 | high | [major] Outputs give the node a `declaration` (the claim's qualified name) and say every occurrence has role `claim`. QSpec FR-322 lets a `claim` node carry `declaration` only with a `declaration` occurrence, and with an aggregate body the schema then requires a `SourceOwner`. A declared node also cannot be shared by two declarations, which the "one identity triple per `claim` occurrence, as FR-337" model assumes. This mirrors SR-15800 FND-003, and FR-410 should follow the planner's decision there. | spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md:49-56 | wrong-requirement |
| FND-003 | high | [major] TC-1140 step 4's second edit (and AC-4) points `ReachesTwo`'s `target` at `c`'s parameter node and recomputes only the key. `dependencies` then differs from the body's targets, so the reader refuses `invalid_semantic_graph` at `/dependencies` (FR-322 graph shape, which precedes FR-475's step) and not `ill_typed`/`operator-ineligible`. The edit must also set `dependencies` to the body's targets. This mirrors SR-15800 FND-004. | spec/test-cases/TC-1140-a-state-graph-claim-emits-a-v2-claim-node.md:31-33, :51-52; spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md:94 | wrong-requirement |
| FND-004 | low | [minor] The lookup rule says a triple whose node "is not a `claim`/`state_graph` node" refuses `clause-root-unresolved`. FR-337's rule, unchanged, says a node that "is not a `temporal`/`temporal_clause` node" refuses. Both triples are `[node, "claim", ordinal]`, so the rules agree only if the I2 reader has two lookups, each told the form it expects. State that the state-graph lookup is its own entry, which refuses a temporal clause root and is refused by FR-337's lookup. That also makes AC-3's temporal-NodeId case well-defined. | spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md:81-85, :93 | correct-requirement-no-evidence |
| FND-005 | low | [minor] Certificate context. FR-337 leaves it to the caller whether an unresolved root refuses the request or rejects a certificate, and FR-338 decides that only for EN-1. For `check_state_graph` (FR-169, the FR-282 product-closure checker), FR-410's use case says the settlement binds the identity the request named, but no requirement says what an unresolved state-graph claim root does. It is a request fault by FR-338's reasoning, since a state-graph certificate holds no context roots. Either state it here or defer it explicitly to QSL-891 or QSL-892. | spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md:33-38, :81-85 | correct-requirement-no-evidence |
| FND-006 | low | [minor] The frontmatter `relationships` omit FR-337, whose identity triple and `clause-root-unresolved` this FR reuses, and FR-092 (node keys). Both are named only in Dependencies prose. | spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md:5-17 | correct-requirement-no-evidence |

## Verdict

CHANGES REQUESTED. Three majors block merge. Each mirrors a QSpec #370
major, so they resolve together once the planner decides the claim node's
declaration form and the predicate node form. The FR-165 and FR-169
relations, the lead defaults (a separate emission FR, reuse of
`clause-root-unresolved`, the subject-free body) and the gate are clean.
Merge only after #370. Merge main first; the merge is clean. Expect a
`spec/spec.md` conflict with #1253.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-007 | low | [minor] Small items. FR-410-AC-1 still spells `from (x.phase != Lost)` while TC-1140 step 1 now uses `G::Phase::Lost`; use one spelling. The new `check_state_graph` rule does not say how the refusal settles (FR-338 states "FR-127 settles it `Failed`"), and `StateGraphCheck` has no refusal arm, so say the request is refused before the check. `make check-spec-validation` now warns `ears:unclassifiable` and `ears:missing-subject` at FR-410 line 95. Split the lookup bullet so each `SHALL` has its subject. | spec/functional/FR-410-emit-a-checked-state-graph-claim-as-a-v2-claim-node.md:95-102, :108; spec/test-cases/TC-1140-a-state-graph-claim-emits-a-v2-claim-node.md:25 | correct-requirement-no-evidence |

## Dispositions

Round 1, reviewed at agent-ix/quire-spec-language@783910e6 (fix commit 4e3d151f plus merges of main). Gate: `QSPEC_DIR=<#370 head afe6d822> make check-spec-validation` exits 0. origin/main (6fb751a) is not an ancestor, but the merge is clean. #1253 still conflicts on `spec/spec.md`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4e3d151f |
| FND-002 | fixed | 4e3d151f |
| FND-003 | fixed | 4e3d151f |
| FND-004 | fixed | 4e3d151f |
| FND-005 | fixed | 4e3d151f |
| FND-006 | fixed | 4e3d151f |

Round 2, reviewed at agent-ix/quire-spec-language@6e472651 (fix commit 3b0a0c8b plus merges of main). Gate: `QSPEC_DIR=<#370 head f326f663> make check-spec-validation` exits 0, with no warning on FR-410 or TC-1140. origin/main (59ba945, #1240) is one commit ahead, and the merge is clean. No new findings.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-007 | fixed | 3b0a0c8b |
