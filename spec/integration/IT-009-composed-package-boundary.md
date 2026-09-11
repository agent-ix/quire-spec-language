---
id: IT-009
title: "Consume model and binding contracts in composed package linking"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: verifies
  - target: ix://agent-ix/quire-specification/IT-010
    type: references
---
# IT-009: Consume model and binding contracts in composed package linking

## Objective

Verify that the compiler binds the shared order/refund example through the actual
model producer and its selected native correspondence, including relationships
between distinct workflow instances and explicitly scoped population roles.

## Target Integration

The public Rust compiler intake/linking path consumes the model producer's
admitted exports and the selected D/F binding contracts. This is a producer/API
boundary, not a second model reader or a fabricated model-shaped JSON mock.
Downstream capability selection uses the compiler's request inventory boundary.

## Preconditions

The affected shared standard and producer contracts are accepted and their
producer/schema implementation supplies the named exports/correspondence. The
compiler's composed entry point exists. Until these conditions hold this test is
planned; neither a mock nor the old finite-state fixture discharges it.

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

## Expected Results

Each success criterion is asserted unconditionally. Linking establishes exact
static correspondence, not evaluated ecosystem truth. Assessment-time rejection
of foreign concrete instances and observation completeness remains the standard's
IT-010/D/F consumer contract; this integration checks that the compiler preserves
the precise requirements that make those later checks possible.
