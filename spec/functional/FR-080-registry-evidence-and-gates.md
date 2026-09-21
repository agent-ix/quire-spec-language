---
id: FR-080
title: "Provide registry evidence for ADR-012 §5.3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
---
# FR-080: Provide registry evidence for ADR-012 §5.3

## Description

QSL SHALL provide the registry evidence ADR-012 §5.3 requires for the
open seam S7 (the capability-kind enum, matched at the registry
advertisement boundary): a property-based permutation-equality test, a
`cargo-deny` ban on `inventory`, `linkme` and `ctor`, a lint gate finding no
`static`, `OnceLock` or `thread_local!` in the registry module, one unit
test per ADR-012 §5.2 row, and the S7 seam probe covering the registry arm.

## Inputs

- The registry implementation (FR-075) and its module boundary.
- ADR-012 §5.2's five explicit registry failure rows.
- ADR-012 §5.1's S7 row: the capability-kind enum and the registry
  advertisement-check seam.

## Outputs

- A property test that fails when a registry's candidate set depends on
  registration order.
- A `cargo-deny` configuration entry that fails the gate if `inventory`,
  `linkme` or `ctor` is a dependency of the registry's crate.
- A lint gate (for example a `clippy`/`ast-grep` rule run in CI) that fails
  when the registry module contains a `static`, `OnceLock` or
  `thread_local!` item.
- One unit test per ADR-012 §5.2 row, five in total.
- An `xtask seam-probe` extension covering the registry's capability-kind
  match arm (S7), consistent with the seam-probe mechanism FR-063 builds
  for S1-S4.

## Behavior

### The permutation property test is generative, not example-based

The property test SHALL construct registries from randomly permuted orders
of the same `BackendDescriptor` set (not a single fixed pair of orderings)
and SHALL assert, for each permutation, that the resulting registry is
equal to a canonical baseline registry and that computing candidate sets
against it yields identical candidate sets, in identical order, for every
item in a fixture item set. The test SHALL fail when a registry
implementation's candidate set for any item depends on insertion order, for
example an implementation that stores registrations in a `Vec` and returns
matches in insertion order rather than sorting by `(identity, manifest
digest)`.

### The `cargo-deny` ban is a positive fail, not an absence of use

The `cargo-deny` configuration SHALL list `inventory`, `linkme` and `ctor`
as denied dependencies, verified by a test that confirms the deny rule
itself is present and triggers when one of the three crates is added to the
dependency graph — not merely by the crate's current absence, which a
future contributor could silently reintroduce without the gate catching it.

### The lint gate scans the registry module specifically

The lint gate SHALL scan the registry module (the module implementing
FR-075) for `static`, `OnceLock` and `thread_local!` items and SHALL fail
when it finds one, verified by a test that injects one of the three into a
copy of the module and confirms the gate reports it.

### One unit test per §5.2 row

Each of ADR-012 §5.2's five rows (duplicate `BackendId`; a capability kind
outside the vocabulary; an unregistered named `BackendId`; a capability kind
no registrant advertises; more than one registrant matches with no named
backend) SHALL have its own unit test asserting that row's documented
outcome, independent of the others.

### The S7 seam probe

The registry's match over the capability-kind enum (advertisement
validation and candidate matching) SHALL carry a probe variant behind the
`seam-probe` cargo feature, consistent with FR-063's mechanism for S1-S4,
and `xtask seam-probe` SHALL report the registry's match sites among the
checked-in `E0004` locations when the probe feature is enabled.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-080-AC-1 | Running the permutation property test against a reference registry implementation with at least three registered backends and at least 20 randomly sampled permutations of their registration order produces no failure; running the same test against a mutant implementation that returns candidates in raw insertion order (undoing the `(identity, manifest digest)` sort) produces a failure on at least one sampled permutation. | Test (TC-194, owned by FR-075) |
| FR-080-AC-2 | `cargo deny check` fails when `inventory`, `linkme` or `ctor` is added to the registry crate's dependencies, and passes on the unmodified tree. | Test (TC-205) |
| FR-080-AC-3 | The lint gate fails when a `static`, `OnceLock` or `thread_local!` item is present in the registry module, and passes on the unmodified module. | Test (TC-206) |
| FR-080-AC-4 | Each of ADR-012 §5.2's five rows has at least one unit test, and each such test fails if its row's documented outcome is not produced (for example, a test for the duplicate-`BackendId` row fails if the second registration is silently accepted instead of refused). | Test (TC-207) |
| FR-080-AC-5 | Under the `seam-probe` feature, `xtask seam-probe` reports an `E0004` location at the registry's capability-kind match arm when a probe variant is added to the capability-kind enum, and this location is in the checked-in expected-location list. | Test (TC-208) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §5.3
  "Registry" bullet is the exact evidence list this requirement implements:
  "a property-based test that registries built from any permutation of the
  same descriptors are equal and give identical candidate sets; a
  `cargo-deny` ban on `inventory`, `linkme` and `ctor`, and a lint gate that
  finds no `static`, `OnceLock` or `thread_local!` in the registry module;
  one unit test per §5.2 row."
- ADR-012 §5.1 row S7 names the capability-kind enum and its registry
  advertisement-check seam this requirement's seam probe covers.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md) is the
  registry implementation this evidence is gathered against; FR-075-AC-2 is
  the behavioral claim the permutation property test (TC-194) verifies.
- [FR-063](FR-063-exhaustive-family-extension-seam-probe.md) is the existing
  seam-probe mechanism (S1-S4) this requirement's S7 probe follows the same
  pattern as.

## Status

Specified under
[quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185).
Not yet implemented.
