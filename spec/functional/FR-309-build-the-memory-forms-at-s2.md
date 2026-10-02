---
id: FR-309
title: "Build the memory clause, access ordering and fence forms at S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-219
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-434
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-435
    type: depends_on
---
# FR-309: Build the memory clause, access ordering and fence forms at S2

## Description

When S2 builds the form of a protocol declaration under
`quire.model.complete/v1`, it SHALL accept the memory surface forms of
QSpec's shared grammar, the `memory-clause` of a `parallel`, the optional
`ordering` of an `attempt` with its `memory-order`, and the `fence` control
with its `fence-order`, and SHALL record each in the protocol forms with
its span (ADR-025 MM-1, MM-6; QSpec FR-434, FR-435). S2 records what was
written; S3 checks it (FR-219, FR-220).

## Use case

A verification operator marks a `parallel` `memory tso`, annotates each
attempt with the ordering its Rust code uses and adds a `seq_cst` fence.
Each annotation reaches the checker with its span, so an inadmissible
ordering is refused at the word the operator wrote.

## Inputs

- The CST of one protocol declaration, parsed by QSpec's shared-grammar
  productions `parallel` with `memory-clause`, the `attempt` alternative of
  `event-node` with `memory-order`, and `fence` with `fence-order`.

## Outputs

In `forms::protocol_clause` (crate `qsl-forms`), beside FR-308's forms:

- `ParallelForm.memory: Option<Spanned<MemoryModelName>>`, with
  `MemoryModelName::{Sc, Tso, Ra}`.
- `AttemptForm.ordering: Option<Spanned<MemoryOrder>>`, with
  `MemoryOrder::{Relaxed, Acquire, Release, AcqRel, SeqCst}`.
- `FenceForm{name, ordering: Spanned<FenceOrder>}`, with
  `FenceOrder::{Acquire, Release, AcqRel, SeqCst}`, as a control.

## Behavior

- S2 SHALL record a `parallel`'s `memory` clause when written and `None`
  when absent; S3 gives the absent clause its `sc` default (FR-219).
- S2 SHALL record an `attempt`'s `ordering` when written and `None` when
  absent; S3 classifies the access and checks the ordering (FR-220).
- S2 SHALL build a `FenceForm` for each `fence` control, in source order
  among its sibling controls.
- S2 SHALL read `memory`, `fence`, `sc`, `tso`, `ra`, `relaxed`,
  `acquire`, `acq_rel` and `seq_cst` as keywords only in the
  `memory-clause`, `memory-order` and `fence-order` positions the shared
  grammar reserves; elsewhere they are identifiers.
- A well-formed CST adds no S2 refusal. A CST that carries an error or
  recovery node SHALL refuse as `FormsCause::RecoveringCst`, with no
  partial form.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-309-AC-1 | ADR-025 §1's `SB` protocol with `parallel Both memory tso` builds `memory: Some(Tso)` with the span of `tso`; the same protocol with no `memory` clause builds `None`. | Test (TC-888) |
| FR-309-AC-2 | `attempt Sx by c1 on M::Cell::set ordering release …` builds `ordering: Some(Release)` with its span; an attempt with no `ordering` builds `None`. `fence F1 ordering seq_cst;` between two attempts builds a `FenceForm` with `SeqCst`, as the second of three sibling controls. | Test (TC-888) |
| FR-309-AC-3 | A field named `acquire` read in a guard builds an identifier, not an ordering. `fence F ordering relaxed;` builds no `FenceForm` and refuses as a recovering CST, since `fence-order` does not admit `relaxed`. Building one declaration twice gives equal forms. | Test (TC-888) |

## Dependencies

- ADR-012 §1, §3 (forms); ADR-025 §1 MM-1, MM-6.
- FR-308 (the protocol forms), FR-219 and FR-220 (the S3 checks).
- QSpec owns the grammar: the shared grammar's `memory-clause`,
  `memory-order`, `fence` and `fence-order` productions (QSpec FR-434,
  FR-435).

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-434, FR-435 (Linear
  STD-138).
