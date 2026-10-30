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

The `qsl_replay` root exports replay-owned wrapper types and functions; it
re-exports no `qsl_eval` item (FR-100-AC-8). A consumer builds the request from
root items alone:

- `RefinementQueryRequest::new(closure, refinement, concrete, abstract_)`: the
  request of a refinement query (FR-314's request of a refinement item). It
  takes the `CheckedClosureHandle` that `acquire_checked_closure` retains, the
  refinement's `NodeKey` and two subjects.
- `RefinementQuerySubject`: one subject (the concrete or the abstract model):
  the domain package and snapshot provisions, the initial `DocumentRef`s, the
  `RefinementUniverse`s (a population's `DeclarationKey` and its object keys),
  the per-expansion candidate bound, the normalization, observation and meter
  limits and the proof bounds. The wrapper converts it into the `qsl_eval`
  subject inside the entry.
- `RefinementQueryKind`: `Initial` or `Step { row }`, converted to the
  `qsl_eval` kind inside the entry.
- `encode_refinement_query(&RefinementQueryRequest, RefinementQueryKind)`.
- `refinement_step_sources(&RefinementQueryRequest) -> Result<Vec<u64>,
  SmtEncodingRefusal>`: for each position of the refinement's `steps` array,
  the position of the same row among the rows as written. The result is a
  permutation of `0..n`. A refinement with no emitted rows, or a request that
  names no refinement of its package, is a fault refusal that names which.

## Behavior

- The query encoding shall be FR-315's, unchanged: the wrapper converts its
  inputs and calls the existing entry.
- The mapping shall be the one the checker emitted with the `steps` array,
  read through the `qsl_eval` accessor `refinement_step_sources`.
- The wrappers shall hold the checked closure and the subject members unchanged.
- The wrappers shall compute no digest, hash, revision, seal, lock or pin; a
  digest-keyed field of the wrapped request is passed through until the digest
  removal program changes the request (QSL-1282).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-492-AC-1 | Over a request built from `qsl_replay` root items alone (the closure from `acquire_checked_closure`), `CasRefinesCounter`'s `Initial` query is a `QF_LIA` script over one state copy, its `Step` query of a row is over two copies, queries of different rows differ, and a query is deterministic. | Test (TC-1275) |
| FR-492-AC-2 | `refinement_step_sources` over `CasRefinesCounter` returns a permutation of `0..7` and each named operation's `steps` position maps to the source row of that operation. | Test (TC-1275) |
| FR-492-AC-3 | Writing `commitB` before `commitA` changes the mapping and not the `Step` query the `commitA` row encodes. | Test (TC-1275) |
| FR-492-AC-4 | The `qsl_replay` root re-exports no `qsl_eval` item: `arch-lint api-surface --qsl . --qsl-only` reports no finding. | Test (arch-lint-api-surface-qsl) |

## Dependencies

FR-314, FR-315, FR-060 (T12), ADR-011.
