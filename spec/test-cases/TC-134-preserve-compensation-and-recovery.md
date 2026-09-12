---
id: TC-134
title: "Preserve compensation, retry, commit and recovery prerequisites"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-048, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: references }
---
# TC-134: Preserve compensation, retry, commit and recovery prerequisites

## Description

Verify the split-shipment, payment-retry and refund choreography through real
native admission, emission and independent artifact reading.

## Test Procedure

Compile parallel shipments S1/S2, one authored payment attempt A1 inside a
bounded repeat, forward effect E1,
refund registration/activation, distinct refund attempts/effects R1/R2, a
forbidding commit and full/partial recovery predicates. Inspect exact operation,
subject, target/capture, anchor, temporal, relationship, population, progress
and closure requirements. For the timed family inspect its `/2` selection: each
temporal definition retains identity, revision, raw-byte digest and artifact,
with exactly one profile-tagged clock configuration. Refuse a missing, duplicate,
untagged or profile-incompatible clock configuration; verify the strict `/1`
reader rejects `/2` and remains unchanged. Swap or omit each prerequisite
independently and include an unactivated compensation control. Exercise attempt
maxima one and the largest representable positive value, and refuse zero plus
one beyond that representation.

For one registered forward effect, inspect the retained requirement that two
distinct concurrently eligible triggers need admitted causal/sequence order,
that redelivery of the selected semantic trigger retains a distinct receipt
without reactivation, and that a later distinct eligible trigger does not
reactivate the same registration. Mutate the static trigger, order, registration
or D-owned relationship selection independently and verify the exact compiler
refusal. Keep concrete F-owned occurrence correlation and the resulting
incomplete/refused assessment outside this compiler test.

## Expected Results

The positive artifact retains one compensation obligation with distinct retry
attempt/effect requirements and exact recovery dependencies. Operation success
cannot replace effect or recovery. Foreign or reordered prerequisites refuse;
missing authoritative exports remain unsupported. Unactivated recovery requires
no fabricated future record. The artifact retains the downstream order,
semantic-trigger, receipt-provenance and no-reactivation requirements but makes
no claim about a concrete trigger set or F/B outcome. Distinct runtime ordinals
for A1 remain assessment inputs and do not mint A2 in the static artifact.
Attempt-bound boundary outcomes preserve the same registration, operation and
subject identities. The timed positive retains the authenticated
`/2` temporal/clock selections without performing L5 settlement; no `/1` artifact
or inferred clock parameter substitutes for them.
