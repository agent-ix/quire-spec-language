---
id: TC-134
title: "Preserve compensation and recovery emission prerequisites"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-048, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: references }
---
# TC-134: Preserve compensation and recovery emission prerequisites

## Description

Define successor verification for split-shipment, payment-retry and refund
choreography through source checking, checked-package v2 emission and
independent artifact reading. The existing legacy B8 test is historical
coverage; it does not verify the checked-package v2 structural identities or
checked-route clock-configuration admission specified here. Runtime evaluation
of checked compensation expressions is covered by TC-1070 through TC-1073 and
TC-142, not by this artifact-emission test.

## Test Procedure

Compile parallel shipments S1/S2, one authored payment attempt A1 inside a
bounded repeat, forward effect E1, refund registration/activation, distinct
refund attempts/effects R1/R2, a forbidding commit and full/partial recovery
predicates. Inspect exact operation, subject, target/capture, anchor, temporal,
relationship, population, progress and closure requirements. For the timed
family's checked-package v2 successor inspects its `/2` selection: each
temporal definition is identified by structural `NodeId {package, path,
ordinal}`, and the selected temporal profile by its `DefinitionRef {authority,
identity}` in `lock.profile_selections` with role `temporal_profile`. Each
selected temporal declaration/definition has exactly one typed clock configuration
tagged to that selection. Refuse a missing, duplicate, untagged or
selection-incompatible configuration; verify the strict `/1` reader rejects
`/2` and remains unchanged. Keep the legacy B8 test as historical coverage;
do not count it as evidence for these checked-package v2 assertions. Swap or
omit each prerequisite independently and include an
unactivated compensation control. Exercise attempt maxima one and the largest
representable positive value, and refuse zero plus one beyond that
representation.

For one registered forward effect, inspect the retained requirement that two
distinct concurrently eligible triggers need admitted causal/sequence order,
that redelivery of the selected semantic trigger retains a distinct receipt
without reactivation, and that a later distinct eligible trigger does not
reactivate the same registration. Mutate the static trigger, order,
registration or D-owned relationship selection independently and verify the
exact compiler refusal. Keep concrete F-owned occurrence correlation and the
resulting incomplete/refused assessment outside this compiler test.

## Expected Results

The positive artifact retains one compensation obligation with distinct retry
attempt/effect requirements and exact recovery dependencies. Operation success
cannot replace effect or recovery. Foreign or reordered prerequisites refuse;
missing authoritative exports remain unsupported. Unactivated recovery requires
no fabricated future record. The artifact retains the downstream order,
semantic-trigger, receipt-provenance and no-reactivation requirements but makes
no claim about a concrete trigger set or F/B outcome. Distinct runtime ordinals
for A1 remain assessment inputs and do not mint A2 in the static artifact. The
checked-package v2 successor retains the `/2` temporal-definition and profile
identities and one typed clock configuration per selected temporal
declaration/definition without performing L5 settlement. The current legacy
B8 test does not establish these checked-package v2 assertions. This test
evaluates no expression; TC-142 covers only
the checked-package read-back integration, while TC-1070 through TC-1073 own
the evaluation cases.
