---
id: FR-265
title: "Derive the separating witness of a state clause from its evaluation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-041
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
---
# FR-265: Derive the separating witness of a state clause from its evaluation

## Description

When S6a evaluates a state clause (FR-107) to a Boolean, QSL SHALL derive
the clause's settlement basis and, when that basis is decisive, its one
separating witness record (QSpec FR-351), as ADR-031 SW-1 to SW-6 and SW-9
decide. The derivation is one function. The FR-109 clause run (FR-266) and
the FR-122 replay (FR-268) both call it, so producer and replay derive the
record the same way.

## Inputs

- The checked state clause and the clause's claim root.
- The S6a evaluation of the clause: its outcome and, beside it, one stop
  report for each quantifier occurrence that stopped before its last
  element. A stop report holds the quantifier's occurrence key (O-07), the
  index of the element it stopped at, the value bound to its binder there
  and that element's source binding (QSpec FR-041-AC-4).
- The admitted observation (FR-106) the evaluation read.

## Outputs

- A settlement basis from QSpec FR-243: `decisive-counterexample`,
  `decisive-witness` or `closed-scope` for a completed Boolean.
- When the basis is decisive, one `SeparatingWitnessRecord` holding:
  - `quantifier`: the decisive quantifier's occurrence key;
  - `deciding_element`: the kernel `Value` bound to its binder at the stop;
  - `index`: the element's zero-based position in the domain's own order;
  - `value_path`: the path from the observation root to the domain
    collection, in QSpec's value-path segment vocabulary;
  - `trace_position`: absent.

## Behavior

- The S6a evaluator SHALL report each early stop of a `forall` (at the first
  element whose body is `false`) and of an `exists` (at the first element
  whose body is `true`) to its caller beside the outcome, and SHALL NOT put a
  stop report inside a `Value`.
- The derivation SHALL walk the decision path from the claim root (ADR-031
  SW-2): into the inner node of a group, the body of a `let`, the selected
  branch of an `if`, the operand of `not` with the polarity flipped, the
  operand of `and` or `or` on which evaluation stopped (the first `false`
  operand of a `false` `and`, the first `true` operand of a `true` `or`,
  otherwise the right operand), and for `implies` the antecedent with the
  polarity flipped when it is `false`, otherwise the consequent.
- The walk SHALL end at the first node of any other kind.
- The walk SHALL visit each node of the claim at most once.
- If the walk ends at a quantifier occurrence that has a stop report, then
  the derivation SHALL settle the decisive basis for the clause's truth:
  `decisive-counterexample` for `false`, `decisive-witness` for `true`.
- If the walk ends at a quantifier occurrence that has a stop report, then
  the derivation SHALL build the record from that occurrence's stop report.
- If the walk ends at a node that is not a stopped quantifier occurrence,
  then the derivation SHALL settle basis `closed-scope` with no record. A
  quantifier that visited every element (a `true` `forall`, a `false`
  `exists`, or an empty domain) has no stop report.
- The record SHALL name the decisive occurrence the walk reaches, the
  root-most one on the path. A quantifier decided inside its body is not
  recorded.
- The value path SHALL be the QSpec FR-207 runtime value path of the
  deciding element's location, with QSpec FR-207's subjects, steps and wire
  spelling (FR-207-AC-9 to AC-11).
- If the domain is computed by `filter` or `map`, or built inside the
  claim, then the index and value path SHALL be the ones QSpec FR-207 gives
  that element (QSpec FR-207-AC-9 to AC-11), and the deciding element SHALL
  stay the value the body was evaluated on, which for `map` is the mapped
  value.
- An evaluation that completes no Boolean (`refused`, `undefined`,
  `incomplete`, or a family result) SHALL yield no basis from this
  derivation and no record; FR-266 gives such a result basis `unavailable`.
- The derivation SHALL read nothing beyond the clause, the stop reports and
  the admitted observation. It SHALL evaluate no element after a stop.

## Criteria fixture

The criteria use FR-108's `Config` domain package and a unit `witness`
whose invariants on `Config::ConfigVersion` at `current` range over the
current observation's `ConfigVersion` population (the population query
FR-104 admits), in population document order (FR-106):

- `AllBelow`: `forall(c in <population>: c.versionNumber < 500)`;
- `SomeAtLeast`: `exists(c in <population>: c.versionNumber >= 500)`;
- `NotAllBelow`: `not AllBelow`'s body;
- `GuardedAll`: `present(self.parent) implies` `AllBelow`'s body;
- `EqualsAll`: `AllBelow`'s body `= present(self.parent)`;
- `LetAll`: `let k = 500 in forall(c in <population>: c.versionNumber < k)`;
- `OrAll`: `AllBelow`'s body `or self.versionNumber = 0`;
- `NestedAll`: `forall(c in <population>: forall(d in <population>:
  d.versionNumber <= c.versionNumber + 100))`;
- `FilteredAll`: `AllBelow`'s body over the `filter` of the population to
  the objects with a `parent`;
- `MappedAll`: `forall(n in map(c in <population>: c.versionNumber): n <
  500)`;
- `BuiltAll`: `forall(n in [self.versionNumber, 600]: n < 500)`;
- `NoneSelected`: `forall` over the `filter` of the population to the
  objects with `versionNumber > 900`, body `c.versionNumber < 0`.

Snapshots: `low` holds `root` (0) and `child` (1, parent `root`); `high`
holds `root` (0), `mid` (600, parent `root`) and `leaf` (700, parent
`mid`); `tail` is `high` with a fourth object `orphan` (800) whose
`parent` is absent. `self` is `child` in `low` and `mid` in `high` and
`tail`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-265-AC-1 | `AllBelow` over `high` derives basis `decisive-counterexample` and a record whose `quantifier` is the `forall`'s occurrence key, `deciding_element` the reference to `mid`, `index` 1, `value_path` the current observation's `ConfigVersion` population root and no trace position. Over `tail`, where `orphan` (800) comes after `mid`, it derives the equal record and basis, and the evaluation's work charges equal those over `high`, so no element after the stop was evaluated. Over `low` it derives `closed-scope` and no record. | Test (TC-740) |
| FR-265-AC-2 | `SomeAtLeast` over `high` and `NotAllBelow` over `high` each derive `decisive-witness` with a record naming `mid` at index 1 and the `exists` or the `forall` occurrence respectively. `SomeAtLeast` over `low` derives `closed-scope`, no record. `EqualsAll` over `high` derives `closed-scope` and no record, though its `forall` stopped. | Test (TC-740) |
| FR-265-AC-3 | Decision path: `GuardedAll` over `high` (parent present) derives `AllBelow`'s record; with `self` `root` (no parent) it derives `closed-scope`, no record. `LetAll` over `high` derives the record for its own `forall`. `OrAll` over `high` with `self` `mid` (`versionNumber` 600) is `false`, its walk ends at the right operand `self.versionNumber = 0`, and it derives `closed-scope`, no record. | Test (TC-740) |
| FR-265-AC-4 | `NestedAll` over `high` is `false` (the outer element `root` fails at inner element `mid`); the record names the outer `forall`'s occurrence, `deciding_element` the reference to `root`, `index` 0, and no inner occurrence. | Test (TC-740) |
| FR-265-AC-5 | Computed and built domains: `FilteredAll` over `high` names `mid` with `index` 1 and the population root as its path (the source position, not the position 0 it holds in the filtered collection). `MappedAll` over `high` holds `deciding_element` 600, `index` 1 and the population root. `BuiltAll` over `low` holds `deciding_element` 600, `index` 1 and a value path rooted at a `built` subject naming the list expression's occurrence key, ending in an index step for position 1 (QSpec FR-207-AC-9). | Test (TC-740) |
| FR-265-AC-6 | No Boolean, no record: `AllBelow` over `high` with an evaluation budget of zero completes `incomplete`, and the derivation yields no basis and no record. `NoneSelected` over `low`, whose filtered domain is empty, derives `closed-scope`, no record. | Test (TC-740) |

## Dependencies

- [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md) SW-1 to
  SW-6 and SW-9.
- [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (the S6a clause
  evaluation that reports the stops), [FR-106](FR-106-admit-snapshots-and-invocations.md)
  (the admitted observation and population order),
  [FR-108](FR-108-run-the-configversion-spine-corpus.md) (the `Config`
  domain package).
- QSpec FR-041 (decisive occurrence and source binding), FR-243 (basis
  labels), QSpec FR-351 (the record), FR-207 AC-9 to AC-11 (the runtime value
  path) and QSpec FR-351-AC-7 (the deciding-quantifier component).

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
- QSpec half (Linear STD-144): QSpec FR-207 AC-9 to AC-11 (value-path
  segment vocabulary and the building-expression root) and QSpec FR-351-AC-7 (the
  deciding-quantifier component).
