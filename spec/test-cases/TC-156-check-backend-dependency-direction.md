---
id: TC-156
title: "Report FB-05 and FB-11 violations over the four-repository backend dependency graph"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: verifies
---
# TC-156: Report FB-05 and FB-11 violations over the four-repository backend dependency graph

## Description

Verify that `arch-lint direction` classifies resolved packages by repository,
reports every FB-05 edge into QSL other than CG's normal dependency, reports
every FB-11 cycle over the combined normal+dev edge graph exactly once, and
reports no IR → QSL edge and no QSL ⇄ IR cycle when run against
quire-contract-ir's real current-head `cargo metadata` output, that resolving
QSL's own manifest classifies its real edge on `quire-contract-model` as
QSL → IR, and
that a stale `--ir`/`--rt`/`--cg` clone is rejected before the edge graph is
even built, and that the shared leaves `quire-exact` and
`quire-semantic-value` classify as no ecosystem repository. Scope: FR-059-AC-1
through FR-059-AC-9.

## Test Procedure

1. Build a synthetic four-repository graph with only CG's normal dependency on
   QSL and no cycle. Run the check.
2. Build a graph with a normal edge from IR into QSL, then a separate graph
   with a dev edge from RT into QSL. Run the check on each.
3. Build a graph with a dev edge from CG into QSL (no normal edge). Run the
   check.
4. Build a graph with a 2-repository cycle (QSL depends on CG, normal edge,
   and CG's own manifest carries a dev edge back to QSL beyond the stated
   exception) and, separately, a longer cycle spanning all four repositories
   over a mix of normal and dev edges. Run the check starting the search from
   each of the four repositories in turn.
5. Build an acyclic graph with edges only running toward QSL and CG (no edge
   originates from QSL or CG). Run the check.
6. Run `arch-lint direction` against real `cargo metadata` output for QSL's
   own manifest and a real local checkout of quire-contract-ir's current head.
   Separately, resolve QSL's own workspace manifest through
   `edges_for_manifest`, the path `direction` uses for `--qsl` (automated).
7. Call the freshness comparison directly with a resolved revision that does
   not match a captured remote `main` head (the reviewer's real repro:
   quire-contract-codegen resolved at local main `bda01f1...`, remote main at
   `a4b2a733...`), and separately with a resolved revision that does match a
   captured remote head (quire-contract-ir at `ef11217...` on both sides).
   Then run `arch-lint direction` end to end against `--qsl .` and a real
   local checkout, and inspect the printed report for the resolved revision
   of every root.
8. Parse a synthetic `cargo metadata` document in which RT depends on
   `quire-exact` and on `qsl-eval`, both sourced from the QSL repository's
   git url, and run the check on the resulting edges.
9. Parse a synthetic `cargo metadata` document in which RT depends on
   `quire-semantic-value`, `qsl-semantics` and `qsl-eval`, all sourced from
   the QSL repository's git url, and run the check on the resulting edges.

## Expected Results

- Step 1: both the FB-05 and FB-11 reports are empty; the check passes.
- Step 2: both edges are reported as FB-05 violations; the FB-05 report names
  each violating edge's source and target.
- Step 3: the dev edge from CG is reported as an FB-05 violation; the stated
  exception excludes only CG's *normal* edge.
- Step 4: each cycle is reported exactly once in the FB-11 report regardless
  of which repository's edges the search started from; the report does not
  duplicate a cycle by rotation.
- Step 5: the FB-11 report is empty.
- Step 6: the FB-05 report names no edge from quire-contract-ir into QSL,
  and the FB-11 report names no cycle between QSL and quire-contract-ir,
  because neither IR manifest declares a QSL dependency (ADR-011 OBS-029);
  step 2's seeded IR → QSL edge is still reported. This output is captured
  for the PR body as real, not synthetic, evidence. Separately, an automated
  test resolves QSL's own workspace manifest through the edge-resolution
  path `direction` uses for `--qsl` and asserts that the edge set contains
  QSL's normal edge on the git-sourced `quire-contract-model`, classified as
  QSL → IR (the permitted direction); a classifier that no longer recognises
  the real IR crate drops that edge and fails the test.
- Step 7: the mismatched-revision call fails distinctly (`Code::Stale`), and
  the error names both the stale local revision and the remote's current
  head; the matched-revision call passes; the end-to-end run's report prints
  the resolved revision for every root, including `--qsl`'s, regardless of
  whether the run passes, fails, or is skipping the comparison via
  `--offline`.
- Step 8: the only edge is RT → QSL via `qsl-eval`, and the FB-05 report
  names exactly that edge; the `quire-exact` dependency contributes no edge.
- Step 9: the edges are RT → QSL via `qsl-eval` and via `qsl-semantics`, and
  the FB-05 report names exactly those two; the `quire-semantic-value`
  dependency contributes no edge.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/graph.rs`, `tools/arch-lint/metadata.rs`
- Automation: Automated Rust unit tests, including step 6's resolution of QSL's own manifest, plus one manual real-data `direction` run (step 6)

## Dependencies

**Upstream:** [FR-059](../functional/FR-059-check-backend-dependency-direction.md).
**Downstream:** none.
