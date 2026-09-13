---
id: FR-116
title: "Map observation outcomes to executable and output consumers"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-009
    type: implements
---
## Description

When delivering an observation assessment to an executable or exported consumer,
the consumer adapter SHALL retain the requested subject, capability, immutable
source/binding/progress/definition/profile identity-revision-digest selections,
assessment execution, decision-scope progress, decision-scope closure,
surrounding-execution progress, surrounding-execution closure,
complete-global-conformance closure, truth, settlement basis, exact decision
support, per-obligation disposition, completeness reasons, and mapping-loss
classification as separate facts.

## Behavior

The adapter SHALL classify each mapped obligation as preserved, conditional,
unrepresented, or refused; incomplete, pending, and refused outcomes are not
Boolean success. It SHALL consume A-owned interchange identities and B-owned
result semantics; codegen and export mappings do not become clause authorities.
It SHALL preserve a late contradiction's prior-result identity and linked
superseding/invalidating result without rewriting historical bytes. It SHALL
refuse definition/profile identity, revision, or digest drift rather than emit a
mapping under a current default.

The adapter SHALL retain the four decision-scope/surrounding-execution
progress/closure axes with their exact scope, authority and boundary identities
under [FR-061](FR-061-report-orthogonal-results.md). It SHALL retain the fifth
typed complete-global-conformance closure record: `closed`, `open`, `incomplete`
or `contradicted` with the exact required execution, branch and workflow
closure-premise identities for a global-conformance claim; `not-required` with
no global premise set for another claim. It SHALL refuse omitted, duplicated,
cross-wired or scope-mismatched required axes, records or premises, a global
premise under `not-required`, or `not-required` substituted for a global claim
or another axis. Assessment completion, completeness, settled truth or another
closure axis SHALL NOT replace a required closure fact or global premise.
When the selected consumer cannot represent a required fact, the adapter SHALL
retain the source fact and explicit mapping loss as conditional, unrepresented
or refused; missing representation SHALL NOT be classified as preserved.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-116-AC-1 | A consumer receives source/binding/progress provenance with each per-obligation disposition. | Test (TC-146) |
| FR-116-AC-2 | An unsupported or lossy mapping is visible as conditional, unrepresented, or refused. | Test (TC-146) |
| FR-116-AC-3 | A consumer cannot convert pending, incomplete, or refused into a passed Boolean result. | Test (TC-146) |
| FR-116-AC-4 | Parsing, serializing, or accepting emitted target text does not alter a mapping outcome or establish semantic preservation. | Test (TC-146) |
| FR-116-AC-5 | Consumer delivery preserves assessment execution, decision-scope progress, decision-scope closure, surrounding-execution progress, surrounding-execution closure, the fifth typed complete-global-conformance closure record with its exact required global premises or non-global `not-required` with no global premises, truth, settlement basis, exact decision support, immutable definition/profile dependencies, and late-result supersession without Boolean or historical-byte promotion. Each required axis, record and premise remains independently scope-bound; omission, duplication, cross-wiring, scope mismatch or invalid `not-required` substitution refuses. Missing target representation remains explicit mapping loss and cannot become preservation. | Test (TC-146) |

## Dependencies

- [Shared foundation](../../proposals/quire-v1/shared-foundation.md).
- [Observation result consumer and output-mapping contract](../../proposals/quire-v1/observation-output-mapping-contract.md).
- A owns interchange/versioning; B owns result semantics; D/E own their mapping meanings.
