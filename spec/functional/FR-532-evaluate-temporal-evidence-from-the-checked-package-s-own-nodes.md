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

### Requirements

- Every route SHALL build the evaluator's clause facts from the package's bytes.
- Every route SHALL name an atom by the index of "Atom index" below.
- The bare route SHALL evaluate every formula entry the package carries,
  window aggregate atoms included, and SHALL give for one package and one request
  the result and charges of the in-process routes.

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
- `formula`: the `formula_graph()` entries, their node ids and operand edges;
- `name`: the declared clause name of the occurrence, read from its
  `temporal_clause_names` row (the evaluator uses it where the trace and the
  progress ledger admit a clause by name); an occurrence with no such row refuses
  `clause-unreadable` with reason `clause-name`;
- `captures`: the capture names in authored order from the captures argument's
  binding names, each with the declared `ValueType` projected, as FR-390 projects
  a binder type, from the typed result type of the capture's initializer term; a
  capture whose type has no kernel representation refuses `clause-unreadable` with
  reason `capture-type`.

`from_emission` and `from_read` build the same value from the same package and keep
the cross-check of FR-515 that the package's `formula_graph()` equals the source
clause it is paired with. They take no activation, profile or clock from the source
clause: on those two routes the source clause serves the cross-check and nothing
else.

### Atom index

An atom is a distinct `holds` node or window aggregate node of the formula. Two uses
with one node id (`formula_graph()` entry `node`; equal terms share one node,
FR-337) are one atom, as the evaluator already keys its letters by node id. The atom
index is the number of the atom among the distinct atom nodes, counted from 0 in the
order of the first use of each in `formula_graph()` pre-order, over atom nodes only:
no connective, constant or temporal entry takes an index. Valuations and aggregate
samples are keyed by this index on every route (the request's valuations and
samples, the evicted records, the `valuation:<atom>@<pos>` dimension,
`instance_bound`). The in-process routes map the checked formula's own arena node of
an atom to this index, so the arena index is no spelling of an atom, because a
package carries no arena. For a formula without a shared atom node the index of the
`k`-th atom use is `k`, the order FR-515 gives `predicate_leaves()` for `holds`
atoms.

A shared atom has one valuation at a position and is looked up once per visited
position: the evaluator activates each distinct formula node once per position
(FR-523: a shared node counts once in `formula_nodes`), so a repeated operand adds
no `valuations`, `visited` or `evaluation_steps` unit beyond the atom's one lookup
(FR-501-AC-12's lookup charge).

### Truth from the graph

The evaluator walks the `formula_graph()` entries with the operators, intervals and
bounds the entries carry (the `quire.op.temporal.*` identities of FR-337, the
interval forms of FR-515) and applies the finite-window semantics of FR-504 to them.
For one formula the verdict, settlement, support, per-visit charges and non-values
are the ones the checked-formula walk gives. A cycle or out-of-range operand position, an operator
without a finite upper bound, a timed interval under a bounded profile and a
`predicate_leaves()` count that differs from the `holds` count are
`clause-unreadable` as FR-516 states.

### Window aggregates

A clause with a window aggregate atom is written as a `quire.op.temporal.aggregate`
entry whose one `aggregate` argument binds the kind, the `quantile` level `q`, the
`fraction` predicate, the value, the optional filter, the window (`window_kind`,
`lower`, `upper`), the normalized `min_count`, the comparison and the threshold
(FR-337-AC-15). QSpec FR-414 defines the aggregate term and QSpec FR-370 "Window
aggregate atom" defines these wire bindings (QSL-1644).

Every route SHALL evaluate such an atom as FR-516 "Window aggregate atoms" states,
from the entry's members read from the bytes and the per-position samples of the
request. `formula_graph()` SHALL read the entry (FR-515: `aggregate` member) and the
truth plan SHALL walk it as the atom it is.

This does not run on a package route today, and no package route has ever raised a cause
for it: QSL emits the node (FR-337-AC-15), QSpec FR-370 catalogues it and the Contract IR
reader QSL locks admits it. The evaluator plans the node as an atom on every route and
evaluates it from the observation owner's per-position samples, but the package subject
view does not yet read the node's argument and the evidence request carries no
`aggregate_samples`, so no package carrying it is evaluated. The criteria for aggregate
atoms (FR-532-AC-11, FR-516-AC-47 to AC-50) are gated on those two.

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
| FR-532-AC-4 | A clause with the four distinct `holds` atoms of FR-515-AC-22 in formula order gives the instance-bound set `{0, 3}` on `from_package` and on the other routes, as atom indices 0 to 3 over the `holds` nodes only; swapping the first two atoms gives `{1, 3}`; for a clause whose checked formula has an arena order that differs from first-use order, valuations are keyed by first-use atom index on all three routes, and a request keyed by the arena index reads a different atom (an absent valuation, a `Missing` non-value with dimension `valuation:<atom>@<pos>`). | Test (TC-1332) |
| FR-532-AC-10 | A clause `always[0,3] (holds(p) and holds(p))` with one shared `holds` node used twice has one atom, index 0: a request with one valuation of atom 0 per position returns a verdict and no `Missing` non-value; a valuation of atom 1 is not read; `usage` counts one `valuations` lookup per visited position, equal to that of `always[0,3] holds(p)`; `instance_bound` holds 0 once when `p` reads a capture; the three routes agree on all of it. | Test (TC-1332) |
| FR-532-AC-11 | Gated on the aggregate-node dependencies of "Window aggregates". Over the event-position false-extension profile, the clause `on origin always[0,3] (sum(v) over past[0,2] <= 15)` with `Value` samples 3, 4, 5, 6 at positions 0 to 3 (window sums 3, 7, 12, 15), progress `Complete { through: 3 }` and a closed decision scope, `temporal_subject` reads the aggregate entry, `formula_graph()` returns its kind, `value`, `predicate` (none for a `sum`), `filter` (none when unwritten), `window_kind`, `lower`, `upper`, `min_count`, comparison and threshold, and the call over `from_package`, `from_emission` and `from_read` returns the same result and charges (FR-516-AC-50), with `truth = True`; with the threshold `<= 14` position 3 reads 15 and the call returns `truth = False` with settlement `DecisiveCounterexample`, on the three routes. | Test (TC-1332) |
| FR-532-AC-12 | An occurrence with no `temporal_clause_names` row refuses `clause-unreadable` with reason `clause-name`, and a capture whose initializer type has no kernel representation refuses `clause-unreadable` with reason `capture-type`, each with an empty charge log on `from_package`; with the rows present the capture names, in authored order, and their value types equal those of the in-process clause on all three routes. | Test (TC-1332) |
| FR-532-AC-5 | A `LedgerBinding` and a `result_identity` produced over `from_package` hold `package.name()` as their package component and no source authority or identity label, and over the same package `from_emission` and `from_read` produce byte-equal values; a ledger produced under the labels is progress for a different binding and is not read. | Test (TC-1332) |
| FR-532-AC-6 | `from_package` over a package with two `temporal_clause_names` rows that pair to one `{ declaration, occurrence }` returns `PackageMismatch::AmbiguousName` and charges nothing; no package makes it return `ClauseRow`, `ProfileSelections` or `Bindings`, which an inspection of the constructor confirms. | Test (TC-1332) |
| FR-532-AC-7 | A clause whose `formula_graph()` carries an operator without a finite upper bound, a timed interval under a bounded profile, an operand cycle, an out-of-range operand position or a `predicate_leaves()` count that differs from the `holds` count gives `clause-unreadable` with an empty charge log over `from_package`; the same shape with a finite bound, a closed interval, acyclic in-range operands and equal counts gives a result; a tick sum that overflows `u64` gives the `history_span` `Incomplete`. | Test (TC-1332) |
| FR-532-AC-8 | Two clauses of different nodes in one package each pair with their own clock row over `from_package`; a `binding` asserting the other clause's row refuses `binding-mismatch` naming the dimension FR-522 names; a node with several occurrences refuses `occurrence-required` without a selector and `occurrence-unknown` with an unknown one; a package with the rows of FR-515-AC-20 (producer ordinals 7 then 3) pairs by key. | Test (TC-1332) |
| FR-532-AC-9 | A search of `qsl-eval/src/temporal/evidence` for the symbols `CheckedTemporalClause::activation`, `::profile`, `::clock` and `SourceIdentity` finds that the evaluator and the three constructors reach the activation, the profile and the clock only through `ClauseFacts`: an inspection finds no read of `CheckedTemporalClause::activation`, `profile` or `clock` outside the cross-check, and `qsl_foundation::SourceIdentity` is no member of `LedgerBinding`. | Inspection (TC-1332) |

## Dependencies

- **Upstream:** FR-515 (views and rows), FR-516, FR-522 to FR-524, FR-526, FR-043,
  FR-504, FR-531; QSpec's finite-window profile identities.
- **Delivery:** the code change retires `CheckedTemporalClause` as an input of the
  call and the arena index as an atom spelling, and makes `ClauseFormula::of_graph`
  dedupe shared nodes by the entry's `node` (the per-use list it reads today would
  count a shared node once per use in `formula_nodes`, against FR-523-AC-6 and
  FR-532-AC-10). The aggregate criteria are gated on QSL-1643 and QSL-1644 (the QSpec catalog
  entry for the aggregate node and its admission by Contract IR).
- **Downstream:** quire-protocol FR-046.
