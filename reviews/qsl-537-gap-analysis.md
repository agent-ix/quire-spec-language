---
id: SR-12196
title: "Gap analysis of quire-spec-language PR #1041: QSL-537 E11b linear path-pair search (FR-168)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@56c681e961845a1305d30d28cc275f1478baa3d0; FR-168 (Description linearity clause, unique path Behavior), FR-168-AC-3; TC-593; qsl-analyze/src/model_check/state_graph/decide.rs, qsl-analyze/tests/it/graph_world.rs, qsl-analyze/tests/it/state_graph.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1041

## Summary

Ticket: QSL-537. PR quire-spec-language#1041. A planless gap analysis,
scoped to the diff. Plan completion: not assessed. I read the criteria
against the tagged tests. No test was run (reviewer-only brief).

Requirements the diff touches:

- FR-168 Description: "one pass over the explored graph, linear in its
  nodes and edges". The old `path_pair` broke this. It built every
  candidate and ranked each one with `position` scans. The new scan meets
  it (SR-12195, Linearity). No AC states linearity, and this PR adds none.
- FR-168 `unique path` Behavior: "the least, by length and then canonical
  edge order, of the paths that first deviate from it: over every position
  of the first path and every other edge out of that position's node into
  `H`, the prefix, that edge, then the shortest path on". This is
  implemented, with the same candidate set and order as before (SR-12195).
- FR-168-AC-3 (TC-593): `unique_path_counts_paths` is unchanged and
  correctly bound. It still reaches `path_pair` in both `PathPair` cases.

Bindings: `unique_path_counts_paths` to FR-168-AC-3 is correct. The new
`the_second_path_is_the_least_deviation_by_canonical_edge_order` is
untagged (FND-001).

Code with no owning requirement: none. `Deviation` and `moves_world` serve
FR-168 and its fixtures.

Declared and not assessed here, as the brief says: FR-170 replay is blocked
(QSL-892), `Holds{basis}` is plain, and the `StateGraphCertificate` path
counts are QSL-891.

## Verdict

**CONDITIONAL**: no untagged AC that the diff claims, and no hollow
evidence. One low spec gap: the tie rule has no AC.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-168 states the second-path tie rule (length, then canonical edge order over first deviations) only in Behavior. No AC checks an equal-length tie, so the new `Ladder` test cannot be traced, and the `below` case (SR-12195 FND-001) has no criterion behind it. AC-3's two pairs differ from the alternatives in length only. Add an FR-168 AC naming the `Ladder` outcome (second `a, d`, not `b, e`) and a below-ranked earlier deviation case, then tag the tests. | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md:125-131; qsl-analyze/tests/it/state_graph.rs:984 |

## Coverage

- Criteria examined: FR-168-AC-3 (tagged, correct binding). FR-168's
  linearity clause and `unique path` second-path rule (no AC).
- Untagged criteria introduced by the diff: none.
- Stubs or coverage inflation: none.
- Semantic review: the intent, test and code for the path-pair rule were
  read in SR-12195. No separate semantic fan-out was run.
- Plan completion: not assessed
