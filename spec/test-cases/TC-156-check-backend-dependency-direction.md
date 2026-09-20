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
reproduces ADR-011 OBS-029's real, currently observed violation when run
against quire-contract-ir's real current-head `cargo metadata` output, and
that a stale `--ir`/`--rt`/`--cg` clone is rejected before the edge graph is
even built. Scope: FR-059-AC-1 through FR-059-AC-7.

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
7. Call the freshness comparison directly with a resolved revision that does
   not match a captured remote `main` head (the reviewer's real repro:
   quire-contract-codegen resolved at local main `bda01f1...`, remote main at
   `a4b2a733...`), and separately with a resolved revision that does match a
   captured remote head (quire-contract-ir at `ef11217...` on both sides).
   Then run `arch-lint direction` end to end against `--qsl .` and a real
   local checkout, and inspect the printed report for the resolved revision
   of every root.

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
- Step 6: the FB-05 report names the real edge from quire-contract-ir into
  QSL that ADR-011 OBS-029 already documents, and the FB-11 report names the
  corresponding real cycle(s); this output is captured for the PR body as
  real, not synthetic, evidence.
- Step 7: the mismatched-revision call fails distinctly (`Code::Stale`), and
  the error names both the stale local revision and the remote's current
  head; the matched-revision call passes; the end-to-end run's report prints
  the resolved revision for every root, including `--qsl`'s, regardless of
  whether the run passes, fails, or is skipping the comparison via
  `--offline`.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/graph.rs`, `tools/arch-lint/metadata.rs`
- Automation: Automated Rust unit tests plus one manual real-data run (step 6)

## Dependencies

**Upstream:** [FR-059](../functional/FR-059-check-backend-dependency-direction.md).
**Downstream:** none.
