---
id: FR-052
title: "Publish canonical native temporal evaluation requests and results"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-051, type: depends_on }
  - { target: ix://agent-ix/quire-observation/FR-004, type: references }
  - { target: ix://agent-ix/quire-contract-ir/FR-026, type: supports }
  - { target: ix://agent-ix/tl-syntax/IF-009, type: implements }
  - { target: ix://agent-ix/tl-syntax/VO-009, type: implements }
---
# FR-052: Publish canonical native temporal evaluation requests and results

## Description

**RETIRED (M-6d), which deletes `protocol_artifact::native_temporal::{request,result}` and its only test, `tests/it/native_temporal_owner.rs`.**

When a downstream bridge supplies one finite native evaluation input for an
FR-051 checked temporal subject, the compiler SHALL publish canonical bounded
request and result contracts whose strict readers rederive the complete input
and native evaluation without accepting caller-authored truth.

The contracts are `quire.native-temporal-request/v1` and
`quire.native-temporal-result/v1`. They make the existing FR-043/044 evaluator
an immutable owner boundary; they do not change native temporal meaning.

## Public subsystem

The implementation SHALL organize the surface as
`protocol_artifact::native_temporal::{request,result}` over the existing
checked-handoff, temporal evaluator and canonical-resource foundations:

```text
request::produce(&ValidatedTemporalSubject, Input, Limits)
  -> Report<RequestDocument>
request::read(bytes, &ValidatedTemporalSubject, Limits)
  -> Report<ValidatedRequest>
result::evaluate(&ValidatedRequest, Relation, Limits)
  -> Report<ResultDocument>
result::read(bytes, &ValidatedRequest, Relation, Limits)
  -> Report<ValidatedResult>
```

Both modules publish immutable `CONTRACT` and `SCHEMA_BYTES`.
`ValidatedRequest` and `ValidatedResult` have no public constructors. `Relation`
is original, superseding or invalidating and accepts a complete validated direct
predecessor rather than a free-form predecessor reference.

## Canonical request

One request binds exactly one FR-051 subject, one semantic obligation instance,
one evaluation anchor and one bridge correspondence. It retains the selected
native definition/profile/clock, exact position population, admitted order
keys, explicit Boolean valuation for every reachable checked leaf at every
required position, activation/capture inputs, authoritative-origin state,
watermark, assessment execution, and the four independent decision-scope and
surrounding-execution progress/closure references and states. Completeness is a
fifth independent reference/state/fact population.

Each external reference is opaque owner evidence containing contract, schema
digest, canonical identity/digest, authority identity/revision/digest, scope and
population as applicable. This library validates its structural use and exact
binding but does not claim to authenticate Quire Observation bytes. Contract IR
performs that cross-owner comparison before accepting a correspondence.

Positions are sorted by semantic coordinate and admitted order, never insertion
order. Position identities are distinct from observation identities. Every
valuation is explicit; absence is incomplete and never false. Event-position
and exact fixed-sample clocks are supported by this v1 bridge contract.
Timestamped/dense clocks, mixed past/future graphs, weak previous, unbounded
intervals and non-origin past histories are typed unsupported or refused as
specified by FR-043 and the selected profile.

## Canonical result

The evaluator consumes only `ValidatedRequest`. It returns one formula-wide
result bound to the request, subject, instance, correspondence, effective limits
and exact input identity. The result independently retains activation,
assessment execution, `true`/`false`/`pending` or an absent truth with a typed
non-value, settlement, all four progress/closure references, completeness and
the exact ordered decision-support position identities. It never returns one
leaf valuation as the formula result.

Only FR-043 evaluation may supply truth and support. A producer input contains
no truth, settlement or support fields. The strict result reader re-evaluates
the exact validated request under the exact relation and requires byte equality;
it never trusts serialized output fields.

A correction creates a new positive revision and retains relation, exact direct
predecessor identity/digest and corrected request identity. It rejects a
self-edge, unchanged request, wrong subject/instance/correspondence, non-direct
predecessor, revision regression and a predecessor whose immutable bytes do not
match its digest. Prior bytes remain unchanged.

## Canonicalization and resources

Both identities, and each position identity, are the lowercase hex SHA-256
over the contract label's byte length as a big-endian `u64`, the label, and
the RFC 8785 encoding (ADR-013 §2, `quire-canonical`) of the document or
position with its identity field omitted and every integer written as its
decimal string, so full-range `i64`/`u64` coordinates, order keys,
watermarks and revisions keep exact identities. Request, result, source,
position, observation, correspondence and predecessor identities cannot
substitute for one another. Readers reject unknown, duplicate, missing or
out-of-order fields, trailing data, noncanonical JSON, invalid tagged shapes,
unknown labels/contracts, unsorted or duplicate populations, graph/reference
errors and every identity/digest mismatch.

Limits independently bound bytes, strings, formula nodes,
positions, valuations, captures, support, history span, evaluation steps,
lineage and total visited work. Work is charged before allocation/traversal.
Exact ceilings pass; one-over or allocation failure returns one typed resource
outcome with no partial request, result or Boolean. A retry begins with fresh
accounting. Each limit is caller-configurable with a published default, used as
given above or below that default; nesting depth is not a limit (ADR-030 D-1).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-1 | **RETIRED (M-6d), which deletes `protocol_artifact::native_temporal::{request,result}` and `tests/it/native_temporal_owner.rs`.** Future and pure-past request documents deterministically bound one FR-051 subject/instance/correspondence to every position, explicit leaf valuation, capture, origin, clock, four progress/closure axes and independent completeness fact, and strict-read to constructor-private views. This criterion was satisfied while the module existed. With the spine `ProtocolClause` frame slices landed, the spine `ProtocolClause` path no longer needs this producer/consumer round trip, and nothing outside the deleted module's own tests called it. The criterion's claim was true when made and is now permanently unfalsifiable rather than false; it is retired, not amended, since there is no successor module for it to describe. | Retired |
| FR-052-AC-2 | **RETIRED (M-6d)**, for the same reason as FR-052-AC-1: result production obtained truth, non-value, settlement and exact support only by evaluating `ValidatedRequest`; a caller could not submit those fields, a leaf Boolean could not stand in for the formula result, and strict reading re-evaluated before admitting bytes. | Retired |
| FR-052-AC-3 | **RETIRED (M-6d)**, for the same reason as FR-052-AC-1: native future and O/H/Y/S/T discriminator vectors preserved FR-043 meaning, inclusive bounds, strong previous, origin versus cutoff, fixed-sample/event-position clocks, pending and every typed non-value without a Boolean fallback. | Retired |
| FR-052-AC-4 | **RETIRED (M-6d)**, for the same reason as FR-052-AC-1: request and result identities changed for every applicable semantic, contract, source, observation, limit or relation mutation, remained stable under declared set-order invariance, and rejected cross-domain identity substitution. | Retired |
| FR-052-AC-5 | **RETIRED (M-6d)**, for the same reason as FR-052-AC-1: four progress/closure axes, completeness, activation, execution, truth, settlement and support remained independently readable; impossible combinations and foreign/cross-wired owner references refused without partial output. | Retired |
| FR-052-AC-6 | **RETIRED (M-6d)**, for the same reason as FR-052-AC-1: original and corrected results preserved immutable bytes and exact direct predecessors; self, wrong-subject/instance/correspondence, unchanged-input, same/lower-revision and non-direct substitutions refused. | Retired |
| FR-052-AC-7 | **RETIRED (M-6d)**, for the same reason as FR-052-AC-1: unknown/duplicate/missing/reordered fields, trailing/noncanonical bytes, missing/duplicate/swapped/foreign valuations or positions, invalid order/history and exact-limit-plus-one inputs returned one deterministic typed failure with no partial view or Boolean. | Retired |
| FR-052-AC-8 | **RETIRED (M-6d)**, for the same reason as FR-052-AC-1: the public surface imported no TL or Contract-IR vocabulary, accepted no parser/evaluator callback/plugin/network/trust flag, and exposed complete borrowed accessors needed for structural downstream comparison without JSON parsing. | Retired |

## Dependencies

FR-043/044 own native evaluation and activation semantics. FR-051 supplies the
constructor-private checked temporal subject and its admitted v2 package.
Quire Observation remains the authority for external observation references;
Contract IR constructs and cross-checks the owner request and joins the result.
