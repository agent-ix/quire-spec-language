---
id: TC-792
title: "StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-301
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-792: StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix

## Description

Verify that a model cause raised inside an enclosing family's evaluation keeps
`StateModel` attribution. Scope: FR-301-AC-1, FR-301-AC-2.

The typed Value-call controls additionally verify FR-100-AC-14 through
FR-100-AC-19. They supplement the two original procedures and results.

## Test Procedure

1. Compile a unit with a `Value` function whose body is `lookup<T>(p, r)`;
   run it through the spine run entry with a population in which `r` is
   absent and the lookup is `absent refused`.
2. Compile a unit with a state clause whose body calls a dispatched query
   operation whose effective precondition is `false` for the receiver; run
   the clause over a snapshot holding that receiver.

### Typed Value-call controls for step 1

Use the actual owning source parser, model selection/normalization and
population/object admission APIs. Do not construct checked nodes, a trusted
effective view, fixed opaque ids, or an evaluator-only substitute. The native
source declares the existing Population parameter form and a Reference
result; its body is exactly `lookup<T>(p, r) absent refused`. Keep the same
source and selected model for both controls. If the source/checked producer
is unavailable, retain this test UNRUN rather than count that failure as
the required model refusal.

Admit an object world holding a real object `r`, and a closed query
population in the same selected model/universe that does not contain it.
Use the PopulationId minted by real binding admission and registered in
that actual environment, and the world's admitted Reference. Call
`qsl_replay::spine::run` with shared typed arguments, the borrowed context,
the actual caller limits and the original Cancel. Retain evidence that
the lookup reached the enclosing Value evaluation: its real catalog cause,
payload and lookup location, not merely a generic non-success exit.

Repeat with an admitted population containing `r`, preserving the source,
model, universe and Reference triple. Make independent mutations of kind,
unregistered population identity, declared maximum, model/universe and
object-world closure. Include a function that ignores its arguments to
check that admission is independent of body use. Combine a wrong-kind
value with a later missing parameter to retain binding-failure precedence.
Exercise malformed raw identity input through the real constructor/reader;
do not forge a typed malformed reference.

Retain all existing scalar tests and their oracles when adapting callers to
the replacement signature. Exercise actual Requested and Deadline causes
at front-end, applicable admission and evaluation boundaries; retain their
different causes and the complete configured accounting limits. Check
zero evaluation work for pre-call failures and exact/one-less real lookup
evaluation charges with admission already permitted, without resetting or
borrowing a helper/population budget. No source-derived count is a runtime
measurement. Repeat the absent/present pair through the driver's use of the
same reviewed public Rust entry, without driver-owned carrier semantics.

## Expected Results

1. `FamilyResult::Refused`; the cause's catalog code begins `state-model`,
   not `value`; category refusal.
2. `FamilyResult::Undefined` with cause `precondition-false`; catalog code
   begins `state-model`, not `protocol-clause`; category undefined.

### Typed Value-call control results

- The absent control reaches the actual lookup and produces the enclosing
  `FamilyResult::Refused` with the StateModel `absent-key` cause, actual
  population/key fields and source location, category refusal, exit 20.
  Setup, compilation, result-kind selection and input admission refusals
  cannot satisfy it.
- The present control completes with the exact supplied Reference triple,
  category success, exit 0; result serialization retains its universe,
  most-specific type and object identity. Always refusing lookup fails.
- Invalid input retains the owning typed construction, model or call
  admission refusal and required fields. A dangling reference is distinct
  from an admitted real reference absent only from the query population.
  No evaluation runs, including for unused parameters. Missing parameter
  takes precedence over the independent wrong-kind value.
- Scalar results and all original refusal/internal-fault oracles remain
  equal. All ten actual accounting ceilings and each actual Cancel cause
  remain intact. Exact/one-less evaluation uses real charge records, not
  inferred helper totals; Requested and Deadline remain distinguishable.
- Driver and library observations agree on these outcomes through the
  same reviewed call boundary. Until actual source, shared-helper,
  strict-reader/admission and consumer qualification exist, these controls
  remain UNRUN. The separate step 2 dispatch/snapshot acceptance remains
  with its SM1 owner.
