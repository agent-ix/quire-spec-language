---
id: FR-219
title: "Declare the memory model of a parallel in source and select it in the request"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-434
    type: depends_on
---
# FR-219: Declare the memory model of a parallel in source and select it in the request

## Description

S3 SHALL check a `parallel` control's optional `memory M` clause, `M` one of
`sc`, `tso` and `ra`, with default `sc` when absent, and SHALL refuse a
nested `parallel` whose default differs from its outermost `parallel`'s and
a weak default on a `parallel` whose join is not `all` with no `outstanding`
clause (ADR-025 MM-1, MM-3). The model checker SHALL resolve each outermost
`parallel`'s model from the request's `memory` member when it names that
`parallel`, else from its source default, and SHALL apply it to every
`parallel` nested inside (ADR-025 MM-2, MM-3). The resolved models SHALL
enter the obligation identity (ADR-025 MM-5).

## Use case

A verification operator's protocol says its branches assume `sc`. They
check the same source under `tso` and `ra` from the request, with no edit,
to see whether the claim survives x86 or C11 atomics. Each model gives its
own obligation, so a proof under `sc` never answers a request under `tso`.

## Inputs

- The parsed `memory` clause of each `parallel`.
- The request's `memory: [{parallel, model}]` member (QSpec FR-331's
  spelling), each entry naming a control by its scoped path (FR-112).

## Outputs

- On each checked `parallel` (FR-218): `memory: MemoryModelName::{Sc, Tso,
  Ra}`, the source default.
- On the protocol subject (FR-205): the resolved model of each outermost
  `parallel`, which selects the `MemoryModel` (`Sc`, `Tso`, `Ra`) the
  protocol system runs it under.
- A typed refusal on a bad clause or selection.

## Behavior

### Source

- S3 SHALL record `Sc` for a `parallel` with no `memory` clause.
- When a nested `parallel` declares a default other than its outermost
  `parallel`'s, S3 SHALL refuse it with
  `invalid_model_binding`/`conflicting-binding`, naming both controls.
- When a `parallel` declares `tso` or `ra` and its join is not `all`, or
  has an `outstanding` clause, S3 SHALL refuse it with
  `unsupported_construct`/`declaration-form` at the join, naming the policy
  and the weak default.

### Request selection

- The model checker SHALL resolve each outermost `parallel` (one nested in
  no other `parallel`) to the model the request's `memory` member names for
  it, or to its source default when the member names none.
- The resolved model SHALL govern that `parallel`, its branches and every
  control nested in them, nested `parallel` controls included.
- When a `memory` entry names a nested `parallel` or a control that is not a
  `parallel`, or two entries name the same `parallel`, the model checker
  SHALL refuse the request `invalid_runtime_input`/`invalid-value`, naming
  the entry, or both entries (QSpec FR-434).
- When a request selects `tso` or `ra` for a `parallel` whose join is not
  `all` with no `outstanding` clause, the pre-check SHALL return the item's
  V-8 outcome, `WeakAccessShape{shape: JoinPolicy}`.
- Sequential outermost `parallel` controls in one protocol MAY resolve to
  different models.

### Identity

- The request writer SHALL write the effective memory model of every
  outermost `parallel`, the request's selection or else the source
  default, into the request, and that model SHALL be part of the request's
  obligation identity (ADR-013 O-09), beside the subject, as QSpec FR-434
  states. A defaulted model SHALL be written as explicitly as a selected
  one. Two requests that resolve every `parallel` to the same models SHALL
  carry the same identity, whether the models came from source or from the
  request.
- The source `memory` clause SHALL enter the checked package's identity as
  source content, and the obligation identity only through the effective
  model (QSpec FR-434).
- The state key's `memory` member (QSpec FR-181) SHALL hold the memory
  component under the effective model (FR-229).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-219-AC-1 | ADR-025 §10's `SB` protocol checks with no `memory` clause (`Sc`) and with `memory tso` (`Tso`). A `parallel` nested in a branch of `SB::Both` checks with no clause or with the same default, and refuses `invalid_model_binding`/`conflicting-binding` naming both controls with another. `memory ra` on a `parallel` joined `any` refuses `unsupported_construct`/`declaration-form` at the join, naming the policy and `ra`; so does `memory tso` with `join all … outstanding continue`. | Test (TC-664) |
| FR-219-AC-2 | Over `SB` with source default `sc`: a request with no `memory` member resolves `SB::Both` to `sc`; `memory: [{parallel: "SB::Both", model: "tso"}]` resolves it and its nested `parallel` to `tso`. An entry naming the nested `parallel`, or naming the attempt `SB::Both::Sx`, refuses `invalid_runtime_input`/`invalid-value`, naming the entry; two entries both naming `SB::Both`, with `tso` and `ra` or with `tso` twice, refuse `invalid_runtime_input`/`invalid-value`, naming both entries. | Test (TC-664) |
| FR-219-AC-3 | The `sc`, `tso` and `ra` requests over `SB` carry pairwise different obligation identities and settle `proved`, `refuted` and `refuted` (ADR-025 §10). `SB` with source `memory tso` and no `memory` member carries the same identity as `SB` with source default `sc` and a `tso` selection. | Test (TC-664) |
| FR-219-AC-4 | A protocol whose `parallel` has no `memory` clause and joins `any`: a request selecting `tso` for it settles V-8, `WeakAccessShape{shape: JoinPolicy}`, before any state is explored; the same request with no `memory` member checks under `sc`. | Test (TC-664) |
| FR-219-AC-5 | Over `SB` with source default `sc`, a request with no `memory` member writes `SB::Both` with effective model `sc` into the request, and its obligation identity equals that of a request with `memory: [{parallel: "SB::Both", model: "sc"}]`. `SB` with source `memory tso` and `SB` with source default `sc` have different checked-package identities, while their `tso` requests carry one obligation identity. | Test (TC-664) |

## Dependencies

- ADR-025 §1 MM-1 to MM-5; ADR-013 O-09; ADR-027 SE-4.
- [FR-112](FR-112-build-protocol-scoped-anchor-forms.md) (scoped paths),
  FR-205 (the protocol subject), FR-218 (checked `parallel`).
- QSpec owns the `memory` clause's grammar, the request's `memory` member
  and the resolved models in the obligation identity, and the catalog
  spelling of the refusals (QSpec FR-434, FR-052, FR-331).

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-434 (Linear STD-138).
