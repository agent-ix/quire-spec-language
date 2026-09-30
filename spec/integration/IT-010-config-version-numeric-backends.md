---
id: IT-010
title: "Execute ConfigVersion through numeric verification backends"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-032
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-034
    type: verifies
  - target: ix://agent-ix/quire-contract-codegen/FR-001
    type: references
  - target: ix://agent-ix/quire-contract-codegen/FR-003
    type: references
  - target: ix://agent-ix/quire-contract-codegen/FR-008
    type: references
  - target: ix://agent-ix/quire-contract-codegen/FR-013
    type: references
---
# IT-010: Execute ConfigVersion through numeric verification backends

## Objective

Verify the first complete numeric formal-specification path using actual checked native source,
native execution, executable IR, generated Rust oracles, model-domain proptest strategies and Kani
contracts. The test exposes any disagreement without converting invalid, unsupported or incomplete
input into a Boolean result.

## Target Integration

The public Rust path is native model/source parsing, linking, checking and packaging →
`lowering::lower_for` with `integer-ir/v1` or `state-scalar-ir/v1` → the
quire-contract-codegen public APIs → compiled generated Rust, proptest and cargo-kani.
`runtime::execute` supplies the native verdict for the same immutable input.

## Preconditions

SL's codegen dependency includes numeric oracle, constructive strategy and numeric/state Kani
support. The generated code consumes quire-contract-runtime. cargo-kani is available locally.
Missing prerequisites fail the test. No hosted workflow is dispatched.

## Inputs

Use the checked-in ConfigVersion model and native `VersionUnchanged` postcondition over
`versionNumber: 0..1000`, plus a plain obligation-free integer comparison over the same domain.
The shared deterministic corpus contains `-1`, `0`, `1000` and `1001`; `-1` and `1001` are
deliberately outside the model domain. In-domain state pairs include unchanged and changed values.
The violating Kani subject always returns an in-domain value different from its input so its
counterexample can be replayed as a valid native invocation. Because VersionUnchanged has no
authored precondition, its Kani adapter uses an explicitly identified, zero-dependency Boolean true
precondition; this is the exact logical identity for absence of a precondition, not an authored
source clause or an approximation of unsupported semantics.

## Test Procedure

1. Generate the Kani bundle for the ConfigVersion clause.
   IT-010-SC-01: the Kani bundle retains the backend options codegen requires.
2. Compile the actual ConfigVersion and plain-integer native fixtures, lower them through their
   explicit targets and pass the emitted bytes through the public strict IR reader to codegen.
   IT-010-SC-02: the numeric oracle, strategy bundle and Kani bundle retain the authored clause,
   requirement, model bounds, pre/post observation identities and original source spans.
3. Evaluate every shared-corpus assignment through the bounded backend adapter and
   `runtime::execute`.
   IT-010-SC-03: in-domain assignments produce equal Boolean verdicts from native execution and
   compiled generated oracles; `-1` and `1001` are classified outside the model domain by both
   paths, produce no Boolean verdict and never invoke the Boolean oracle.
4. Compile and run the generated satisfying, violating, broad and boundary strategy surfaces.
   IT-010-SC-04: every sampled value is within `0..=1000`, every tagged expectation matches the
   generated oracle and native verdict, framework discards are zero, and the accepted/rejected/
   discarded counts are reported.
5. Run the generated Kani contract against the identity subject and an always-changed, in-domain
   subject, with concrete playback enabled.
   IT-010-SC-05: the identity subject proves, the changed subject fails, the printed counterexample
   and canonical absent-precondition identity are retained, the printed counterexample is decoded
   without substituting another value, and replay through `runtime::execute` returns the same false
   contract verdict.
6. Submit the actual ConfigVersion `ParentOrder` and `NoCycle` clauses to the backend projection/
   generation path.
   IT-010-SC-06: each request refuses at the earliest boundary that owns its unrepresentable
   object/graph semantics (`present` before `deref` for the authored `ParentOrder` expression, and
   `reaches` for `NoCycle`), reports the authored clause and exact source locus, emits no partial
   backend artifact and never fabricates a substitute IR expression.

## Expected Results

Every success criterion passes using actual public Rust APIs, emitted bytes, compiled generated
Rust and the Kani executable. Valid model-domain values agree across native execution,
oracle, proptest and Kani. Outside-domain, unsupported, incomplete and tool-unavailable outcomes
remain distinct from false. A Kani counterexample is accepted only when the exact decoded input
replays through native execution with the same verdict.

## Metadata

- Priority: P0
- Target Integration: native compiler/runtime → contract IR → codegen/proptest/cargo-kani
- Automation: Automated local Rust integration test

## Dependencies

**Upstream:** FR-032, FR-033 and FR-034; quire-contract-codegen numeric oracle, strategy and Kani
revisions delivered by codegen PRs #29, #30 and #31. **Downstream:** no implementation dependency;
the broader object/graph semantics remain gated on a future representable IR design.

## Notes

The Boolean-oracle result type cannot represent invalid arithmetic or an out-of-domain model value.
This test therefore admits only obligation-free comparisons, performs domain admission before a
Boolean oracle call and requires explicit refusal elsewhere. It does not approximate ParentOrder or
NoCycle, edit `resources/native-v1/`, publish a crate or dispatch hosted CI.

## Traceability

IT-010 verifies the ConfigVersion runtime workflow and both numeric/state lowering requirements
against the separately owned codegen contracts. Each `IT-010-SC-NN` token binds to a real Rust test
assertion at implementation time.
