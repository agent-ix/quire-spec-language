---
id: IT-005
title: "Qualify native formal-declaration consumption and static judgments"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: verifies
---

## Objective

Exercise native resolution and static typing against explicit formal declarations,
preserving source identity and observing specified refusals. This is an LC02
integration boundary within IT-002's full state workflow.

## Target Integration

The native compiler consumes Contract IR FR-013 DeclarationEnvironment and
FR-019 public Rust interfaces. FR-023 BoundPackage is the existing later
executable projection binder. The pinned upstream decision is accepted ADR-0054
at 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f, merged in PR61 with issue #54 closed.
No Filament reader, internal agent-ix-semantic-ir dependency, universal model
adapter, or new Contract IR model layer is needed for generic language work.

## Preconditions

The owner adopted specification e897f810a7356d4ce8fd19026221ebda7b65596f for
internal implementation; ADR-0054 subsequently corrects the model-ownership
assumption. Retain those original bytes and read the ownership correction
explicitly rather than silently changing their digests.

A specifies its concrete native request/result API, source/revision mapping,
import/closure representation and resource/error limits before implementation.
FR-013 and docs/formal-environment-linking.md now specify the resolution API,
canonical formal-artifact selection, explicit self binding, retained source
labels and structured declaration locations. Numeric source revision/anchor
mapping is required by the later lowering API, not invented during linkage.
The public IR interface already exists; this is native implementation work,
not a request for another shared model service. Use exact IR and native pins.
Record qualification with Rust 1.98.1 in accordance with the upstream FR-019
policy; distinguish actual executed commands from proposed migration metadata.

Missing native implementation leaves a case planned. Failure of an intentionally
adverse input must be observed through the actual implementation, not substituted
by a missing setup or a skip. Original source bytes and loci remain immutable.

## Inputs

The generic integration lane constructs admitted formal environments through
the real public Rust API and records the explicit native source correspondence.
Its initial positive model is an acyclic BoundedCounter record with a value
field of signed integer 0..1000, reject overflow, and a reviewed identity under
its requirement owner. This is an independently authored formal test model,
not a generated Filament datatype relabeled as proof semantics.

Use valid exact import/source bindings plus separately admitted missing,
ambiguous and stale native-resolution controls. Each environment is validated
by the shared API; ambiguity at native export resolution is distinct from an
invalid duplicate inside one IR environment. Earlier malformed-input refusals
cannot impersonate the required native diagnostic.

The existing ConfigVersion source/compiled fixture and the abstract rule-model
hypotheses in specification typing-cases.json remain separate inputs for the
concrete object/definedness case. Both VersionNumber and Version have bounds
0..1000 but different declaration identities and containing models. The actual
older min-1-only model is a separate refused control. None supplies object or
reference semantics from layout, and no historical fixture is rewritten.

A, as the native modeling-language owner, specifies and qualifies the narrow
reference/population and other mappings required by those concrete cases. IR
refusal is correct for constructs outside FR-013. This work remains part of the
full state-workflow goal but does not block the generic lane. No new TypeSpec/
Node producer execution is authorized or required by this integration design.

## Test Procedure

1. Record exact native/IR revisions, public constructors, formal declarations,
   source correspondence and compiler version for the selected lane.
   - IT-005-SC-01: The generic lane uses the existing formal API; a concrete
     semantic extension is identified only for cases that need it.
2. Run TC-020 through native resolution over the valid formal environment.
   - IT-005-SC-02: Owner-qualified type/field identities and native occurrence
     spans survive in a separately constructed LinkedPackage.
3. Run TC-021–TC-024 with the admitted native adverse controls.
   - IT-005-SC-03: Missing/ambiguous/stale requests return the required located
     diagnostics and no partially successful LinkedPackage.
4. Run the TC-025–TC-029 judgments when their concrete native semantics exist.
   - IT-005-SC-04: Compare actual static judgments with the independently
     authored expectations; unsupported mappings remain explicit, not erased.
5. Record commands/API harness, revisions, input bindings and observations.
   - IT-005-SC-05: Distinguish native outcomes, IR projection refusals, remaining
     native implementation and unexecuted concrete semantic cases.

## Expected Results

The ten TCs pass only after real execution. A generic lane cannot establish all
native definedness semantics or replace IT-002's healthy/violating/refused-or-
incomplete runtime and backend observations. There is no external #54 delivery
gate; A owns the remaining native specification and implementation.

## Metadata

Priority: High. Status: planned native integration. Local Rust verification only.
The native implementation specification owns its bounded work and runner limits.

## Dependencies

- [FR-005](../functional/FR-005-link-shared-model.md)
- [FR-006](../functional/FR-006-check-defined-expressions.md)
- [IT-002](IT-002-native-state-workflow.md)
- [TM-003](../model-linking/tests.md)
