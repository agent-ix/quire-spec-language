---
id: FR-075
title: "Compute lowering-target candidates from a registered-backend registry"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
---
# FR-075: Compute lowering-target candidates from a registered-backend registry

## Description

QSL SHALL replace the fixed `ProjectionTarget` catalog at
`src/lowering/target.rs` (the `targets!` macro block) with a registry that
backends populate by registering a `BackendDescriptor` under one contract.
QSL SHALL compute each requested item's candidate set from that registry's
current contents, matching on capability kind alone (ADR-012 §7.1, §7.2).
The registry SHALL define no local capability-kind type. The registry SHALL
consume the canonical `Capability` value type
([quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213)),
whose members are the ten labels FR-290 fixes and FR-057 admits into QSL.

## Inputs

- A `BackendDescriptor { id: BackendId, manifest_digest: Digest, advertises: set of (Capability, mode) }`
  per registration, where `mode` is `bounded` or `unbounded` (ADR-012 §7.1).
  `manifest_digest` is the digest of the backend's own FR-331 provider
  manifest content, computed under `quire.tool-manifest.jcs/v1`'s digest
  rule (ADR-012 §7.1, ADR-013 O-19); the registry receives it as part of
  the descriptor and does not compute it. A backend supplies one descriptor
  per registration, so one `BackendId` carries exactly one
  `manifest_digest` at a time; a second registration under the same
  `BackendId` with a different `manifest_digest` is a repeated identity and
  is refused under FR-075-AC-4, the same as a second registration with an
  unchanged digest.
- A requested item's capability kind (the `Capability` value FR-057's
  admission recorded for it) and, optionally, a named `BackendId`.
- The registry's current contents at the moment candidates are computed.

## Outputs

- A candidate set per item: zero, one or more `(BackendId, manifest digest)`
  pairs, ordered bytewise by identity and then by manifest digest
  (FR-290 "Candidate set and negotiation").
- A distinct unknown-backend marker, carrying the named identity, when the
  request names a `BackendId` the registry does not hold.
- A registration refusal, keyed by backend identity, when a registration
  repeats an already-registered `BackendId`.

## Behavior

### Candidate computation matches on capability kind alone

For each requested item, the registry SHALL compute the candidate set as
follows, matching a registration to an item only by capability kind and
never by mode, display name or registration order:

- When the request names a registered `BackendId`, the set SHALL be that
  backend when its `advertises` set contains the item's capability kind, and
  SHALL be empty otherwise.
- When the request names a `BackendId` the registry does not hold, the
  result SHALL be the unknown-backend marker carrying that identity, and
  SHALL NOT be reported as an empty candidate set.
- When the request names no backend, the set SHALL be every registered
  backend whose `advertises` set contains the item's capability kind.

Mode (`bounded`/`unbounded`) SHALL NOT affect which backends are candidates;
comparing a candidate's advertised mode against the item's extent is
`negotiate_*`'s work, after candidates are computed (ADR-012 §1.1, §7.2 step
3), and this requirement's registry SHALL NOT perform it.

### One capability type, no local duplicate

The registry SHALL import the capability-kind labels a backend advertises,
and the capability kind a requested item carries, from the one canonical
`Capability` type. The QSL source tree SHALL define no second type, in the
registry module or elsewhere in `#185`'s scope, whose variants name FR-290's
capability-kind labels.

### Registration is refused, not silently merged, on a repeated identity

If a registration names a `BackendId` the registry already holds, then the
registry SHALL refuse the new registration with
`invalid_capability`/`duplicate-backend`, naming the identity, and SHALL
leave the existing registration unchanged and in effect.

### The registry is an ordinary value, not ambient state

The registry SHALL be an ordinary value (for example a `BTreeMap` keyed by
`BackendId`) built by the orchestrating driver and passed as an argument to
every consumer (ADR-012 §7.1). No registry consumer SHALL read backend
registrations through a global, a `static`, a `OnceLock`, a `thread_local!`,
or a plugin-discovery mechanism such as `inventory`, `linkme` or `ctor`
(ADR-012 §5.3; FR-080 records the gates that enforce this).

### Registries agree regardless of build order

Two registries built from the same set of `BackendDescriptor` values, added
in any order, SHALL be equal, and SHALL compute the identical candidate set,
in the identical order, for every item. Candidate computation SHALL depend
only on the registry's contents and the requested item, never on the order
registrations were added.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-075-CON-1 | The registry module SHALL depend on no ambient global, thread-local, or link-time registration mechanism (`inventory`, `linkme`, `ctor`) | Design | Test (TC-205, TC-206) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-075-AC-1 | Given a registry holding one backend that advertises the requested item's capability kind, and no named backend in the request, the candidate set is exactly that backend. Given a registry holding a backend that does not advertise the item's kind, the candidate set is empty. | Test (TC-193) |
| FR-075-AC-2 | Given two registries built by adding the same set of `BackendDescriptor` values in two different orders, the two registries are equal, and computing candidate sets for the same set of items against each yields identical sets in identical order for every item. | Test (TC-194) |
| FR-075-AC-3 | Given a request naming a `BackendId` the registry does not hold, the result is the unknown-backend marker carrying that identity, distinguishable from an empty candidate set (which arises only from a registered backend that does not advertise the item's kind, or from no registrant advertising the kind at all). | Test (TC-195) |
| FR-075-AC-4 | Given a registration naming a `BackendId` already held by the registry, the registration is refused with `invalid_capability`/`duplicate-backend` naming the identity, and a subsequent candidate computation still reflects only the original registration's advertised kinds. | Test (TC-196) |
| FR-075-AC-5 | The registry module's public and internal capability-kind matching uses only the canonical `Capability` type; no enum defined inside `#185`'s scope carries variants named for an FR-290 capability-kind label. | Test (TC-193) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §7.1
  and §7.2 fix the registry's shape (an ordinary value, built once, passed as
  an argument) and the candidate-computation algorithm this requirement
  implements; §5.2 lists the explicit registry failure cases FR-075-AC-3 and
  FR-075-AC-4 verify.
- [FR-057](FR-057-admit-shared-capability-kinds.md) admits the ten FR-290
  capability-kind labels into QSL and states the same candidate-set rule from
  the composed-linker admission side; this requirement implements it at the
  registry.
- [FR-290](ix://agent-ix/quire-specification/FR-290) (quire-specification)
  is the normative source for the ten capability-kind labels, the
  `(kind, mode)` advertisement shape, and the candidate-set ordering rule
  this requirement's Outputs section restates.
- [quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185)'s
  issue body describes "the canonical six-kind `Capability` value type,"
  taking the `protocol` family's six members (FR-290's `Families` table, the
  set FR-060 dispatches independently) as the whole vocabulary. That phrase
  is stale: FR-290's "Vocabulary authority" section states verbatim that its
  ten labels, not six, "are the single capability-kind vocabulary of the
  Quire ecosystem," from which "`quire-spec-language`'s `Capability` type,
  the backend descriptors registered in its registry... take their
  members." FR-290 governs; this requirement and FR-057 specify against the
  ten-label vocabulary throughout, and #213's `Capability` type is expected
  to carry all ten.
- The canonical `Capability` value type and its outcome constructors are
  owned by [quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213);
  this requirement consumes that type and specifies no shape for it.
- [FR-080](FR-080-registry-evidence-and-gates.md) records the permutation
  property test (FR-075-AC-2, evidenced by TC-194) as part of the ADR-012
  §5.3 registry evidence obligations, and the `cargo-deny`/lint gates that
  enforce FR-075-CON-1.

## Status

Specified under
[quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185).
Not yet implemented. `src/lowering/target.rs` still declares the fixed
three-variant `targets!` catalog this requirement replaces. Implementation
waits on [quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213)
for the canonical `Capability` type.
