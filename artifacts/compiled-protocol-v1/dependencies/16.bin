# Native finite global protocol definition

Profile identity: `quire.protocol.finite-global/v1`; revision: `1-draft.4`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**

Working draft: local rule and dependency links select the current checkout.
Git preserves drafting history; a coherent baseline is frozen at acceptance.
This editing policy does not relax immutable runtime selections or digest checks.

## Required definitions and interfaces

| Identity | Revision | Definition artifact | Consumed interface |
| --- | --- | --- | --- |
| `quire.state.graph/v1` | `1-draft.3` | [State/query/graph](state-graph.md) | Checked shared values, guards, captures and predicate references |
| `quire.temporal.bounded-facet/v1` | `1-draft.3` | [Temporal meaning](temporal-common.md) | Explicit activation, bounded clock/history/progress and temporal-reference contract |

The source root admits protocol declarations. State and temporal declarations
select their own profiles; an imported declaration retains that exact selection.
Each authored timed obligation additionally selects its concrete temporal
clock/boundary profile through its `using` alias or referenced temporal clause.
The common temporal facet alone supplies no default clock interpretation.

## Admitted forms and obligations

Admit finite typed roles/relationships/channels; distinct send, receive,
operation-attempt, effect and domain-event nodes; checks; explicit causal
sequence; role-owned labeled choice with total visible exclusive/exhaustive
guards; bounded parallel branches with explicit all-branch join; and bounded
repetition with progress and an explicit exhausted path. No implicit race,
cancellation, message order, reliable delivery or exactly-once business effect
is assumed.

Admit exact related-instance annotations, finite snapshot/window inputs, awaited
events with inclusive deadlines, compensation templates with separate forward
registration and eligible activation captures, causal retries, achieved recovery
relations, commit boundaries and explicit workflow closure. Definite value
availability follows the native causal graph; a sibling or optional branch does
not supply a value before its required join/path. Repeated delivery, invocation
attempt and successful effect remain distinct identities.

All value conditions use the shared expression rules and their supplied anchors;
they cannot inspect pending monitor status, retag captures, infer population
membership or perform business actions. Forward success creates a modeled
compensation obligation, not an external compensator. A successful refund call
alone cannot establish achieved recovery or complete global conformance.

Global conformance checks the exact declared causal, channel, visibility,
correlation, activation, recovery and closure premises. All permitted branch
interleavings remain admissible. Open/incomplete histories cannot establish
complete global success. Finite replay and settled incremental results must
agree for equal admitted inputs/premises.

Projection, realizability, refinement and composition remain separately requested
claims with their own premises and backend dispositions; a finite passing trace
does not prove them. No result grants testing adequacy or evidence sufficiency
from observed conformance. C's assurance completion is not a semantic dependency.

## Selected protocol rules

The native surface fixes concrete syntax/binding details; B's requirements own
protocol/conformance/result meaning. A disagreement is an integration defect,
not permission to choose a favorable interpretation or overwrite either owner.

| Rule | Current normative file |
| --- | --- |
| FR-049 | [Native choreography binding](../../../spec/functional/FR-049-bind-native-choreography-syntax.md) |
| FR-050 | [Protocol instances](../../../spec/functional/FR-050-bind-protocol-instances.md) |
| FR-051 | [Communication identities](../../../spec/functional/FR-051-preserve-communication-identities.md) |
| FR-052 | [Bounded control](../../../spec/functional/FR-052-represent-bounded-control.md) |
| FR-053 | [Choice visibility](../../../spec/functional/FR-053-enforce-choice-visibility.md) |
| FR-054 | [Channel premises](../../../spec/functional/FR-054-bind-channel-premises.md) |
| FR-055 | [Obligation activation](../../../spec/functional/FR-055-activate-protocol-obligations.md) |
| FR-056 | [Compensation registration](../../../spec/functional/FR-056-register-compensation.md) |
| FR-057 | [Commit and recovery](../../../spec/functional/FR-057-enforce-commit-recovery.md) |
| FR-058 | [Retry and partial recovery](../../../spec/functional/FR-058-preserve-retry-and-partial-recovery.md) |
| FR-059 | [Finite global conformance](../../../spec/functional/FR-059-assess-finite-global-conformance.md) |
| FR-060 | [Independent claims](../../../spec/functional/FR-060-separate-protocol-claims.md) |
| FR-061 | [Orthogonal results](../../../spec/functional/FR-061-report-orthogonal-results.md) |
| NFR-020 | [Protocol resource bounds](../../../spec/non-functional/NFR-020-bound-protocol-processing.md) |
| NFR-021 | [Protocol reproducibility](../../../spec/non-functional/NFR-021-reproduce-protocol-results.md) |
| Native surface | [Choreography surface](../choreography-surface.md) |
| Protocol meaning | [Protocol contract](../protocol-contract.md) |

## Result settlement and compatibility

This draft consumes the current canonical result contract, the 1-draft.3
temporal facet and the 1-draft.3 state dependency. It retains assessment execution,
decision-scope progress, decision-scope closure, surrounding-execution progress,
surrounding-execution closure, truth, settlement basis and exact decision support
as independent facts. A completed
assessment may settle an obligation from a decisive witness or counterexample
while the decision scope and surrounding execution remain open, only when the
selected temporal meaning preserves that truth under every admitted continuation.
Missing support makes that decision unavailable; missing unrelated facts remain
visible without erasing independently settled truth. A later contradiction
creates a linked superseding or invalidating result with a corrected input
identity and preserves the prior result bytes.

This does not weaken FR-059's complete global conformance rule. A satisfied
payment obligation cannot certify the still-open workflow, an unknown population
or an unsupported independently requested claim.

The complete-global-conformance closure record is a fifth independent fact:
global claims retain `closed`, `open`, `incomplete` or `contradicted` plus the
exact required execution, branch and workflow premises. Other claims retain
`not-required` with no global premise set. Missing, duplicated, cross-wired or
scope-mismatched required records/premises refuse; none of the other four
progress/closure axes substitutes for this record.

Runtime requests retain their exact selected definitions, identities, revisions
and digest domains. Missing, stale, substituted or incompatible dependencies
refuse their dependent request. A changed semantic selection changes the linked
subject; an existing result cannot be relabelled under a different selection.

The canonical result now carries exact assumptions, separate observation and
protocol adequacy, and decision-support oracle dependence/lineage. Its enclosing
artifact reference digests the complete canonical result, avoiding a digest
inside its own preimage. The outgoing portable verification projection remains
a separate requested adapter under
[FR-062](../../../spec/functional/FR-062-project-results-to-portable-verification.md).
It does not change source meaning or make E02 support a condition of native
protocol admission. An outgoing projection must retain its canonical source
result and preserve all independent target findings under the selected adapter.

The [observation result contribution](../../../spec/functional/FR-115-report-activation-participation-and-adequacy.md)
and [output mapping](../observation-output-mapping-contract.md) preserve these
independent progress/closure axes, settlement, support and linked supersession.
The current [observation contract](../observation-contract.md) supplies the
producer selection and range correspondence. Draft normalization establishes no
implementation, conformance or adoption claim.
