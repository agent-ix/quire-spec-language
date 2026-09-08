---
id: IT-005
title: "Qualify native model consumption and static judgments"
type: IT
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: verifies
---

## Objective

Exercise real native linking and static typing against qualified shared model
declarations, preserving source identity and observing each specified refusal.
This isolates the LC02 prerequisite of IT-002's full state workflow.

## Target Integration

Native ParsedUnit and its future linker/checker consume the shared typed-model
adapter owned by contract-IR #54, coordinated by C under the delivery owner.
The existing #50 BoundPackage decoder is a later executable-projection boundary,
not an alternative source of model authority. No new CLI shape is specified here.

## Preconditions

The owner adopted specification PR8 at e897f810a7356d4ce8fd19026221ebda7b65596f
for internal implementation on 2026-09-08. Its FS02/FS03/FS05 definitions are
the selected target. Independent B/C adoption and qualification remain separate.

Before executing this integration lane, record the reviewed shared adapter's
exact release/commit, actual public entry points, supported object-reference
representation, finite bounds/units/multiplicity/snapshot contexts, native
consumer revision and exact source/requirement revision mapping. Current IR
decc99a430a0894de489102dbab04e83d1fb804f does not supply that typed-model view.
Its supplied-declaration environment does not qualify model provenance.

The native linking/checking implementation and its resource/error contracts
must be specified and reviewed before code. This test design does not invent
missing adapter types, a new model decoder or implementation-specific ceilings.
Missing producer/adapter/native capability leaves this lane unexecuted and
reported as a setup dependency; it cannot pass by skipping inside a test.

## Inputs

Two separately qualified input families are required:

- ConfigVersion: the actual compiled model under tests/fixtures/model-output,
  its source/manifest/lock/provenance and the selected native import. The
  declaration IDs include ix://example/config/type/ConfigVersion,
  ix://example/config/field/ConfigVersion-parent and
  ix://example/config/field/ConfigVersion-versionNumber. Its historical
  producer is 3b75e01c652ba00bb07c352ff5467419401e792b. Existing byte audits do
  not establish typed-model qualification.
- Rule-model judgments: the exact declaration hypotheses and named cases in
  specification e897f81 proposals/state-core/fixtures/typing-cases.json.
  These are abstract independently authored expectations, not model wire or
  an existing compiled model. The shared model owner must supply a reviewed
  qualified realization before these integration cases execute. In particular,
  Version is 0..1000; it must not be confused with ConfigVersion's different
  VersionNumber domain. Preserve the original expectations and compare actual
  static judgments separately. No new TypeSpec/Node producer is authorized.

Original bytes and source locations remain immutable. Missing, ambiguous and
stale controls use the adapter owner's versioned adverse fixtures so that an
earlier malformed-wire refusal cannot impersonate the intended linkage failure.
Synthetic source edits are independently authored Rust test inputs with newly
computed source digests and recorded original occurrence ranges.

## Test Procedure

1. Check all preconditions and record the two qualified input families.
   - IT-005-SC-01: Every required capability and exact input binding exists;
     absent qualification is recorded as unexecuted setup, not a passing test.
2. Run TC-020 through the actual adapter and native linker.
   - IT-005-SC-02: Exact public type/field IDs and native occurrence spans are
     retained in a separately constructed LinkedPackage.
3. Run TC-021–TC-024 with isolated admitted adverse controls.
   - IT-005-SC-03: Missing/ambiguous/stale requests produce their required
     diagnostics and no partially successful LinkedPackage.
4. Run TC-025–TC-029 through the native checker using the qualified rule model.
   - IT-005-SC-04: Each named expected static judgment matches the observed
     result, including refusal codes and source loci; no evaluator is invoked.
5. Record exact commands/API harness, revisions, input digests and observations.
   - IT-005-SC-05: Evidence distinguishes native outcomes, adapter refusals and
     setup absence; no authored hypothetical evaluation value is an observation.

## Expected Results

TC-020–TC-029 pass only after real execution with the qualified inputs. Existing
parser success, audit counts, public constructors or hypothetical judgments
cannot substitute for LC02 qualification. IT-002 still owns healthy/violating/
refused or incomplete runtime and backend observations.

## Metadata

Priority: High. Status: planned, setup unavailable. Local Rust verification only.
The future implementation specification owns bounded native work and runner
budgets before execution; this packet does not silently borrow syntax ceilings.

## Dependencies

- [FR-005](../functional/FR-005-link-shared-model.md)
- [FR-006](../functional/FR-006-check-defined-expressions.md)
- [IT-002](IT-002-native-state-workflow.md)
- [TM-003](../model-linking/tests.md)
