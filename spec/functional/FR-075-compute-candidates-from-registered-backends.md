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

QSL SHALL provide a registry, the layer-R `route` module's `Registry`
(ADR-011 §6.1), that backends populate by registering a `BackendDescriptor`
under one contract. QSL SHALL compute each requested item's candidate set
from that registry's current contents, matching on capability kind alone
(ADR-012 §7.1, §7.2).

The registry sits alongside the fixed `ProjectionTarget` catalog at
`src/lowering/target.rs` (the `targets!` macro block), which remains the
lowering-target selector. Resolving a backend by `BackendId` in `route` is
this requirement's work (#185, QSL-46). The catalog is deleted with
`lowering` in ADR-011 §7.3 M-6a, once the skeleton spine is green (ADR-011
§6.2, the `lowering` row).
The registry SHALL define no local capability-kind type. The registry SHALL
consume the canonical `Capability` value type
([quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213)),
whose members are the ten labels FR-290 fixes and FR-057 admits into QSL.

## Inputs

- A `BackendDescriptor { id: BackendId, manifest_digest: Digest, tool: pinned tool identity, advertises: set of (Capability, mode) }`
  per registration, where `mode` is `bounded` or `unbounded` (ADR-012 §7.1).
  `manifest_digest` is the digest of the backend's own FR-331 provider
  manifest content, computed under `quire.tool-manifest.jcs/v1`'s digest
  rule (ADR-012 §7.1, ADR-013 O-19); the registry receives it as part of
  the descriptor and does not compute it. `tool` is the backend's pinned
  tool identity (ADR-012 §7.1); this requirement's registry carries it
  through registration and candidate output unread and uninterpreted — the
  probe that checks a routed backend's tool against this pin runs after
  routing, at run time (ADR-012 §7.4, FR-290 "Tool absence"), which is
  outside this requirement's scope. A backend supplies one descriptor per
  registration, so one `BackendId` carries at most one held `manifest_digest`
  and `tool` at a time. The registry's uniqueness key is `BackendId` alone
  (FR-290 "Candidate set and negotiation": "a backend identity is unique
  within a registry"), so a second registration under an already-held
  `BackendId` is judged against the held descriptor: when the two
  descriptors are equal (same identity, manifest digest, tool and advertised
  pairs), the repeat is one registration and is not refused (FR-075-AC-7);
  when they differ in any member, including the manifest digest alone, the
  identity conflicts (FR-290 "Candidate set and negotiation"), and every
  registration of it -- the one already held and the new one -- is refused,
  the held registration is withdrawn, and the identity is permanently
  unregistered (FR-075-AC-4). ADR-013 O-19's field-wise equality ("two
  backend identities are equal iff both fields are equal") states when two
  wire `backend{identity, manifest_digest}` members are the same value; it is
  not the registry's uniqueness key, which is `BackendId` alone, but it is
  exactly the pair FR-290's refusal keys by: (identity, manifest digest).
- A requested item's capability kind (the `Capability` value FR-057's
  admission recorded for it) and, optionally, a named `BackendId`.
- The registry's current contents at the moment candidates are computed.

## Outputs

- A candidate set per item: zero, one or more `(BackendId, manifest digest)`
  pairs, ordered bytewise by identity and then by manifest digest
  (FR-290 "Candidate set and negotiation").
- A distinct unknown-backend marker, carrying the named identity, when the
  request names a `BackendId` the registry does not hold -- including an
  identity whose registrations have conflicted (FR-075-AC-4).
- One registration refusal per distinct manifest digest, keyed by
  `(backend identity, manifest digest)`, when two or more admitted
  registrations of one `BackendId` are not all equal (FR-075-AC-4); every
  such refusal the registry currently holds is available from
  `Registry::refusals`, ordered bytewise by `(identity, manifest digest)`.

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

### Reading a `backend` member

A candidate's wire form is ADR-013 O-19's `backend` member: an `identity`
string and a `manifest_digest` digest record. When QSL reads one, it SHALL keep
the identity verbatim, with no normalization, and SHALL check the digest's
domain before its digest string (ADR-013 C-27).

If the digest names no domain, a label FR-201 does not define, or an FR-201
domain other than `quire.tool-manifest.jcs/v1`, then QSL SHALL refuse the
member for that domain, whatever the digest string holds. If the domain is
right and the digest string is not exactly 64 lowercase hexadecimal digits,
then QSL SHALL refuse the member for its digest.

Writing a candidate's identity, domain and digest string and reading them back
SHALL give the same candidate.

### A repeated identity is idempotent when identical, and conflicts otherwise

If a registration names a `BackendId` the registry already holds, the
registry SHALL compare the new descriptor with the held one:

- If the two are equal (same identity, manifest digest, tool and advertised
  pairs), the repeat SHALL be treated as one registration and SHALL NOT be
  refused (FR-075-AC-7).
- If the two differ in any member, the registry SHALL refuse every admitted
  registration of that identity -- the one already held and the new one --
  with `invalid_capability`/`duplicate-backend`, one refusal per distinct
  manifest digest keyed by `(identity, manifest digest)`, SHALL withdraw the
  held registration, and SHALL treat the identity as permanently
  unregistered: a later registration of it, even one identical to an earlier
  registration, is refused on arrival, and a request naming it receives the
  unknown-backend marker (FR-075-AC-4).

The registry snapshot SHALL be a function of the set of admitted
registrations it has received, independent of the order they arrived in:
two registries built from the same registrations in different orders SHALL
be equal, SHALL report the same registration refusals in the same order,
and SHALL compute identical candidate sets for every item (FR-075-AC-2,
FR-075-AC-4, FR-075-AC-7).

### The registry is an ordinary value, not ambient state

The registry SHALL be an ordinary value (for example a `BTreeMap` keyed by
`BackendId`) built by the orchestrating driver and passed as an argument to
every consumer (ADR-012 §7.1). No registry consumer SHALL read backend
registrations through a global, a `static`, a `OnceLock`, a `thread_local!`,
or a plugin-discovery mechanism such as `inventory`, `linkme` or `ctor`
(ADR-012 §5.3; FR-080 records the gates that enforce this).

### Registries agree regardless of build order

Two registries built from the same set of `BackendDescriptor` values, added
in any order, SHALL be equal, SHALL report the same registration refusals in
the same order, and SHALL compute the identical candidate set, in the
identical order, for every item. This holds whether or not the set contains
identical repeats or conflicting descriptors under one identity: candidate
computation, and the conflict/refusal state, SHALL depend only on the
registry's contents and the requested item, never on the order registrations
were added.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-075-CON-1 | The registry module SHALL depend on no ambient global, thread-local, or link-time registration mechanism (`inventory`, `linkme`, `ctor`) | Design | Test (TC-205, TC-206) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-075-AC-1 | Given a registry holding one backend that advertises the requested item's capability kind, and no named backend in the request, the candidate set is exactly that backend. Given a registry holding a backend that does not advertise the item's kind, the candidate set is empty. | Test (TC-193) |
| FR-075-AC-2 | Given two registries built by adding the same set of `BackendDescriptor` values in two different orders, the two registries are equal, they report the same registration refusals in the same order, and computing candidate sets for the same set of items against each yields identical sets in identical order for every item -- including a set that contains an identical repeat of one identity's descriptor and a conflicting pair under another. | Test (TC-194) |
| FR-075-AC-3 | Given a request naming a `BackendId` the registry does not hold, the result is the unknown-backend marker carrying that identity, distinguishable from an empty candidate set (which arises only from a registered backend that does not advertise the item's kind, or from no registrant advertising the kind at all). | Test (TC-195) |
| FR-075-AC-4 | Given a registration naming a `BackendId` already held by the registry with an unequal descriptor, every registration of that identity -- the one already held and the new one -- is refused with `invalid_capability`/`duplicate-backend`, one refusal per distinct manifest digest keyed by `(identity, manifest digest)`; the held registration is withdrawn, the identity is permanently unregistered (a later registration of it, even an identical one, is refused on arrival, and a request naming it receives the unknown-backend marker), and registrations of other identities are unaffected. | Test (TC-196, TC-447) |
| FR-075-AC-5 | The registry module's public and internal capability-kind matching uses only the canonical `Capability` type; no enum defined inside `#185`'s scope carries variants named for an FR-290 capability-kind label. | Test (TC-193) |
| FR-075-AC-6 | A `backend` member written from a candidate and read back equals it, and an identity with leading and trailing spaces is kept verbatim. An absent domain, an unknown domain label and `quire.source.bytes/v1` each refuse for the domain, and `quire.source.bytes/v1` still refuses for the domain with a digest string that is not hex; with the right domain, 64 uppercase hex digits and a 2-character string each refuse for the digest. | Test (TC-433) |
| FR-075-AC-7 | Given a registration naming a `BackendId` already held by the registry with an equal descriptor (same identity, manifest digest, tool and advertised pairs), the repeat is one registration and is not refused; the registry and its candidate sets are unchanged. | Test (TC-447, TC-448) |

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
  this requirement's Outputs section restates. Its "Candidate set and
  negotiation" section (FR-290-AC-9, FR-290-AC-10) is also the normative
  source for the idempotent-repeat and conflicting-identity rules
  FR-075-AC-7 and FR-075-AC-4 implement.
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
Implemented under QSL-46 (PR #305) over the canonical `Capability` type (QSL-173, `qsl-semantics/src/check/capability.rs`). Since QSL-184 (ADR-011
§7.3 X-9) it is the crate `qsl-route` (`qsl-route/src/lib.rs`).
`src/lowering/target.rs` keeps its fixed three-variant `targets!` catalog as
the lowering-target selector; it is deleted with `lowering` in ADR-011
§7.3 M-6a (Description).

By Acceptance Criterion:
- FR-075-AC-1: backed (`TC-193`):
  `candidate_set_matches_registered_backends_advertising_the_requested_kind`
  (`qsl-route/tests/it/route_registry.rs`).
- FR-075-AC-2: backed (`TC-194`):
  `every_permutation_of_three_descriptors_gives_an_equal_registry_and_identical_candidates`,
  `all_120_orderings_of_five_descriptors_agree` and
  `permutation_invariance_holds_with_repeats_and_conflicts_in_the_multiset`
  (`qsl-route/tests/it/route_registry.rs`).
- FR-075-AC-3: backed (`TC-195`):
  `unregistered_named_backend_yields_a_distinct_unknown_backend_marker`
  (`qsl-route/tests/it/route_registry.rs`).
- FR-075-AC-4: backed (`TC-196`, `TC-447`):
  `conflicting_backend_identity_registration_refuses_both_under_either_order`
  (`qsl-route/tests/it/route_registry.rs` and `qsl-route/src/lib.rs`),
  `tc_282_duplicate_backend_identity::db_03`, `db_04`, `db_05`, `db_06` and
  `db_08`, and `three_way_conflict_reports_one_refusal_per_distinct_digest`
  (`qsl-route/tests/it/route_registry.rs`), mirroring quire-specification
  TC-282's DB-01..DB-08 vectors under every registration order. `db_07`
  (a malformed advertised mode, never a `duplicate-backend` conflict) is
  cited under FR-057-AC-8 instead.
- FR-075-AC-5: backed for its matching half (`TC-193`):
  `advertises_kind_matches_only_the_exact_capability` (`qsl-route/src/lib.rs`)
  shows candidate matching goes through `Capability` and tells two kinds
  apart. Its second half, that no other enum in scope carries FR-290
  capability-kind variants, holds by inspection: `route` imports
  `qsl_semantics::check::Capability` and defines no capability-kind type.
- FR-075-AC-6: backed (`TC-433`):
  `backend_member_round_trips_through_its_wire_parts` and
  `backend_member_with_a_wrong_digest_domain_refuses_before_the_bytes`
  (`qsl-route/src/lib.rs`).
- FR-075-AC-7: backed (`TC-447`, `TC-448`):
  `identical_repeat_registration_is_not_refused`
  (`qsl-route/tests/it/route_registry.rs` and `qsl-route/src/lib.rs`), and
  `tc_282_duplicate_backend_identity::db_01_identical_repeat_holds_once_with_no_refusal`/`db_02_identical_repeats_plus_another_identity_hold_both`
  (`qsl-route/tests/it/route_registry.rs`).
