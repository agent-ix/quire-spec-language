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

Compile parallel shipments S1/S2, payment attempts A1/A2, forward effect E1,
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

For one registered forward effect, supply two distinct concurrently eligible
triggers without an admitted causal/sequence order, then the same triggers with
an admitted order. Redeliver the selected semantic trigger under a distinct
receipt identity and then supply a later distinct eligible trigger. Mutate only
the F-owned occurrence correlation or D-owned relationship identity in paired
controls; neither may be reconstructed from the other or from ingestion time.

## Expected Results

The positive artifact retains one compensation obligation with distinct retry
attempt/effect requirements and exact recovery dependencies. Operation success
cannot replace effect or recovery. Foreign or reordered prerequisites refuse;
missing authoritative exports remain unsupported. Unactivated recovery requires
no fabricated future record. An unordered concurrent trigger set remains
incomplete or refused according to the selected F/B input contract and is never
ordered by timestamp or ingestion. The admitted-order case selects one exact
trigger; its redelivery retains receipt provenance without another activation,
and the later distinct trigger does not register or activate a second
compensation. Attempt-bound boundary outcomes preserve the same registration,
operation and subject identities. The timed positive retains the authenticated
`/2` temporal/clock selections without performing L5 settlement; no `/1` artifact
or inferred clock parameter substitutes for them.
