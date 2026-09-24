---
id: FR-057
title: "Admit exactly the shared FR-290 capability kinds"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-271
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-272
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-010
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-016
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: references
---
# FR-057: Admit exactly the shared FR-290 capability kinds

## Description

When a requested clause/capability pair reaches the QSL composed linker, the
linker SHALL admit its capability label only if the label is exactly one of the
ten capability kinds of vocabulary `quire.capability-kind/v1`, which
`ix://agent-ix/quire-specification/FR-290` fixes.

The QSL `Capability` type is language admission only (quire-specification
AD-016, arrow 1). It names which claim a declaration requests. It grants no
checking, lowering, proof or execution, and it negotiates nothing.

## Admitted vocabulary

Capability vocabulary `quire.capability-kind/v1` is the FR-290 Values table at
quire-specification revision `55d2fcc`
([quire-specification#135](https://github.com/agent-ix/quire-specification/pull/135)).
It admits these ten labels and no others. The labels, their meanings, their
families and their spelling are FR-290's. QSL defines no local alias,
abbreviation or extra member.

| Label | FR-290 family |
| --- | --- |
| `value-validity` | value |
| `operation-contract` | state-model |
| `finite-replay` | finite-replay |
| `temporal-satisfaction` | temporal-trace |
| `global-conformance` | protocol |
| `monitorability` | protocol |
| `local-projection` | protocol |
| `refinement` | protocol |
| `realizability` | protocol |
| `composition` | protocol |

## Inputs

- A requested clause/capability pair: a namespace-local declaration, one
  capability label and a `required` flag
  ([FR-036](FR-036-link-composed-native-packages.md)).
- For a serialized carrier of capability labels that QSL reads: its declared
  capability vocabulary identity.
- For a backend registration: the (kind, mode) pairs the backend advertises.

## Outputs

- For an admitted label: the one `Capability` value whose label is that label,
  retained in the requested pair unchanged.
- For a refused pair: a refusal with code `invalid_capability`, cause
  `absent-kind` or `unknown-kind`, the exact received label bytes when present,
  and the pair's request index. The refused pair stays at its request index in
  the request report.
- For a refused carrier: a refusal with code `invalid_capability`, cause
  `unsupported-version`, and the received identity or its absence. No request
  report is produced from a refused carrier.

## Behavior

### Diagnostic code

The QSL composed linker SHALL report every label refusal and carrier refusal
with code `invalid_capability` and exactly one cause from the closed set
`absent-kind`, `unknown-kind` and `unsupported-version`. Registration refusals
carry the registry causes below, and family-checker refusals carry their own
catalog codes.

`invalid_capability` and its causes are catalogued in
`quire.native.diagnostics/v1` revision `1-draft.5` (quire-specification FR-271
and FR-272). It is a refusal of the request, not an unsupported backend
result, so `unsupported_projection` and `unimplemented_capability` are never
produced at admission.

### Spelling, identity and order

The QSL composed linker SHALL match a capability label by exact equality of its
decoded UTF-8 bytes with one admitted label.

The QSL composed linker SHALL NOT normalize a received label before matching.

The QSL composed linker SHALL treat two `Capability` values as equal exactly
when their labels are byte-equal.

The ten kinds carry no order. Declaration order in the vocabulary table, in the
Rust type and in any serialization confers no strength, precedence or dispatch
priority. The QSL composed linker SHALL compare a set of kinds as a set.

The QSL composed linker SHALL keep the caller's request order. That order is
the request index identity FR-036 retains, not an order over kinds. Two pairs
with the same declaration and kind stay as two pairs at their own indices, each
with its own `required` flag; they are never merged.

Received label bytes count against the supplied-bytes limit of the FR-036
package accounting.

### Serialization and version

The QSL composed linker SHALL serialize a `Capability` value as its exact
FR-290 label as a JSON string.

When QSL emits a serialized carrier of capability labels, QSL SHALL declare
vocabulary identity `quire.capability-kind/v1` in it.

If a serialized carrier that QSL reads declares a vocabulary identity other
than `quire.capability-kind/v1`, or declares none, then QSL SHALL refuse the
carrier with `invalid_capability`/`unsupported-version`.

QSL SHALL NOT read any label of a carrier refused for its version.

The member that holds the identity on a cross-repository format belongs to
that format's owner (FR-290). On the backend-provider envelope it is
quire-specification FR-331's `capability_vocabulary`. The member on QSL's own
carriers is assigned under
[#211](https://github.com/agent-ix/quire-spec-language/issues/211).

### Refusal of labels outside the vocabulary

If a requested pair carries no capability label, or its label is JSON `null`,
then the QSL composed linker SHALL refuse the pair with
`invalid_capability`/`absent-kind`.

The QSL composed linker SHALL NOT default a missing label to any kind.

If a requested pair carries a label that is not byte-equal to an admitted
label, including the empty string or a non-string JSON value, then the QSL
composed linker SHALL refuse the pair with `invalid_capability`/`unknown-kind`,
naming the exact received bytes.

The QSL composed linker SHALL NOT map, alias or replace a refused label with an
admitted kind, and SHALL NOT attach a suggested replacement to the refusal.

If a refused pair is required, then the QSL composed linker SHALL report
complete admission unavailable. A refused pair that is not required does not
affect admission of the other pairs.

### Claim forms and their kinds

The QSL composed linker SHALL record for each requested item exactly one
kind: the kind the FR-290 claim-form assignment table gives its clause's own
claim form. The QSL composed linker SHALL record no kind for an expression
nested in a clause, such as a function application or a `case` expression:

| Claim form | Kind |
| --- | --- |
| Boolean clause over pure expressions, including function application (#217) | `value-validity` |
| Clause containing a `case` expression over a sum type | `value-validity` |
| `case` exhaustiveness obligation | none; language admission discharges it (quire-specification FR-146), and an unproved obligation refuses as `undefined_expression`/`unproved-exhaustiveness` |
| Operation precondition, postcondition or invariant | `operation-contract` |
| Frame obligation (`modifies`, `creates`, `deletes`) | `operation-contract` |
| Replay of one finite execution | `finite-replay` |
| Refinement between two operation contracts or state models (the relation-family refinement gates, [#191](https://github.com/agent-ix/quire-spec-language/issues/191) and [#192](https://github.com/agent-ix/quire-spec-language/issues/192)) | `operation-contract`, one claim per clause implication |
| Abstraction relation (quire-specification FR-353) | none; it is a premise of the `operation-contract` claim it binds |
| Temporal requirement under a finite-trace or infinite-trace profile | `temporal-satisfaction` |
| Refinement between two protocols | `refinement` |
| Other protocol analysis claim | its `protocol`-family kind |

Which family records which `Requirements` is decided in
[#210](https://github.com/agent-ix/quire-spec-language/issues/210); each
recorded requirement names one kind from this table.

### Kind applicability

A requested pair's kind applies to exactly one QSL declaration family:

| Kind | QSL declaration family |
| --- | --- |
| `value-validity` | predicate |
| `operation-contract` | state |
| `finite-replay` | protocol |
| `temporal-satisfaction` | temporal |
| `global-conformance`, `monitorability`, `local-projection`, `refinement`, `realizability`, `composition` | protocol |

QSL has four declaration families where FR-290 names five. FR-290's
`finite-replay` family applies to a QSL protocol declaration, because only a
choreography is replayed.

If a requested pair's kind does not apply to its declaration's family, then the
QSL composed linker SHALL record the pair as an inapplicable capability, naming
the declaration's family. A required inapplicable pair makes complete admission
unavailable. An inapplicable pair withholds no declaration body from its family
checker.

### Family-body admission

Checking a declaration's body under its semantic family is language admission,
not a capability kind. No capability request selects or withholds a body.

The QSL composed linker SHALL hand every declaration whose names resolved to
its family checker.

If the selected profile does not admit a declaration's family form, then the
family checker SHALL refuse it with `unsupported_construct`/`declaration-form`,
and the body is never represented as checked.

### Stage ownership

The QSL composed linker is the stage that declares a capability requirement. The
QSL composed linker SHALL retain every admitted requested pair, with its
declaration, kind and `required` flag, in the handoff FR-036 defines.

The QSL composed linker SHALL NOT read backend support, solver presence or
registration state when admitting a requested pair. Its admission entry point
takes no registry or backend parameter.

Each backend advertises (kind, mode) pairs by registering under one
registration contract (quire-specification AD-010 and FR-290). The registry,
the candidate sets it computes and the routing are implemented by
[#185](https://github.com/agent-ix/quire-spec-language/issues/185), which
consumes this type and defines no second capability vocabulary.

QSL reads no provider-manifest bytes. The driver that reads a backend's FR-331
provider manifest registers the backend with four values from it: the backend
identity, the manifest digest, the pinned tool identity and the advertised
(kind, mode) labels exactly as stated (ADR-013 C-28).

When a backend registers, the registry SHALL admit each advertised kind under
the same rules as a requested pair.

If a backend advertises an absent or unknown kind, or a mode other than
`bounded` or `unbounded`, or repeats a registered backend identity, then the
registry SHALL refuse that backend's registration with `invalid_capability`
(`absent-kind`, `unknown-kind`, `unknown-mode` or `duplicate-backend`), keyed by
backend identity. The refused registration contributes nothing, and any
registration already held under that identity stands.

For each admitted item, the registry SHALL compute the candidate set under the
FR-290 candidate-set rule from one immutable registry snapshot, and SHALL
return it as one typed value: the ordered candidate list, or the unknown-backend
mark carrying the named identity. That value is the item's `candidates` in the
quire-specification FR-331 request. The FR-331 envelope writer serializes it
under FR-331's member layout, which agent-ix/quire-specification#134 owns (ADR-013
QC-12, C-29); QSL defines no candidate-set wire of its own. A
candidate is a registered backend's (identity, manifest digest); candidates are
ordered bytewise by identity, then digest. The FR-331 `manifest` holds one
descriptor per backend in that snapshot. The snapshot is retained as assessment
provenance; a registration made during the request affects only later
requests. Each item also carries its extent and the extent classification
(`bounded` or `unbounded`) that
[#222](https://github.com/agent-ix/quire-spec-language/issues/222) defines.

Negotiation is not a QSL stage. quire-contract-codegen's `negotiate_*` is the
single negotiation point (quire-specification AD-016). It settles each item
from its candidate set and extent under FR-290, evaluating an absent or unknown
kind first, then an absent extent classification, then this table top row
first:

| Candidate set | Disposition |
| --- | --- |
| unknown-backend mark, or a registered backend with no negotiation arm | `invalid-request`, `invalid_capability`/`unknown-backend` |
| inconsistent with the manifest or the named backend, or containing a candidate that does not advertise the kind | `invalid-request`, `invalid_capability`/`inconsistent-candidates` |
| empty | `unsupported`, warned, naming the item's kind and any named backend; `unsupported_projection`/`unsupported-requested-capability` |
| more than one, and the request names none | `invalid-request`, `invalid_capability`/`ambiguous-backend`, naming every candidate in candidate order |
| exactly one | that candidate's arm: `supported`, `requires-bound`, `unsupported` (warned) or `invalid-request` |

An item with no extent classification settles `invalid-request` with
`invalid_capability`/`absent-extent`.

The routing SHALL take settled dispositions as input data, and SHALL route an
item to its one candidate only when the item is settled `supported`.

The routing SHALL NOT make any selection depend on registration order, display
text or ambient global state. There is no preference order among candidates.

### Absence, unsupported, refusal, timeout and hold

These cases are distinct. None of them is reported as another.

| Case | Where it is settled | Result |
| --- | --- | --- |
| Capability absence | QSL admission | Refusal `invalid_capability`/`absent-kind`. |
| Unknown kind | QSL admission | Refusal `invalid_capability`/`unknown-kind`. |
| Unsupported vocabulary version | QSL carrier read | Refusal `invalid_capability`/`unsupported-version`. |
| Backend absence: the candidate set is empty | Negotiation | `unsupported`, with one warning naming the item's kind and any named backend; `unsupported_projection`/`unsupported-requested-capability`. |
| Unsupported claim: the one candidate's arm does not discharge this IR form | Negotiation | `unsupported`, warned; `unsupported_projection`/`unsupported-requested-capability` (quire-specification FR-272). |
| Unbounded extent on a bounded-only candidate, finite bound available | Negotiation | `requires-bound`; the extent and bound predicate are [#222](https://github.com/agent-ix/quire-spec-language/issues/222)'s. |
| Unbounded extent on a bounded-only candidate, no finite bound | Negotiation | `unsupported`, warned; `unsupported_projection`/`unbounded-extent`. |
| Several candidates and no named backend, an unregistered or arm-less named backend, inconsistent candidates, or no extent classification | Negotiation | `invalid-request`, with its `invalid_capability` cause. |
| Solver or tool absence: the routed backend's adapter probe finds its pinned tool missing or mismatched, errors or exceeds its limit | Run of a `supported` item | FR-331 result `unsupported`, warned, naming the kind, backend and expected and actual tool identity; `unsupported_projection`/`tool-unavailable`. |
| Tool changed after a passing probe, before the run completes | Run of a `supported` item | FR-331 result `failed`, naming the expected and actual tool identity; `unsupported_projection`/`tool-unavailable`. The result `failed`, not the cause, distinguishes it from absence found at probe. |
| Timeout | Run of a `supported` item | A run result mapped by the IR outcome map, never a disposition. |
| Hold | Nowhere | No stage produces a hold. |

A hold is a stage suspending an item's settlement until an external event, such
as a backend registration. No stage in this requirement produces one.

The routing SHALL NOT convert an `unsupported` item into a refusal or a hold,
and SHALL NOT delay any other item's routing because of it.

The QSL composed linker SHALL NOT produce a refusal from backend state.

Routing takes settled dispositions and nothing else. A run result, including a
timeout or a tool-absence result, is not a routing input, so no run result
changes an item's disposition or route, and routing has nothing to re-route or
re-run under another mode. Recording a tool-absence `unsupported` result, or a
`failed` result for a tool that changes after a passing probe, with
`unsupported_projection`/`tool-unavailable`, belongs to the FR-331 result
producer (quire-specification FR-290-AC-8, FR-331-AC-6). Artifacts the item
emitted before the probe are retained and carry no verdict. The probe's
placement belongs to quire-contract-codegen.

An `unsupported`, `requires-bound` or `invalid-request` item emits no substitute
artifact. Complete aggregate success over settled dispositions is the FR-331
accounting record's result, joined on the request index (quire-specification
AD-016).

### One capability type

QSL SHALL have exactly one type that carries capability-kind labels: the
canonical `Capability` value type that
[#213](https://github.com/agent-ix/quire-spec-language/issues/213) implements
from this requirement.

Other QSL types named for capabilities carry no capability kind and admit no
FR-290 label. These include the complete-V1 feature identifier and the
checker's definition permissions. Their ownership is decided in #211.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-057-AC-1 | Each of the ten labels in this requirement's v1 table is admitted and round-trips to a byte-identical JSON string. The admitted set equals that table's ten values exactly, with no extra and no missing member. | Test (TC-153) |
| FR-057-AC-2 | A missing or `null` label refuses with `invalid_capability`/`absent-kind` and is never defaulted. Every other label that is not byte-equal to an admitted label refuses with `invalid_capability`/`unknown-kind` and names the received bytes. This includes `""`, a non-string value, a case variant (`Global-Conformance`), a separator variant (`value_validity`), a display form (`Operation contract`), a padded label (` composition`), a Rust variant name (`ValueValidity`) and each of `FamilyCheck`, `StateOperation`, `FiniteReplay` and `TemporalProjection`. No refusal carries a replacement kind. A refused pair stays at its index; a refused required pair makes complete admission unavailable. | Test (TC-153) |
| FR-057-AC-3 | A carrier QSL reads that declares `quire.capability-kind/v1` is read. A carrier that declares no identity, or any other identity string, refuses with `invalid_capability`/`unsupported-version`, naming the received identity or its absence, and none of its labels is read. | Test (TC-154) |
| FR-057-AC-4 | Kind equality is label equality. Permuting a set of kinds leaves set comparison unchanged. Permuting the requested pairs changes only request indices, and each pair keeps its own kind. Two pairs with the same declaration and kind both stay at their own indices. | Test (TC-153) |
| FR-057-AC-5 | The admission entry point takes no registry or backend parameter. Admitting the same requested pairs yields identical admitted pairs and static meaning whatever backends are registered. A declaration whose names resolved reaches its family checker without any capability request. | Test (TC-155) |
| FR-057-AC-6 | Given settled dispositions in which one item is `unsupported` for an empty candidate set and one is `supported`, routing routes only the `supported` item. The `unsupported` item gets no target and no artifact, and is not turned into a refusal or a hold. The other item routes without delay. | Test (TC-155) |
| FR-057-AC-7 | The QSL source tree defines one type carrying capability-kind labels, and no other type parses or emits an FR-290 label. | Test (TC-153) |
| FR-057-AC-8 | A backend registration advertising an absent or unknown kind or an unknown mode, or repeating a registered identity, is refused with `invalid_capability` and its cause, keyed by backend identity; the refused registration contributes nothing, and any registration already held under that identity stands. Candidate sets, their order, and the routing of `supported` items are identical under every registration order; two capable backends with no named backend yield two candidates, never a chosen one. | Test (TC-155) |
| FR-057-AC-10 | Each claim form in this requirement's claim-form table requests exactly its listed kind, one kind per item; a nested expression adds no kind; a `case` exhaustiveness obligation and an abstraction relation request none. | Test (TC-153) |
| FR-057-AC-11 | Each kind is applicable to exactly the family this requirement's applicability table gives it. A required `operation-contract` request on a state declaration is admitted; a `finite-replay` request on a state declaration is an inapplicable capability naming the state family, and its declaration's body still reaches its family checker. | Test (TC-115) |

## Dependencies

- quire-specification FR-290 (`ix://agent-ix/quire-specification/FR-290`) at
  revision `55d2fcc` owns the ten labels, their meanings and families, the
  claim-form assignment, (kind, mode) advertisement, the candidate-set rule and
  the tool-absence result. Its FR-290-AC-4 owns the backend-absence settlement
  at negotiation.
- quire-specification AD-010 and AD-016 fix the single registration contract,
  the single negotiation point, the four dispositions and the FR-331 accounting
  join. FR-331 carries the per-item `candidates`.
- quire-specification FR-271 and FR-272 at catalog revision `1-draft.5` own
  `invalid_capability` and the `unsupported_projection` causes. QSL's
  reference moves to that revision before #213 lands.
- [FR-036](FR-036-link-composed-native-packages.md) consumes the admitted pairs.
- #213 implements the canonical `Capability` value type, the admission rules
  and the removal of backend reading from `requests::report`. #185 implements
  registration, candidate sets and routing over that type. FR-057-AC-10 also
  needs function application (#217), the refinement gates (#191, #192) and the
  abstraction relation (quire-specification FR-353) in QSL.
  FR-057-AC-1 to AC-5, AC-7 and AC-10 have no #222 prerequisite; only `requires-bound` and the
  `unbounded-extent` case are #222's.
- Where this requirement abbreviates FR-290 (diagnostic payloads, report
  order, `inconsistent-candidates` coverage), FR-290 at `55d2fcc` governs.
- #210 decides which family records which requirements. #211 decides the QSL
  carrier member for the vocabulary identity and the ownership of the other QSL
  types named for capabilities.

## Status

Specified under
[#229](https://github.com/agent-ix/quire-spec-language/issues/229). The
canonical `Capability` value type and its total FR-290 wire conversion
(ADR-013 C-24) are implemented (QSL-173, `qsl-semantics/src/check/capability.rs`).
FR-057-AC-1, FR-057-AC-2 and FR-057-AC-4 are partial: each AC's value-type
portion (label round-trip, refusal-with-bytes and label equality on this
type) is passed under TC-153; each AC's admission portion (the composed
linker actually receiving and admitting a requested clause/capability pair)
is planned under #213.

Since QSL-46 (PR #305), `linking::composed::requests::Request` carries the
canonical `crate::check::Capability`, and no other capability-kind type exists
in `requests`. `requests::report` reads no backend; the layer-R `route`
registry computes candidate sets over the same type (FR-075). `admitted_bodies`
is every declaration whose names resolved, independent of capability requests,
as "Family-body admission" states. `report` marks a request inapplicable by
this requirement's applicability table (`requests::families`); FR-057-AC-11 is
backed by TC-115 (`tests/it/composed_admission_stages.rs`).

QSL-46 adds registration from advertised labels
(`qsl_route::BackendDescriptor::admit`, refusing `absent-kind`,
`unknown-kind` and `unknown-mode` keyed by backend identity) and the routing
step (`qsl_route::routing`), which takes settled dispositions as data and
gives a target only to a `supported` item. FR-057-AC-6 and FR-057-AC-8 are
backed by TC-155 steps 3 to 6 (`qsl-route/tests/it/routing.rs`).

FR-057-AC-3, FR-057-AC-5, FR-057-AC-7 and FR-057-AC-10 are not yet backed by
tests traced to them. Remaining work: #213 lands the carrier-version refusal
(AC-3) and composed-linker admission of a received clause/capability pair.
TC-154, the rest of TC-155, and the admission portion of TC-153 (including
AC-7 and AC-10), are planned under that ticket.
