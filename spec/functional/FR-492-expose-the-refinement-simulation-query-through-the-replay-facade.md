---
id: FR-492
title: "Expose the refinement simulation query and its step-row mapping through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-492: Expose the refinement simulation query and its step-row mapping through the replay facade

## Description

A backend that checks the per-step simulation obligations of a refinement,
for example CG's check that its source rows are a permutation of the query
rows, needs FR-315's `Initial` and `Step { row }` queries and the mapping from
each position of the refinement node's `steps` array to the row as written.
`encode_refinement_query` and its request and kind types are `qsl-eval`'s;
ADR-011 FB-05 and T-12 rule (a) (FR-060 T12-A) forbid CG to name `qsl_eval` or
`qsl_replay::spine`. This requirement exposes them at the `qsl_replay` root.

## Outputs

At the root: `encode_refinement_query`, `RefinementCertificateRequest`,
`RefinementQueryKind` (`Initial`, `Step { row }`), the subject and package
types the request is built from (`ModelSubject`, `ModelSimulationRequest`,
`PopulationUniverse`, `ExpansionLimits`, `ObservationLimits`,
`CheckedPackage`) and `refinement_step_sources`.

`refinement_step_sources(request) -> Result<Vec<u64>, SmtEncodingRefusal>`
returns, for each position of the refinement's `steps` array, the position of
the same row among the refinement's rows as written. The result is a
permutation of `0..n`. A request that names no refinement of its package is a
fault refusal.

## Behavior

- The query encoding SHALL be FR-315's, unchanged; the root exports the
  existing entry and SHALL NOT restate it.
- The mapping SHALL be the one the checker emitted with the `steps` array and
  SHALL NOT be recomputed.
- The entry and mapping SHALL record and compare no digest, hash, revision,
  seal, lock or pin of their own.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-492-AC-1 | Through the root, `CasRefinesCounter`'s `Initial` query is a `QF_LIA` script over one state copy, its `Step` query of a row is over two copies, queries of different rows differ, and a query is deterministic. | Test (TC-1275) |
| FR-492-AC-2 | `refinement_step_sources` over `CasRefinesCounter` returns a permutation of `0..7` equal to the order the checker emitted, and each named operation's `steps` position maps to the source row of that operation. | Test (TC-1275) |
| FR-492-AC-3 | Writing `commitB` before `commitA` changes the mapping and not the `Step` query the `commitA` row encodes. | Test (TC-1275) |

## Dependencies

FR-314, FR-315, FR-060 (T12), ADR-011.
