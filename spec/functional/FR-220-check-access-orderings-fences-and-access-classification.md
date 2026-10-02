---
id: FR-220
title: "Check access orderings and fences, and classify every access"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-219
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-435
    type: depends_on
---
# FR-220: Check access orderings and fences, and classify every access

## Description

S3 SHALL check an attempt's optional `ordering o` (`relaxed`, `acquire`,
`release`, `acq_rel`, `seq_cst`) and the `fence Name ordering o;` control
(`acquire`, `release`, `acq_rel`, `seq_cst`) (ADR-025 MM-6). S3 SHALL
classify every attempt in every `parallel` as a load, store, read-modify-
write (RMW) or local access over its shared footprint, atomic when it has
an ordering and non-atomic otherwise, and SHALL refuse an ordering its kind
does not take (ADR-025 MA-1 to MA-3). Under a weak model, the checker SHALL
admit only single-location accesses, refusing at S3 what it decides
symbolically and settling the rest in the pre-check (ADR-025 MA-4).

## Use case

A verification operator annotates each attempt with the ordering its Rust
code uses. The compiler tells them when an annotation cannot apply, such as
an `acquire` store, and when an operation touches two shared fields at once,
which no single atomic instruction does, so that what they check under a
weak model corresponds to real accesses.

## Inputs

- The checked `parallel` controls and their attempts (FR-218, FR-114), with
  each attempt's model read and write footprints (ADR-021 POR-1, POR-3).
- The source default (FR-219) and, in the pre-check, the resolved model.

## Outputs

- On each checked attempt in a `parallel`: `AccessClass::{Load, Store,
  Rmw, Local}` symbolically over its shared footprint, and its
  `ordering: Option<Ordering>`.
- A checked `fence` control with its ordering.
- The S3 refusals `ill_typed`/`operator-ineligible` for an ordering and
  `unsupported_construct`/`declaration-form` for an access shape, and the pre-check's V-8 `WeakAccessShape{site, transition, shape}`,
  with `shape` one of QSpec FR-439's two shapes: `MultiLocation{locations}`
  (`multi-location`) for an attempt of FR-220's shape, and `JoinPolicy`
  (`join-policy`) for a weak `parallel` that joins `any` (FR-219).

## Behavior

### Sharing and classification

- A location SHALL be shared in a weak `parallel` when the instantiated
  footprints of attempts in two or more of its branches contain it and at
  least one writes it; every other location an attempt touches SHALL be
  branch-local.
- S3 SHALL classify each attempt in every `parallel`, whatever its source
  default, symbolically over receivers and parameters: a load reads its one
  shared location only; a store writes it with no pre-state read; an RMW
  reads it in the pre-state and writes it; a local attempt has no shared
  location. EN-1 SHALL instantiate the classification per transition
  identity over the universes.
- An access with an ordering SHALL be atomic and one without non-atomic.

### Admitted orderings

- S3 SHALL admit `relaxed`, `acquire`, `seq_cst` or none on a load;
  `relaxed`, `release`, `seq_cst` or none on a store; one of the five on an
  RMW; none on a local attempt; and `acquire`, `release`, `acq_rel` or
  `seq_cst` on a fence.
- When an access carries an ordering its kind does not take, or an RMW
  carries none, S3 SHALL refuse it with `ill_typed`/`operator-ineligible`, naming the
  attempt or fence.
- Under `sc`, orderings SHALL change no behaviour.

### Access shape

- When a `parallel` declares a weak default and an attempt's shared
  footprint symbolically holds two or more locations, an any-object
  location or a population membership, S3 SHALL refuse it with
  `unsupported_construct`/`declaration-form`, naming the attempt and the
  locations.
- When the resolved model is weak and the pre-check finds such an attempt
  instance on instantiation, the pre-check SHALL return the item's V-8 outcome,
  `WeakAccessShape{site, transition, shape: MultiLocation{locations}}`.
- Under `sc`, every attempt SHALL be admitted whatever its shape.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-220-AC-1 | Over ADR-025 §10's `SB` with source default `sc`, S3 classifies `Sx` and `Sy` as atomic stores, `Ly` and `Lx` as atomic loads, and (`c1`, `r`) and (`c2`, `r`) as branch-local. A variant whose loads carry no ordering classifies them as non-atomic loads. A `fetch_add` attempt with `ordering acq_rel` classifies as an atomic RMW. | Test (TC-665) |
| FR-220-AC-2 | A load with `ordering release`, a store with `ordering acquire`, an RMW with no ordering, a local attempt with `ordering relaxed` and `fence F ordering relaxed;` each refuse `ill_typed`/`operator-ineligible`, naming the attempt or fence. A fence with each of `acquire`, `release`, `acq_rel` and `seq_cst` checks. | Test (TC-665) |
| FR-220-AC-3 | An attempt whose operation writes two shared fields of its receiver, in a `parallel` declaring `memory tso`, refuses `unsupported_construct`/`declaration-form`, naming both locations; in a `parallel` with source default `sc` it checks, and a request selecting `tso` for that `parallel` settles V-8 `WeakAccessShape` with shape `MultiLocation` naming both locations. | Test (TC-665) |
| FR-220-AC-4 | `SB` under `sc` with every ordering `relaxed`, and again with every ordering `seq_cst`, reaches the same 16 states and settles `proved` both times. | Test (TC-665) |
| FR-220-AC-5 | The pre-check's `WeakAccessShape.shape` takes exactly QSpec FR-439's two shapes: AC-3's two-field attempt with a request selecting `tso` settles V-8 with `MultiLocation{locations}` naming both locations, written `{"type":"multi-location","locations":[…]}`; FR-219-AC-4's `parallel` that joins `any`, with a request selecting `tso`, settles V-8 with `JoinPolicy`, written `{"type":"join-policy"}`. | Test (TC-665) |

## Dependencies

- ADR-025 §1 MM-6, §2 MA-1 to MA-4; ADR-021 POR-1, POR-3 (footprints).
- [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md), FR-218
  (checked `parallel` and attempts, ADR-027), FR-219 (models).
- QSpec owns the `ordering` member, the `fence` control and the catalog
  spelling of the refusals (QSpec FR-435).

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-435 (Linear STD-138),
  and QSpec FR-439 for the `weak-access-shape` cause and its two shapes.
