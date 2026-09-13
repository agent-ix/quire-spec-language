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
`lowering::lower_for` with `integer-ir/v1` or `state-scalar-ir/v1` → the pinned
quire-contract-codegen public APIs → compiled generated Rust, proptest and cargo-kani 0.67.0.
`runtime::execute` supplies the native verdict for the same immutable input. Cargo dependency
resolution, generated proof graphs and local command output supply the version and option evidence.

## Preconditions

SL selects codegen revision `5e2a6a994d2107f36294078ad202467a4c66bb75`, which includes numeric
oracle, constructive strategy and numeric/state Kani support. The root dependency graph resolves
quire-contract-ir revision `04eb6f849c03be23177d373549c6c272551f957d` once; codegen's declared IR
identity, SL's lowering types and the lockfile shall agree. The generated code consumes runtime
revision `8a4d02b9ff4633cf6d02fd8bdf6ee1b11ad76354`. Rust 1.98.1 and cargo-kani
0.67.0 with executable SHA-256
`7f143a251d11c7e6e232bbf2cbccf56f9ce66a5f0107eeb3008698e6715f55d9` are available locally.
Missing or mismatched prerequisites fail the test. No hosted workflow is dispatched.

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

1. Resolve the locked Rust dependency graph and inspect the selected codegen, contract IR, runtime
   and cargo-kani identities.
   IT-010-SC-01: the lockfile contains the reviewed codegen revision and one contract-IR revision;
   codegen reports that same IR identity, and the Kani version, executable digest and exact options
   are retained.
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
Rust and the pinned Kani executable. Valid model-domain values agree across native execution,
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

SL previously selected IR revision `690bde7f2dc58662cf9ff0595c2c0e3b17107c6f`; the commits between
the reviewed codegen IR revision and that revision change only IR specification artifacts, not Rust
sources. IT-010 selects the codegen revision's executable provenance across the complete Cargo graph
instead of retaining two byte-identical Rust packages under different source identities.

## Traceability

IT-010 verifies the ConfigVersion runtime workflow and both numeric/state lowering requirements
against the separately owned codegen contracts. Each `IT-010-SC-NN` token binds to a real Rust test
assertion at implementation time.
