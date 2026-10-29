---
id: TC-142
title: "Evaluate compensation slots after strict package read-back"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-390, type: references }
---
# TC-142: Evaluate compensation slots after strict package read-back

## Description

Scope: FR-049-AC-9.

Verify the end-to-end replacement for the retired version-2 state evaluator:
strictly read the package emitted for a compensation fixture, select the exact
checked protocol slots and evaluate them through FR-390's slot entry point.
TC-1070 through TC-1073 own the detailed expression, admission, unavailable
input and accounting cases; this test adds the emitted-bytes/read-back path.

## Test Procedure

Compile the real compensation fixture used by TC-1070 through the supported
checked source pipeline, emit its checked-package V2 bytes and admit them with
the strict version-exact reader. Confirm the admitted package exposes the
identity derived and admitted by that reader, and that neither a missing nor a
caller-substituted artifact identity can be supplied to the evaluation entry
point; there is no `UnpublishedArtifact` runtime branch in this checked-package
path. From the admitted checked package, select the
registration/activation captures, trigger guard, retry relation and recovery
predicate by their exact protocol slots. Supply the corresponding `ProtocolSlot`
and typed `ProtocolSupply` cases from TC-1070 and evaluate through
`CheckedPackageEvaluation::evaluate_protocol_slot` with the shared meter.
Repeat the same slot calls over the checked package before emission. Compare
results and charge logs. Cross one slot with another slot's supply and verify
admission refusal before charge. Do not call `state::evaluate_v2` or construct
an artifact-value `StateView`.

## Expected Results

The strict reader's checked package selects and evaluates the same exact
compensation slots as the source-checked package, with equal typed results and
charge logs. A crossed slot/supply refuses before charge. All protocol
expression, typed-failure, unavailable-input and budget boundaries are the
single cases specified by TC-1070 through TC-1073; this integration case adds
only the emitted V2/read-back round trip.
