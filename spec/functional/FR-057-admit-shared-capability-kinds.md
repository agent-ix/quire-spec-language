---
id: FR-057
title: "Admit exactly the shared FR-290 capability kinds"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: specifies
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-010
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-016
    type: depends_on
---
# FR-057: Admit exactly the shared FR-290 capability kinds

## Description

When a requested capability reaches the compiler, the compiler SHALL admit it
only if its label is exactly one of the six capability kinds that
`ix://agent-ix/quire-specification/FR-290` fixes, under capability vocabulary
version `quire.capability-kind/v1`.

The QSL `Capability` type is language admission only (quire-specification
AD-016, arrow 1). It names which claim a declaration requests. It grants no
checking, lowering, proof or execution, and it negotiates nothing.

## Admitted vocabulary

Capability vocabulary version `quire.capability-kind/v1` admits these six
labels and no others. The labels, their meanings and their spelling are
FR-290's. QSL defines no local alias, abbreviation or extra member.

| Label | Meaning (FR-290) |
| --- | --- |
| `global-conformance` | Finite global protocol conformance under the exact selected closure premises. |
| `monitorability` | Whether the selected admitted fragment can be monitored under the declared observations. |
| `local-projection` | Whether one selected participant projection is defined under its exact premises. |
| `refinement` | Whether one exact protocol/contract refines another under a selected relation. |
| `realizability` | Whether the selected protocol admits a causal implementation strategy. |
| `composition` | Whether selected components compose under independently discharged assumptions. |

## Inputs

- A requested pair: a namespace-local declaration, one capability label and a
  `required` flag ([FR-036](FR-036-link-composed-native-packages.md)).
- For a serialized carrier of capability labels: its declared capability
  vocabulary version.

## Outputs

- For an admitted label: the one `Capability` value whose canonical label is
  that label, retained in the requested pair unchanged.
- For a refused label or version: a refusal with code `invalid_capability`,
  one cause from `absent-kind`, `unknown-kind` or `unsupported-version`, the
  exact received label or version bytes, and the request index. A refused
  request enters no later stage.

## Behavior

### Spelling, identity and order

The compiler SHALL match a capability label by exact byte equality with one
admitted label.

The compiler SHALL NOT normalize a received label before matching. Case
folding, trimming, separator substitution, Unicode normalization and
display-form conversion are not applied.

A `Capability` value's identity is its label. Two values are equal exactly when
their labels are byte-equal.

The six kinds carry no order. Declaration order in the vocabulary table, in the
Rust type and in any serialization confers no strength, precedence or dispatch
priority. A set of kinds compares as a set.

A requested-pair inventory keeps the caller's request order. That order is the
request index identity FR-036 retains, not an order over kinds.

### Serialization and version

The canonical serialized form of a `Capability` value SHALL be its exact FR-290
label as a JSON string.

A Rust variant name, debug form or display string is not a serialized form.

Any serialized artifact that carries capability labels SHALL declare capability
vocabulary version `quire.capability-kind/v1`.

If a serialized carrier declares a capability vocabulary version other than
`quire.capability-kind/v1`, or declares none, then the compiler SHALL refuse
the carrier with `invalid_capability`/`unsupported-version`, naming the
received version or its absence, and SHALL read none of its labels.

### Refusal of labels outside the vocabulary

If a requested pair carries no capability label, then the compiler SHALL refuse
it with `invalid_capability`/`absent-kind`.

A missing label is never defaulted to `global-conformance` or to any other kind.

If a requested pair carries a label that is not byte-equal to an admitted
label, then the compiler SHALL refuse it with
`invalid_capability`/`unknown-kind`, naming the exact received label.

A refused label is never mapped, aliased or replaced by an admitted kind. The
refusal carries no suggested replacement.

### Stage ownership

The QSL composed linker is the stage that declares a capability requirement. It
SHALL retain every admitted requested pair, with its declaration, kind and
`required` flag, in the handoff FR-036 defines.

The QSL composed linker SHALL NOT consult backend support when admitting a
requested pair.

Backend support, solver presence and registration state do not change whether
a label is admitted, and they do not change static meaning.

Each backend advertises the kinds it can discharge by registering under one
registration contract (quire-specification AD-010). The registry and the
routing that selects a target backend are implemented by
[#185](https://github.com/agent-ix/quire-spec-language/issues/185), which
consumes this type and defines no second capability vocabulary.

Negotiation happens at one point, quire-contract-codegen's `negotiate_*`
(quire-specification AD-016). It settles each admitted requested item as
exactly one of `supported`, `requires-bound`, `unsupported` or
`invalid-request`, over the kinds registered backends advertise.

The routing implemented by #185 selects a target backend only for an item that
negotiation settled `supported`. Target selection does not depend on
registration order, display text or ambient global state.

### Absence, unsupported, refusal, timeout and hold

These cases are distinct. None of them is reported as another.

| Case | Where it is settled | Result |
| --- | --- | --- |
| Capability absence | QSL admission | Refusal `invalid_capability`/`absent-kind`. |
| Unknown kind | QSL admission | Refusal `invalid_capability`/`unknown-kind`. |
| Unsupported vocabulary version | QSL admission | Refusal `invalid_capability`/`unsupported-version`. |
| Solver or backend absence: no registered backend advertises the requested kind | Negotiation | `unsupported`, with a warning naming the requested kind's exact label. |
| Unsupported claim: a registered backend advertises the kind but cannot discharge this item's construct | Negotiation | `unsupported`, with the backend's catalog cause. |
| Unbounded item that a bound makes dischargeable | Negotiation | `requires-bound`; the boundedness design is [#222](https://github.com/agent-ix/quire-spec-language/issues/222)'s. |
| Malformed request for its subject | Negotiation | `invalid-request`. |
| Timeout | Run of a `supported` item | A run result, never a negotiation disposition. |

When no registered backend advertises a requested kind, negotiation SHALL
settle that item `unsupported` with a warning naming the kind's exact label.

Solver or backend absence is never a refusal and never a hold. No stage waits
for a backend to register before settling an item, and absence of one item's
backend does not delay any other item's settlement.

A refusal is a language-admission result only. No backend state produces one.

Timeout is a result of running a `supported` item. It never becomes
`unsupported`, a refusal or a hold. Its FR-331 result is set by the IR-owned
outcome map (quire-specification AD-016), through the structured outcome
constructors of [#213](https://github.com/agent-ix/quire-spec-language/issues/213).

An `unsupported`, `requires-bound` or `invalid-request` item emits no substitute
artifact. If such an item is required, then complete aggregate success is
unavailable (FR-036).

### One capability type

QSL SHALL have exactly one type that carries capability-kind labels: the
canonical `Capability` value type that #213 implements from this requirement.

Other QSL types named for capabilities carry no capability kind and admit no
FR-290 label. These include the complete-V1 feature identifier and the
checker's definition permissions. Their ownership is decided in
[#211](https://github.com/agent-ix/quire-spec-language/issues/211).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-057-AC-1 | Each of the six FR-290 labels is admitted and round-trips to a byte-identical JSON string. The set of admitted labels equals FR-290's six values exactly, with no extra and no missing member. | Test (TC-153) |
| FR-057-AC-2 | An absent label refuses with `invalid_capability`/`absent-kind` and is never defaulted. Each unknown label refuses with `invalid_capability`/`unknown-kind` and names the received bytes, including a case variant (`Global-Conformance`), a separator variant (`global_conformance`), a display form (`Global conformance`), a padded label (` composition`), a Rust variant name (`GlobalConformance`) and each of `FamilyCheck`, `StateOperation`, `FiniteReplay` and `TemporalProjection`. No refusal carries a replacement kind. | Test (TC-153) |
| FR-057-AC-3 | A carrier that declares `quire.capability-kind/v1` is read. A carrier that declares no version, or any other version string, refuses with `invalid_capability`/`unsupported-version`, naming the received version or its absence, and none of its labels is read. | Test (TC-154) |
| FR-057-AC-4 | Kind equality is label equality. Permuting a set of kinds leaves set comparison unchanged. Permuting the requested pairs changes only request indices, and each pair keeps its own kind. | Test (TC-153) |
| FR-057-AC-5 | Admitting the same requested pairs with no registered backend, with one backend and with several backends yields identical admitted pairs and identical static meaning. The linker reads no backend state. | Test (TC-155) |
| FR-057-AC-6 | A requested kind that no registered backend advertises settles `unsupported` with a warning naming the kind's exact label. It is not a refusal and not a hold. Another item in the same request, whose kind a registered backend does advertise, settles independently. | Test (TC-155) |
| FR-057-AC-7 | The QSL source tree defines one type carrying capability-kind labels, and no other type parses or emits an FR-290 label. | Inspection |

## Dependencies

- quire-specification FR-290 (`ix://agent-ix/quire-specification/FR-290`)
  owns the six labels and their meanings. Adding, removing or renaming a kind
  is an FR-290 change followed by a new capability vocabulary version here.
- quire-specification AD-010 and AD-016 fix the single registration contract,
  the single negotiation point and the four dispositions.
- [FR-036](FR-036-link-composed-native-packages.md) retains the requested pairs
  and the aggregate rule.
- #213 implements the canonical `Capability` value type and the shared outcome
  constructors. #185 implements the registry and routing over that type. #222
  owns boundedness and `requires-bound`.
- The per-family applicability of each kind, and which repository runs the
  negotiation over the registry, are decided in
  [#210](https://github.com/agent-ix/quire-spec-language/issues/210).
  Serialized-carrier ownership is decided in #211.

## Status

Specified under
[#229](https://github.com/agent-ix/quire-spec-language/issues/229). The
admitted vocabulary, the refusal codes and the stage split are not yet
implemented. `linking::composed::requests::Capability` still declares a
different four-member request vocabulary, and `requests::report` still reads a
caller-declared backend's support. #213 replaces that type with the canonical
`Capability`. #185 moves backend support out of the linker into registration
and routing. TC-153, TC-154 and TC-155 are planned under those tickets.
