---
id: FR-532
title: "Evaluate temporal evidence from the checked package's own nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-504
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/quire-protocol/FR-046
    type: references
---
# FR-532: Evaluate temporal evidence from the checked package's own nodes

## Description

`EvidencePackage::evaluate_temporal_evidence` (FR-516) reads three things from the
source side of the package it was built over: the checked clause's activation,
profile and clock name (`CheckedTemporalClause`), the checked formula the truth
evaluator walks, and the source identity labels of a progress binding. The
package's own nodes and rows carry the first, and the shape of the second, and
FR-515 already derives the formula's operations, intervals and operand edges and the
instance-bound atoms from them. This requirement makes the bytes the only source of
all three, adds `EvidencePackage::from_package`, and states that the three routes
(from an `Emission`, from a read-back package, from a bare package) return the same
`EvidenceResult`.

## Semantic authority and boundary

FR-043 and FR-504 own truth, FR-044 and FR-524 activation and captures, FR-501 and
FR-502 the profile and the progress ledger, FR-516 the request, the result and the
refusal causes, FR-522 the binding. This requirement changes where the evaluator
reads its clause facts and states one atom spelling; it changes no truth rule and
no cause.

## Inputs

`EvidencePackage::from_package(package: &'p BarePackage<'p>) -> Result<Self,
PackageMismatch>` (FR-531). It takes no supply. The call is
`evaluate_temporal_evidence(&self, clause, request, meter)`, unchanged.

## Outputs

`Result<EvidencePackage, PackageMismatch>`, and `EvidenceResult` from the call.
`from_package` returns `PackageMismatch::AmbiguousName` when two
`temporal_clause_names` rows pair to one `{ declaration, occurrence }`, as
`from_read` does, and no other variant: `ClauseRow`, `ProfileSelections` and
`Bindings` compare an `Emission` with a package, and a bare package has none.

## Behavior

### One clause value, from the bytes

On every route the evaluator receives one `ClauseFacts` value built from the
subject view and the held rows:

- `activation`: `Origin` or `Each` as `activation()` returns it;
- `profile`: the evaluator profile (`EventPositionFalseExtension`,
  `FixedSampleFalseExtension`, `TimestampedEventFiniteWindow`) that the `profile`
  row's definition identity names in the closed table of QSpec's finite-window
  profile identities, or the infinite-trace profile by its own identity;
- `clock`: the `TemporalBinding` row's `clock_name` and clock configuration, keyed
  `{ declaration, role, ordinal }`;
- `formula`: the `formula_graph()` entries and operand edges.

`from_emission` and `from_read` build the same value from the same package and keep
the cross-check of FR-515 that the package's `formula_graph()` equals the source
clause it is paired with. They take no activation, profile or clock from the source
clause: on those two routes the source clause serves the cross-check and nothing
else.

### Atom index

An atom is named by the position of its `holds` entry in `formula_graph()`
pre-order, counted over every entry of the list, and valuations are keyed by that
position on every route (the request's valuations, the `valuation:<atom>@<pos>`
dimension, `instance_bound`). Where the checked formula's own arena index of an
atom differs from that position, the position is the index on the in-process routes
too: the arena index is no spelling of an atom, because a package carries no arena.
FR-515's rule that the `k`-th `predicate_leaves()` entry is the `k`-th `holds` node
orders the `holds` entries within this numbering.

### Truth from the graph

The evaluator walks the `formula_graph()` entries with the operators, intervals and
bounds the entries carry (the `quire.op.temporal.*` identities of FR-337, the
interval forms of FR-515) and applies the finite-window semantics of FR-504 to them.
For one formula the verdict, settlement, support, per-visit charges and non-values
are the ones the checked-formula walk gives. A shape the graph cannot carry (a
window aggregate, which the package route has never read) is `clause-unreadable`, as
it is on every route today. A cycle or out-of-range operand position, an operator
without a finite upper bound, a timed interval under a bounded profile and a
`predicate_leaves()` count that differs from the `holds` count are
`clause-unreadable` as FR-516 states.

### The package identity of a progress binding and a result

The package component of a `LedgerBinding` (FR-502) and of `result_identity`
(FR-043-AC-13) is `package.name()` on every route. The source identity labels
(authority and identity) are not members. A ledger produced under the labels binds
a different package component, so it is progress for a different binding (FR-502)
and is not read; no compatibility reading exists.

### Equivalence

For one package and one request, the three routes return the same `Ok` result or
the same refusal, field for field and in the canonical byte encoding of
FR-043-AC-13, with equal charge logs, `usage`, ledgers and `result_identity`. The
equivalence is between routes over one package. It is no claim about a package
whose bytes differ from a source: `from_emission` and `from_read` refuse such a
package `clause-unreadable` at the cross-check (FR-515-AC-24), and `from_package`,
which has no source to differ from, evaluates the package as admitted.

### Counters and accounting

All thirteen counters (FR-523) are charged at the same points with the same units on
the three routes. Construction charges nothing.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-532-AC-1 | For each of **R**'s four temporal clauses over its three packages (FR-517), the `EvidenceResult` of `evaluate_temporal_evidence` over `EvidencePackage::from_package` equals, field for field and in its canonical byte encoding, the result over `from_emission` and over `from_read`, with the charge log, `usage`, ledgers and `result_identity` included, for one request per truth outcome (`True`, `False`, `Pending`), per activation state and per `Incomplete` counter of FR-523. | Test (TC-1332) |
| FR-532-AC-2 | Replacing one clock row, one `CheckedNodeId`, one instance-bound atom, one operation, one interval or one operand edge in a package built from **R**'s and admitted by the reader (each in its own run) changes the `EvidenceResult` of `from_package`, and the unmutated package gives the equal results of AC-1; over the mutated package the in-process routes refuse `clause-unreadable` (FR-515-AC-24) where the formula or an operation differs and `from_package` does not. | Test (TC-1332) |
| FR-532-AC-3 | For a unit the test authors over `examples/config-version` (capture, `on each` activation, guard) with four `holds` atoms, one clause per operator of FR-337's temporal list (`always`, `eventually`, `once`, `historically`, `until`, `release`, `since`, `triggered`, `not`, `and`, `or`, `implies`), the verdict, settlement, support and per-counter usage over `from_package` equal those over the other two routes at every position of a trace that distinguishes the operator. | Test (TC-1332) |
| FR-532-AC-4 | A clause with the atoms of FR-515-AC-22 in formula order gives the instance-bound set `{0, 3}` on `from_package` and on the other routes, as positions in `formula_graph()` pre-order; for a clause whose checked formula has an arena order that differs from pre-order, valuations are keyed by pre-order position on all three routes, and a request keyed by the arena index reads a different atom (an absent valuation, a `Missing` non-value with dimension `valuation:<position>@<pos>`). | Test (TC-1332) |
| FR-532-AC-5 | A `LedgerBinding` and a `result_identity` produced over `from_package` hold `package.name()` as their package component and no source authority or identity label, and over the same package `from_emission` and `from_read` produce byte-equal values; a ledger produced under the labels is progress for a different binding and is not read. | Test (TC-1332) |
| FR-532-AC-6 | `from_package` over a package with two `temporal_clause_names` rows that pair to one `{ declaration, occurrence }` returns `PackageMismatch::AmbiguousName` and charges nothing; no package makes it return `ClauseRow`, `ProfileSelections` or `Bindings`, which an inspection of the constructor confirms. | Test (TC-1332) |
| FR-532-AC-7 | A clause whose `formula_graph()` carries an operator without a finite upper bound, a timed interval under a bounded profile, an operand cycle, an out-of-range operand position, a window aggregate or a `predicate_leaves()` count that differs from the `holds` count gives `clause-unreadable` with an empty charge log over `from_package`; the same shape with a finite bound, a closed interval, acyclic in-range operands and equal counts gives a result; a tick sum that overflows `u64` gives the `history_span` `Incomplete`. | Test (TC-1332) |
| FR-532-AC-8 | Two clauses of different nodes in one package each pair with their own clock row over `from_package`; a `binding` asserting the other clause's row refuses `binding-mismatch` naming the dimension FR-522 names; a node with several occurrences refuses `occurrence-required` without a selector and `occurrence-unknown` with an unknown one; a package with the rows of FR-515-AC-20 (producer ordinals 7 then 3) pairs by key. | Test (TC-1332) |
| FR-532-AC-9 | The evaluator and the three constructors reach the activation, the profile and the clock only through `ClauseFacts`: an inspection finds no read of `CheckedTemporalClause::activation`, `profile` or `clock` outside the cross-check, and `qsl_foundation::SourceIdentity` is no member of `LedgerBinding`. | Inspection (TC-1332) |

## Dependencies

- **Upstream:** FR-515 (views and rows), FR-516, FR-522 to FR-524, FR-526, FR-043,
  FR-504, FR-531; QSpec's finite-window profile identities.
- **Delivery:** the code change retires `CheckedTemporalClause` as an input of the
  call and the arena index as an atom spelling.
- **Downstream:** quire-protocol FR-046.
