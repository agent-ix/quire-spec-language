---
id: FR-053
title: "Preserve opaque semantic-trigger identity across native temporal evaluation"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-052, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-093, type: references }
  - { target: ix://agent-ix/quire-specification/FR-230, type: references }
  - { target: ix://agent-ix/quire-specification/FR-300, type: references }
---
# FR-053: Preserve opaque semantic-trigger identity across native temporal evaluation

## Description

When a downstream integration submits an event-triggered native-temporal
evaluation, the compiler SHALL retain the semantic trigger-event identity as
exact opaque bytes through the canonical request, evaluation, result and strict
reader surfaces.

The identity is selected by the observation owner. This compiler validates its
presence, bounded representation and request/result equality; it SHALL NOT
interpret it as UTF-8, synthesize it from a timestamp, receipt, display text,
payload, capture, static binding coordinate or table position, or claim to
authenticate the observation owner.

## Inputs

One FR-051 checked temporal subject; a bounded nonempty opaque semantic-trigger
identity; the existing finite native-temporal inputs and exact selected
definition/profile/clock authority of FR-052. Whole-execution origin remains a
separate selected origin identity and is never encoded as an empty trigger.

## Outputs

One versioned native-temporal request/result pair and constructor-private views
whose trigger accessor returns the byte-exact selected identity and whose
activation accessors retain the checked activation state, selected trigger
captures, and exact evaluation anchor. The views return one typed refusal
without a partial request/result or a string fallback.

## Behavior

The public native-temporal API SHALL replace the caller-authored textual
`instance` field with an opaque byte identity type. The canonical v2 request and
result contracts SHALL encode that type reversibly with a declared binary
encoding, retain it in their identity preimages, and reject missing, empty,
noncanonical, malformed, stale, cross-request or result-substituted values.
The v1 request/result reader remains version-specific and SHALL NOT decode a v2
document as v1 or translate a v1 textual instance into a v2 trigger identity.

For one checked subject and exact request, strict result reading SHALL compare
the complete opaque trigger identity byte-for-byte before it evaluates temporal
truth. A correction preserves its predecessor's immutable trigger bytes; a
changed trigger is a different request/result rather than a correction of the
same obligation.

This requirement publishes only the native-temporal owner side of QSpec
FR-300. QProtocol compares the retained bytes to QObs's authority-qualified
subject and binds the selected activation site; this compiler does not import
QObs Rust types or reconstruct a protocol obligation.

The version-two request view SHALL expose only the selected trigger's immutable
captures and the exact evaluation anchor already admitted into the canonical
request. The version-two result view SHALL expose only the checked activation
state already admitted by strict result reading. Those accessors carry no
QProtocol control handle, QObs subject, obligation identity, or construction
operation; QProtocol alone joins the owner facts into the FR-300 binding.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-053-AC-1 | A non-UTF-8 semantic trigger identity round-trips byte-for-byte through v2 request production, evaluation and strict result reading. | Test (TC-141) |
| FR-053-AC-2 | Replacing the trigger bytes changes request and result identities; a result carrying bytes from another request or a correction with changed trigger bytes refuses. | Test (TC-141) |
| FR-053-AC-3 | Empty, malformed, noncanonical or cross-version trigger encodings refuse with no partial view, Boolean truth or textual fallback. | Test (TC-141) |
| FR-053-AC-4 | A timestamp, receipt, display name, payload, capture, static binding coordinate or declaration-table position cannot substitute for the supplied trigger bytes. | Test (TC-141) |
| FR-053-AC-5 | The v1 reader rejects v2 contracts and v2 rejects v1 textual-instance documents without translation. | Test (TC-141) |
| FR-053-AC-6 | A strictly read v2 request exposes its admitted trigger captures and evaluation anchor, and its corresponding strictly read v2 result exposes the checked activation state; neither view can expose a caller-authored obligation or QObs subject. | Test (TC-141) |

## Dependencies

- [FR-052](FR-052-publish-native-temporal-evaluation-owner.md) owns the
  canonical request/result evaluation and strict-reader boundary extended here.
- QSpec FR-093 and FR-230 establish semantic-trigger and obligation identity.
- QSpec FR-300 establishes the wider activation-binding contract consumed by
  QProtocol.
