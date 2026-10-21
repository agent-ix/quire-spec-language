---
id: FR-150
title: "Check the symmetric population annotation and refuse identity-observing forms at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
---
# FR-150: Check the symmetric population annotation and refuse identity-observing forms at S3

## Description

A population declaration may carry the `symmetric` annotation, which states
that the identities of the population's objects are interchangeable in every
clause that reads it (ADR-021 SYM-8). QSL SHALL read the annotation at
intake, carry it into the checked package, and refuse at S3 every form that
observes the identity of an annotated population's objects other than by
equality, dereference, membership and passing as a value (ADR-021 SYM-3).
The annotation does not apply any reduction; a request opts in to symmetry
separately (FR-151).

## Use case

A verification operator marks `config_history` `symmetric` because its
configs are interchangeable. If a clause in the unit folds over a set of
`ConfigVersion` references with a step outside FR-145's catalog, the operator learns it at once, at that `fold`,
rather than from an `inconclusive` verdict after a long request (US-019).

## Inputs

- An admitted domain package whose population records may carry the
  `symmetric` marker (QSpec model contract; the spelling and the marker are
  QSpec's, see References).
- A `1-draft` unit and its checked clauses: operation preconditions and
  postconditions and invariants (FR-104), temporal clause atoms (FR-123),
  `terminal when` predicates (FR-124), state constraints (FR-158), and
  refinement rows where the unit has them (ADR-020).

## Outputs

- Each population record in the checked package with `symmetric: bool`.
- An S3 refusal for each identity-observing form over an annotated
  population, or no refusal.

## Behavior

- Intake SHALL read the `symmetric` marker of each population record and
  carry it, unchanged, on the population's record in `PackageDeclarations`
  and in the checked package.
- A population is **read** by a clause when the clause's checked body has a
  subexpression whose static type is, or contains, `Reference<T>` for a
  member type `T` of the population, or a `Set`, `Bag`, `Sequence`,
  `Option` or record whose element or field type contains one.
- When a clause reads an annotated population, the checker SHALL refuse
  each of these forms that applies to a value whose static type contains a
  reference into that population:
  1. a `fold` or `reduce` over a `Set` or `Bag` whose step is outside QSpec
     FR-145's commutative and associative catalog, or another traversal that
     depends on element order. The class is decided by catalog membership: a
     step outside the catalog is refused conservatively as an order-sensitive
     traversal even when its result happens not to depend on order (a constant
     step), because FR-145 admits no other proof of order independence;
  2. a conversion of a reference to a value of another type, including to
     text;
  3. a reference literal that names a key.
  An ordering comparison over references is already ill-typed (the S3
  ordering arm) and keeps its existing refusal.
- QSpec FR-145 refuses a `fold` or `reduce` over a `Set` or `Bag` whose step is
  outside its catalog, `ill_typed`/`operator-ineligible`, in every unit.
- When the source of such a fold or reduce contains a reference into an
  annotated population, the checker SHALL report it as form 1
  (`identity-observing-form`); otherwise the checker SHALL report FR-145's
  `operator-ineligible`. The split is per fold, from its source type, since a
  unit may read an annotated and an unannotated population.
- The checker SHALL check a `fold` or `reduce` whose step is in the catalog as
  FR-145 does, annotated population or not; it is not form 1.
- The identity-observing cause names the population the operator should
  unannotate or restructure, which is why it takes precedence.
- Each refusal SHALL be `ill_typed`/`identity-observing-form` at the span of
  the refused node and SHALL name the node kind and the population's
  declaration identity text. The code and cause are QSpec's (References).
- The checker SHALL report every refused node of a unit, in source order,
  and SHALL produce no checked package for a unit with one.
- A clause that observes an annotated population only by equality, field
  read, `deref`, `reaches`, membership, quantification and passing as a
  value SHALL check as it does without the annotation.
- A form listed above over a population that is not annotated SHALL be
  checked by the rules that apply without the annotation.
- The annotation SHALL enter the checked package's identity, so a package
  that differs only by the annotation has a different package identity
  (ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-150-AC-1 | ADR-021 §7.1's ConfigVersion unit with `config_history` annotated checks: its post clause reads `self.versionNumber`, and the invariants `ParentOrder` and `NoCycle` read `parent` by `deref` and equality. The checked package's `config_history` record has `symmetric: true`, and its package identity differs from the same unit's package without the annotation. | Test (TC-565) |
| FR-150-AC-2 | Adding field `peers: Set<Reference<ConfigVersion>>` and a post clause that chooses the new parent with `fold` over `self.peers` refuses `ill_typed`/`identity-observing-form` at the `fold` node's span, naming `fold` and `config_history`; no checked package is produced. The same unit without the annotation refuses `ill_typed`/`operator-ineligible` at the `fold` (FR-145: a step that chooses an element is not commutative and associative). A `fold<Tally>(acc, p in self.peers: acc + 1, identity: 0)` counting the peers, with `type Tally = Integer;`, checks in both units (`Integer` is a keyword and cannot stand as the fold's type name). | Test (TC-565) |
| FR-150-AC-3 | Over the annotated unit, an invariant comparing `self` to a reference literal naming key `a` refuses at the literal; a temporal atom converting `c` to text refuses at the conversion; a `terminal when` predicate with a `reduce` over a `Bag` of `ConfigVersion` references refuses at the `reduce`; a state constraint with a `fold` over such a set refuses at the `fold`, each with a step outside FR-145's catalog (it chooses an element). A unit with two refused forms reports both, in source order. | Test (TC-566) |
| FR-150-AC-4 | Over the annotated unit, an invariant `forall p in config_history: p.parent = self implies p != self` and an atom `holds(c.parent = a.parent)` check, and `fold<Tally>(acc, x in s: acc + 1, identity: 0)` over a `Set<Int[0, 3]>` field `s`, with `type Tally = Integer;`, checks: neither reads the population by an identity-observing form. | Test (TC-566) |

## Dependencies

- ADR-021 SYM-3 and SYM-8; ADR-013 O-09 (package identity).
- [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md) (population
  records), [FR-104](FR-104-check-state-clauses.md) (state clause checking),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (temporal clauses), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (`terminal when`), [FR-158](FR-158-cut-the-search-with-a-state-constraint.md)
  (state constraints).
- QSpec owns the annotation's spelling, its place in the model contract, the
  identity-observing forms as normative semantics and the diagnostic code
  and cause (ADR-021 QS-3).

## Remaining work

- QSpec FR-381 still states that the same forms over an unannotated population
  are admitted, which FR-145 contradicts; QSpec Linear STD-244 settles it and
  the precedence above stands until then.

## References

- QSpec half: QSpec FR-381 (Linear STD-134; ADR-021 §9 QS-3). Owning ticket: Linear
  QSL-368.
