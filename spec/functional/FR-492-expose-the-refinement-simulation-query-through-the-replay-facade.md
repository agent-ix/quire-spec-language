---
id: FR-492
title: "Expose the refinement simulation query through the replay facade"
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
# FR-492: Expose the refinement simulation query through the replay facade

## Description

A backend that checks the per-step simulation obligations of a refinement
needs FR-315's `Initial` and `Step { row }` queries. The refinement node's
`steps` array holds the rows in declaration order, so a query's `row` is the
row's position among the rows as written.
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
- `CheckedClosureHandle::refinement_node_key(name) -> Result<NodeKey,
  RefinementKeyRefusal>`: the node key of the refinement declared as `name`,
  resolved from the retained closure (no digest; it does not depend on how
  documents are keyed). `RefinementKeyRefusal` is `UnknownName`,
  `NotARefinement` (declared as a state clause) or `NotEmitted`.
- `RefinementUniverse::for_population(package, population, keys)` and
  `RefinementUniverse::new(population, keys)`: a universe keyed by the declared
  names. `RefinementUniverse` and `RefinementQuerySubject` are
  `#[non_exhaustive]`; `RefinementQuerySubject::new(..)` builds a subject with
  default normalization and observation limits and no proof bounds.
- `RefinementQuerySubject`: one subject (the concrete or the abstract model):
  the domain package and snapshot provisions, the initial `DocumentRef`s, the
  `RefinementUniverse`s (a population's `DeclarationKey` and its object keys),
  the per-expansion candidate bound, the normalization, observation and meter
  limits and the proof bounds. The wrapper converts it into the `qsl_eval`
  subject inside the entry.
- `RefinementQueryKind`: `Initial` or `Step { row }`, converted to the
  `qsl_eval` kind inside the entry.
- `encode_refinement_query(&RefinementQueryRequest, RefinementQueryKind)`.

## Behavior

- The query encoding shall be FR-315's, unchanged: the wrapper converts its
  inputs and calls the existing entry.
- The wrappers shall hold the checked closure and the subject members unchanged.
- The wrappers shall compute no digest, hash, revision, seal, lock or pin; a
  digest-keyed field of the wrapped request is passed through until the digest
  removal program changes the request (QSL-1282).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-492-AC-1 | Over a request built from `qsl_replay` root items alone (the closure from `acquire_checked_closure`), `CasRefinesCounter`'s `Initial` query is a `QF_LIA` script over one state copy, its `Step` query of a row is over two copies, queries of different rows differ, and a query is deterministic. | Test (TC-1275) |
| FR-492-AC-2 | **RETIRED (steps are in declaration order, so there is no position mapping).** | Test, retired |
| FR-492-AC-3 | **RETIRED (steps are in declaration order, so there is no position mapping).** | Test, retired |
| FR-492-AC-4 | The `qsl_replay` root re-exports no `qsl_eval` item: `arch-lint api-surface --qsl . --qsl-only` reports no finding. | Test (arch-lint-api-surface-qsl) |
| FR-492-AC-5 | `refinement_node_key("CasRefinesCounter")` over the retained closure returns the node key of that refinement, and a second call returns the same key. | Test (TC-1275) |
| FR-492-AC-6 | `refinement_node_key` over a name the closure does not declare returns `UnknownName`, and over a name declared as a state clause returns `NotARefinement`. | Test (TC-1275) |
| FR-492-AC-7 | `RefinementUniverse::for_population(package, population, keys)` equals the universe of the `DeclarationKey` of that population declared in that package. | Test (TC-1275) |

## Dependencies

FR-314, FR-315, FR-060 (T12), ADR-011.
