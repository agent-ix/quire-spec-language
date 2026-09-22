---
id: FR-089
title: "Consolidate value::accounting's byte-identical duplicates onto the quire-exact kernel"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-089: Consolidate value::accounting's byte-identical duplicates onto the quire-exact kernel

## Description

ADR-011 §6.1's K-leaf rule and its QSL-166 row record that QSL's
`value::accounting` module duplicated eight `quire-exact` kernel items
byte-for-byte (`length_amount`, `ScalarLimits`, `LimitKind`, `ChargePoint`,
`Incomplete`, `InjectedDenial`, `Charge` and `Meter`): `quire-exact` existed
as their kernel definition, but QSL's own copy was never deleted when the
kernel crate was extracted (#213 S-1/X-1). This requirement formalizes the
consolidation as a standing, regression-tested property rather than a
one-time diff: QSL's own source tree SHALL define none of these four
type names a second time outside `quire-exact`, so a future change cannot
reintroduce the duplicate silently.

`model::accounting` is out of scope: ADR-011 §6.1's QSL-166 row also names
`model::accounting`'s own `Meter`/`Incomplete`/`LimitKind` as a byte-identical
duplicate, but the ticket's ruling (QSL-164, 2026-09-21) keeps
`model::accounting` as layer-3 `model`'s own separate meter, not a kernel
type, so this requirement's verification is scoped to `src/value/`, where
`value::accounting` lived, rather than QSL's whole `src/` tree (see
Behavior).

## Inputs

- QSL's `src/value/` tree and `quire-exact/src/accounting.rs`, as they exist
  after `value::accounting`'s removal.

## Outputs

- A source-level guarantee that `struct Meter`, `struct Incomplete`,
  `enum LimitKind` and `enum ChargePoint` are not defined anywhere under
  QSL's `src/value/` tree, so the only remaining definition reachable from
  `value::*` is `quire-exact`'s.

## Behavior

### No duplicate definition under value::

QSL's `src/value/` tree SHALL define no `struct Meter`, `struct Incomplete`,
`enum LimitKind` or `enum ChargePoint`. Every QSL module that uses these
types SHALL import them from `quire_exact` directly; no re-export shim or
alias module stands in for the deleted `value::accounting` module (ADR-011
QSL-166 row: "none: no shim, no alias module, no re-export").

This is scoped to `src/value/`, not QSL's whole `src/` tree: all four names
have pre-existing, unrelated private or public namesakes elsewhere in this
crate (a module-private JSON/proof-budget `struct Meter` in
`src/checking/proof.rs`, `src/package/encoding.rs`, `src/package/intake.rs`
and `src/protocol_artifact/encoding.rs`; an unrelated `pub enum LimitKind` in
`src/command.rs`; an unrelated `pub struct Incomplete` in
`src/temporal/result.rs`; and `src/model/accounting.rs`'s own, deliberately
separate `Meter`/`Incomplete`/`LimitKind`, kept by the QSL-164 ruling,
2026-09-21). None of these is a copy of the kernel type or this
requirement's concern, and a crate-wide scan by bare name over-claims
against them.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-089-AC-1 | A source scan of QSL's `src/value/` tree finds no defining location for `struct Meter`, `struct Incomplete`, `enum LimitKind` or `enum ChargePoint`. Reintroducing any one of the four (e.g. restoring `value/accounting.rs`) fails this scan. | Test (TC-271) |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1's K-leaf rule and its QSL-166 row.

## Status

Specified and implemented under Linear QSL-166: `value::accounting` is
deleted, its eight items' call sites repoint at `quire_exact::{..}`, and
`model::accounting` is left untouched per the QSL-164 ruling.
