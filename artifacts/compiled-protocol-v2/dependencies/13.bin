# Observation progress and restoration definition candidate

Selection identity: `quire.observation.progress/v1`; revision: `1-draft.3`.
**Candidate only: not adopted, serialized, canonicalized, or implemented.**

Working draft: local rule and dependency links select the current checkout.
Git preserves drafting history; a coherent baseline is frozen at acceptance.
This editing policy does not relax immutable runtime selections or digest checks.

This definition selects scoped assertions, contradiction and restoration rules
with the range-aware binding for all three clock families. It is not a clock
profile, event schema, replay engine, result store or executable monitor. E owns
clock/history/closure meaning, B owns result dispositions and D owns the selected
subjects, relationships and populations.

## Required definitions and rules

| Dependency | Identity | Revision/version | Current normative contract |
| --- | --- | --- | --- |
| Observation binding | `quire.observation.binding/v1` | `1-draft.3` | [Observation binding](observation-binding.md) |
| Observation-range correspondence | `quire.observation.range/v1` | `1-draft.1` | [Range correspondence](observation-range.md) |
| Observation binding rules | `quire.observation.contract/v1` | Current working draft | [Observation contract](../observation-contract.md) |
| Temporal lifecycle/result facet | `quire.temporal.bounded-facet/v1` | `1-draft.3` | [Temporal meaning](temporal-common.md) |
| Shared result rules | B FR-061 | Current working draft | [Orthogonal results](../../../spec/functional/FR-061-report-orthogonal-results.md) |
| Producer interface | FCD Producer interface | `1.2.0` | [Producer selection](../observation-contract.md#producer-interface-selection) |

An actual assertion or restoration request selects its required dependencies by
exact identity, revision and bytes. Missing, stale, substituted or inconsistent
dependencies refuse the assertion; local draft links cannot select replacement
runtime artifacts, default to a current clock/profile/result or begin a fresh
complete stream. Producer object, raw document, definition and result digests
remain distinct domains.

## Required assertion identity

Every progress, completeness, closure, late-contradiction or restoration assertion
retains the following immutable selections:

| Value | Required meaning |
| --- | --- |
| Assertion identity | Assertion kind, unique assertion identity and exact progress definition identity/revision/bytes. |
| Binding selection | Exact binding definition identity/revision/bytes and applied role/binding identities. |
| Scope selection | Exact workflow/subject/relationship handles, population or window identity, membership rule and required source set. |
| Temporal and range selection | Exact clock/profile identity/revision/bytes, selected range and coverage row, inclusive asserted boundary and history/closure premise. |
| Record and revision | Admitted-record identities, ingestion/order information and the retained observation/history revision to which the assertion applies. |
| Restoration selection | Exact predecessor assessment/checkpoint identity, retained-state revision, resource-limit identity and replay basis when continuing from retained state. |

No local clock name, timestamp, EOF marker, quiet transport, trace identifier or
matching field shape can substitute for these selections. An assertion from O2,
another source set or another window cannot settle O1.

## Range-aware progress and closure

Every assertion retains its authority and independently scoped decision-scope
progress, decision-scope closure, surrounding-execution progress and
surrounding-execution closure where applicable. A result retains each required
axis with its exact scope, authority and boundary; omission or cross-wiring
refuses and cannot be repaired by completeness or completed assessment execution.
A late contradiction identifies the prior assertion/result and produces linked
supersession or invalidation under the corrected input identity without
rewriting historical bytes.

Result consumers also retain B's independent complete-global-conformance closure
record: global claims retain its state and exact required premises; other claims
retain `not-required` with no global premise set. A progress assertion cannot
manufacture that record or substitute another closure axis for it.

For an event-position range `[start,end)`, progress covers only the selected
integer positions and silence supplies no new position. For a fixed-sample range,
progress covers exact indexed samples under the selected epoch/period/unit,
including silent samples only when their required valuations are supplied. For a
timestamp range, a native timestamp tick is covered only when it lies in the
producer's half-open interval. A timestamped native inclusive endpoint equal to
the producer end is not covered or closed by that range. Start participates once;
end belongs only to a separately selected range that contains it.

Member identity does not substitute for record identity, and record identity
does not substitute for a position/sample/timestamp anchor. Missing required
coverage remains incomplete; wrong authority, family, endpoint, range, or
identity refuses. A decisive record at 29 may preserve its exact support while
unrelated coverage remains incomplete. Compatible surrounding-execution changes
cannot erase an independently settled truth or its exact support, and closing a
decision scope cannot supply the separate closure premises of complete global
conformance.

## Validity and restoration boundary

A valid assertion is an observation input, not a result. It establishes no truth,
activation, participation, completeness, causality or execution beyond the
selected E/B contracts. Restart or incremental continuation without the selected
predecessor/history and restoration basis remains incomplete or refused; it
cannot silently replace history or infer progress. Resource-limit and record-set
changes remain identified assessment inputs under the same selected meaning;
a changed definition selection requires a newly linked subject.
