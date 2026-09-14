---
id: IT-009
title: "Consume model and binding contracts in composed package linking"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-047
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: references
  - target: ix://agent-ix/quire-specification/IT-010
    type: references
  - target: ix://agent-ix/filament-core-data/FR-117
    type: depends_on
  - target: ix://agent-ix/filament-core-data/FR-127
    type: depends_on
  - target: ix://agent-ix/filament-core-data/FR-129
    type: depends_on
---
# IT-009: Consume model and binding contracts in composed package linking

## Objective

Verify that the compiler binds the shared order/refund example through the
selected real model producer and retains the static correspondence and role
requirements owned by FR-036. This integration is an enablement gate for the
referenced evaluation requirements; it does not execute or verify them.

## Target Integration

The public Rust compiler intake/linking path consumes D's Producer interface
1.2.0 at
[accepted merge 404288282402d60de007295ccbafa960532b955e](https://github.com/agent-ix/filament-core-data/tree/404288282402d60de007295ccbafa960532b955e/crates/baseline-producer)
and its constructor-admitted bundle, exports and producer/native
correspondence. The Cargo dependency and lock SHALL select that exact immutable
revision. This is a producer/API boundary, not a second model reader or a
fabricated model-shaped JSON mock. Downstream capability selection uses the
compiler's request inventory boundary.

F is outside the exercised IT-009 boundary. F's selected observation contract
supplies concrete records, occurrence correlation, membership, observation
anchors, progress and completeness only at assessment time. IT-009 retains the
static requirements for those later inputs without supplying F data or claiming
observation admission, evaluation, protocol conformance or assessment results.

## Preconditions

The affected shared standard is accepted by quire-specification PR #15. The
producer contract and implementation are accepted by filament-core-data PR #99
at merge `404288282402d60de007295ccbafa960532b955e`, and the compiler's composed
entry point exists. The test must compile against that selected revision;
neither a model-shaped mock nor the old finite-state fixture discharges it.

## Inputs

The standard's two-order ecosystem scenario with Order, Payment and Refund model
exports, a shared predicate, state operation contract, temporal obligation and
protocol. Include distinct O1/O2 workflow relationship roles, a finite selected
population role and an explicitly typed clock role. Static input contains exact
definition/model selections and binding contracts; no live instances or future
observations are required to link the template.

## Test Procedure

1. Obtain the admitted model exports and correspondence from the selected real
   producer, then submit the explicit source/definition inventory to the compiler.
   IT-009-SC-01: exact export ownership, native byte selections and producer
   canonical selections are retained in their own domains.
2. Link with no assessment records. IT-009-SC-02: all three families bind the same
   declared model identities and retain distinct relationship/population/clock
   role requirements without fabricating runtime values.
3. Substitute a foreign same-shaped model export and independently swap a producer
   canonical digest into a native byte selection. IT-009-SC-03: each dependent
   binding refuses with its selected source and cause; independent bindings remain.
4. Inspect roles for O1/O2 and the selected population. IT-009-SC-04: each retains
   its owning declaration, endpoint/instance constraints and finite scope premises;
   equal field values or local role spelling cannot collapse those requirements.
5. Submit supported state and unsupported temporal-projection requests against
   the admitted subject. IT-009-SC-05: both request dispositions survive and the
   aggregate cannot claim complete success.
6. Inspect the resolved `agent-ix-baseline-producer` source and execute the same
   direct adapter suite after selecting the accepted producer revision.
   IT-009-SC-06: both the manifest and lock select exactly
   `404288282402d60de007295ccbafa960532b955e`,
   the producer crate remains `publish = false`, and every success/refusal above
   is produced by the compiled accepted dependency rather than by copied types.

## Expected Results

Each success criterion is asserted unconditionally. Linking establishes exact
static correspondence, not evaluated ecosystem truth. Evaluation, graph
traversal, protocol conformance and observation completeness remain assigned to
FR-046 through FR-049, NFR-009 and the downstream B/F contracts; they are not
verified by this integration.

## Status

`tests/producer_correspondence.rs::real_producer_retains_cross_family_identity_and_backend_refusal`
and its direct-adapter mutation controls execute the public Producer 1.2 adapter
and composed compiler boundary. They compile against the accepted producer
merge selected by Cargo, construct the producer's constructor-admitted type and
never deserialize or reinterpret its private admitted representation. The
three source families retain one exact admitted producer/native model selection,
the emitted state, temporal and protocol binders share the selected nominal
`Node` type without merging declaration-owned handles, and temporal captures,
workflow instances, role instances and relationship bindings remain distinct.
The same run retains an admitted state request and an unsupported temporal
projection, makes aggregate success unavailable and exposes no unsupported body
as checked. It supplies no assessment observations and makes none of the
downstream evaluation claims excluded above.

The producer acceptance proves only the static half of Producer interface 1.2.
This integration does not claim population, snapshot, window, observation,
progress or assessment-closure support from that static admission.
