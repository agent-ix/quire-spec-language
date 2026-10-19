---
id: SR-12195
title: "Code review of quire-spec-language PR #1041: QSL-537 E11b linear path-pair search for unique path (FR-168)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@56c681e961845a1305d30d28cc275f1478baa3d0; qsl-analyze/src/model_check/state_graph/decide.rs, qsl-analyze/tests/it/graph_world.rs, qsl-analyze/tests/it/state_graph.rs; FR-168"
review_set: subset
---
# Code review of quire-spec-language PR #1041

## Summary

Ticket: QSL-537. PR quire-spec-language#1041, branch `qsl-537-e11b`, one
commit. Scope is `git diff origin/main...HEAD` only. Code review with the
Rust lane folded in. Reviewer-only: no build, no test run, no clippy. The
equivalence argument below is by reading.

This is a pure performance refactor. `Reader::path_pair` used to build
every deviating candidate path in full and compare them with
`(len, ranks(path))`, where `ranks` walked each source's `outgoing` list
with `position`. That is quadratic or worse. It now does one scan that
keeps a small `Deviation { length, position, edge, below }` and builds only
the chosen second path. `ranks()` is deleted. No outcome or evidence
changes are intended, and none follow from the argument below.

### Equivalence with the old algorithm

The candidate set is unchanged. For each position `p` of `first`, each edge
`e != first[p]` out of that position's node with `h_edge(e)` and finite
`d_y[target]`. Old candidate: `first[..p] ++ [e] ++ descend(target)`.

- **Length.** `d_y` is the BFS distance over the `h_edge` subgraph
  (`backward_over`). Every node at finite distance `k > 0` therefore has an
  admitted edge to a node at `k - 1`, so `descend_edges` never takes its
  `break` and returns exactly `d_y[target]` edges. The old length was
  `p + 1 + d_y[target]`, which equals `Deviation::length`. The prefix is
  `first[..p]`, `p` edges, so there is no off-by-one.
- **Order on equal length.** Two candidates at the same `p` share their
  prefix and differ at `p`. Their edges come out of the same node and have
  distinct positions in `outgoing`, so the lesser is the one scanned
  first. The new rule keeps the incumbent on a same-position tie. Correct.
  Two candidates at `pA < pB` agree before `pA`. At `pA`, A takes `eA` and
  B takes `first[pA]`, so A is the lesser exactly when
  `rank(eA) < rank(first[pA])`. That is A's `below` flag, which is set
  while scanning `outgoing` before reaching `taken`. Because the scan runs
  in position order, `candidate.position >= current.position` always holds.
  The new rule replaces the incumbent on a later position exactly when
  `!current.below`. Correct.
- **Distinctness.** No two `(p, e)` pairs yield the same edge sequence, so
  the old strict `<` never met a true tie. The new rule is a total order
  over the same set, and the scan keeps its minimum.
- `below` is computed over the whole `outgoing` list, filtered edges
  included. That matches the old `ranks()`, which used the position in the
  unfiltered list. `taken` is always in `outgoing[source]`, because
  `descend_edges` picks from that list, so the old
  `unwrap_or(usize::MAX)` arm was dead and deleting it loses nothing.

### Other checks (clean)

- **Edges into open nodes and unreachable targets.** `path_pair` runs only
  at a known node (`reaches_open == FAR`), so no admitted edge reaches an
  open node. The `d_y[target] == FAR` skip is kept and is the same as
  before. The `count-2-node-has-a-second-path` fault path is unchanged.
- **Overflow.** `position + 1 + d_y[target]` is computed only after the
  `FAR` check, so every term is at most the node count.
- **Determinism.** Only `Vec`s indexed in canonical `outgoing` order. No
  hash maps.
- **Linearity (FR-168).** `first` is a shortest path with strictly
  decreasing `d_y`, so it is simple. The scan therefore visits each node's
  `outgoing` at most once: at most E edges in total, with O(1) work and an
  O(1) `h_edge` per edge. The final `descend_edges` is at most E. There is
  no hidden quadratic. The old per-candidate `to_vec`, `descend` and
  `ranks` (O(len·deg) each) are gone.
- **Tests.** `fork_world` and `ladder_world` now share `moves_world`. The
  `Fork` world it builds is the same: same package, alias, object, range 4
  and contract text, with `F::Fork::{name}` produced from `{alias}::{object}`.
  The new `Ladder` test's expected value is derived by hand in its doc
  comment, and that derivation is right: `d_y` is 2/1/1/0 for pos 0/1/2/3.
  `first = a,c`. `b,e` (pos 0, above) and `a,d` (pos 1, above) both have
  length 2. The ranks are `[1,0]` and `[0,1]`, so `a,d` is the lesser. The
  premise assertions (`edge_names` at pos 0 and 1) pin the canonical order
  that the derivation relies on. Under an "earliest deviation wins" rule
  the test would pick `b,e` and fail. See FND-001 for the half of the rule
  it cannot fail on.
- **The unchanged AC-3 test** (`unique_path_counts_paths`, TC-593 /
  FR-168-AC-3) still runs the path pair. Unsequenced, the second path is a
  position-0 deviation of the shortest length. Sequenced with `reset`, it
  is the position-1 `reset` deviation of length 5, which beats the
  position-2 `reset` of length 6. So it covers the strict-length
  comparison and the prefix-plus-descend rebuild. It does not cover any
  equal-length tie.

## Verdict

**CONDITIONAL**: correct and linear, with no major finding. FND-001 is a
mutation-surviving branch in the new tie rule. FND-002 is the missing
equivalence check that a pure refactor of this kind should carry. Neither
blocks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test can fail on the `below` half of the tie rule. Delete `&& !current.below` and every test stays green: `Ladder` has both deviations ranked above (`below = false`), and AC-3 has no equal-length tie. The case the conjunct exists for is not tested: an earlier deviation by an edge ranked *below* the first path's edge, tied in length with a later one. Such an edge cannot lead one step nearer, or `first` would have taken it, so the tie needs a sideways edge. Example: node 0 at `d=2` with edges `s` (rank 0, to a node X at `d=2`) and `t` (rank 1, to Y at `d=1`); `first = t, …`. A deviation `s, …` has length 3, and a sideways edge out of Y to another `d=1` node also gives length 3. The correct answer is the `s` deviation; "later always wins" picks the other. Add a world of that shape, such as a second `moves_world` fixture. | qsl-analyze/src/model_check/state_graph/decide.rs:500-505; qsl-analyze/tests/it/state_graph.rs:980-1023 |
| FND-002 | low | A pure performance refactor ships without an equivalence check against the algorithm it replaces. Only two worlds (Ladder, and job with and without reset) exercise `path_pair`. Keeping the old build-every-candidate `(len, ranks)` comparison as a `#[cfg(test)]` oracle, and checking it against the new scan on the existing worlds plus a few hand-built `ExploredStateGraph`s (as the `decide.rs` unit tests already build), would have caught FND-001 and any later drift. | qsl-analyze/src/model_check/state_graph/decide.rs:471-519; qsl-analyze/src/model_check/state_graph/decide.rs:680 |
| FND-003 | low | The new `Ladder` test carries no `#[trace]`, while every neighbouring FR-168 test does. FR-168's Behavior states the second-path rule ("least, by length and then canonical edge order, of the paths that first deviate"), but no AC states the tie, so there is nothing for the test to bind to. See SR-12196 FND-001. | qsl-analyze/tests/it/state_graph.rs:984-985 |

## Dispositions

Round 1, reviewed at 2ce5815f31f5368bfe7552953bf2ea4573c9e20e: fix commit
2ce5815f3 on top of merge f95dcdef0 of origin/main. The PR's diff against
main is still the three Rust files, FR-168, TC-593 and these reviews.
Checked by reading only. No build or test run (reviewer-only brief).

- FND-001: the `Detour` world. Its `d` is 2/2/1/1/0 for pos 0/1/2/4/5,
  and `first = t, w`. At position 0, `s` ranks below `t` and gives
  `s, u, x` (length 3). At position 1, `y` gives `t, y, x` (length 3). With
  `&& !current.below` deleted, the later deviation replaces the earlier one
  and the test fails. `Ladder` still fails under an "earliest wins" rule.
  Between them, both halves of the tie rule can now fail.
- FND-002: `by_candidates` matches the deleted code line for line: the same
  candidate filter, the same `first[..position] ++ edge ++ descend_edges`,
  and the same `(len, ranks)` strict `<`. The one change is that `ranks`
  uses `expect` in place of the dead `unwrap_or(usize::MAX)`. Ties are
  covered deterministically, because the oracle runs on the job, `Ladder`
  and `Detour` shapes, with `outgoing` in list order. In `Detour`, node 0
  has the below-ranked tie and node 2 the above-ranked one. The 2,000
  seeded random graphs (2 to 7 nodes, self-loops and parallel edges) add
  breadth. Whether they contain ties is not counted, which is fine given
  the fixed shapes.
- FND-003: both integration tests carry `#[trace("TC-593",
  "FR-168-AC-10")]`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2ce5815f3 |
| FND-002 | fixed | 2ce5815f3 |
| FND-003 | fixed | 2ce5815f3 |
