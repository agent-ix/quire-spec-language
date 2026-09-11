---
id: FR-115
title: "Report observation adequacy without promoting missing evidence"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-009
    type: implements
---
## Description

When emitting an observation contribution to an assessment result, the result
adapter SHALL preserve requested capability, assessment execution,
decision-scope progress, decision-scope closure, surrounding-execution progress,
surrounding-execution closure, complete-global-conformance closure, truth,
settlement basis, exact decision support,
activation, participation, completeness, adequacy, claim strength, provenance,
and immutable definition/profile dependencies as separate facts.

## Behavior

The adapter SHALL retain the independent decision-scope progress,
decision-scope closure, surrounding-execution progress and surrounding-execution
closure facts with their exact scope, authority and boundary identities under
[FR-061](FR-061-report-orthogonal-results.md). The adapter SHALL NOT substitute
one progress or closure axis for another, merge them into a single subject state,
or infer either closure from completed assessment execution or completeness.
When a required retained axis is omitted or cross-wired to another axis or
scope, the adapter SHALL refuse the inconsistent contribution.

When a completed assessment has a settled truth with complete exact decision
support, the adapter SHALL preserve that truth and support independently of the
surrounding-execution progress and closure facts admitted by FR-061. The adapter
retains the corresponding limitations in completeness.

The adapter SHALL retain B's typed complete-global-conformance closure record as
a fifth independent fact under [FR-059](FR-059-assess-finite-global-conformance.md)
and [FR-061](FR-061-report-orthogonal-results.md): `closed`, `open`, `incomplete`
or `contradicted` with the exact required execution, branch and workflow
closure-premise identities for a global-conformance claim; `not-required` with
no global premise set for another claim.

When the fifth record or a required global premise is omitted, duplicated,
cross-wired or scope-mismatched, the adapter SHALL refuse the inconsistent
contribution. The adapter SHALL refuse a global premise under `not-required`,
a global claim marked `not-required`, or another progress/closure axis replaced
with `not-required`.

While complete-global-conformance closure is open, incomplete or contradicted,
the adapter SHALL NOT report complete global success from a closed decision
scope, closed surrounding execution or a decisive witness/counterexample alone.

An untriggered compensator, absent external observation, unsupported obligation,
or missing trigger telemetry SHALL NOT count as demonstrated success or as an
adequacy result whose required activation and participation evidence is present.
A confirmed fault and independently observed mitigation SHALL retain failure-mode,
property, binding, and run lineage through the B-owned result contract; this
requirement creates no second evidence store. When the result adapter emits a
settled truth on an open subject, it SHALL retain its `closed-scope`,
`decisive-witness`, `decisive-counterexample`, `unsettled`, or `unavailable`
basis and every exact supporting fact/progress identity. When a later
contradiction is admitted, the result adapter SHALL retain earlier result bytes
and emit only a linked superseding or invalidating result under the corrected
input identity.
The adapter SHALL retain every selected definition/profile identity, revision,
and digest that its contribution depends on; it SHALL NOT replace one with a
current default or a compatible-looking definition.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-115-AC-1 | Healthy, violating, untriggered, missing-data, and unsupported traces remain visibly distinct. | Test (TC-145) |
| FR-115-AC-2 | Dropped trigger telemetry and mis-correlated response cannot improve adequacy. | Test (TC-145) |
| FR-115-AC-3 | Confirmed fault/mitigation lineage retains source, binding, property, and run identities. | Test (TC-145) |
| FR-115-AC-4 | Assessment execution, decision-scope progress, decision-scope closure, surrounding-execution progress, surrounding-execution closure, complete-global-conformance closure, truth, settlement basis, exact decision support, activation, participation, completeness, adequacy, claim strength, and provenance remain independently readable. Each global closure state retains its exact required premise set; another claim retains `not-required` with no global premise set. Independently omitting, duplicating or cross-wiring a required axis or global premise, supplying a premise under `not-required`, or substituting `not-required` for a global claim or another axis refuses the inconsistent contribution. | Test (TC-145) |
| FR-115-AC-5 | Decisive-witness satisfied and decisive-counterexample violated results remain settled with exact support while the decision scope and surrounding execution are open; compatible changes to surrounding-execution progress/closure preserve the same deciding support and truth. A closed decision scope does not close the surrounding execution or establish complete global conformance without its own required closure premises. A late contradiction retains prior bytes and emits only a linked superseding or invalidating result. | Test (TC-145) |
| FR-115-AC-6 | Every selected definition/profile identity, revision, and digest remains immutable at the observation-result boundary; drift or substitution is refused. | Test (TC-145) |

## Dependencies

- [Shared foundation](../../proposals/quire-v1/shared-foundation.md).
- [B's reviewed FR-061 result contract](FR-061-report-orthogonal-results.md) owns assessment execution, independent decision-scope and surrounding-execution progress and closure, the fifth typed complete-global-conformance closure record, settlement basis, decision support, and result-combination semantics.
- [B's FR-007 result contract](ix://agent-ix/quire-verification/FR-007) owns independent execution, activation, observation, participation, assumption, logical, and provenance findings; Quoin retains evidence authority.
