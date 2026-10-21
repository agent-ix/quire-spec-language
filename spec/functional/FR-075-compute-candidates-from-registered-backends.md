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
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
---
# FR-075: Compute lowering-target candidates from a registered-backend registry

## Description

QSL SHALL provide a registry, the layer-R `route` module's `Registry`
(ADR-011 §6.1), that backends populate by registering a `BackendDescriptor`
under one contract. QSL SHALL compute each requested item's candidate set
from that registry's current contents, matching on capability kind and, for
`probabilistic-satisfaction`, the item's exact evidence kind (ADR-012 §7.1,
§7.2; FR-290, "Evidence kinds").

Resolving a backend by `BackendId` in `route` is this requirement's work
(#185). The native `lowering` module and its fixed `ProjectionTarget` catalog
are deleted.
The registry SHALL define no local capability-kind type. The registry SHALL
consume the canonical `Capability` value type
([quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213)),
whose members are the twelve labels FR-290 fixes and FR-057 admits into QSL.

## Inputs

- A `BackendDescriptor` per registration, with backend identity, provider
  origin, advertised `(Capability, mode)` pairs, retained
  `domains` (present or absent), and the admitted `evidence_kinds` set
  supplied when it advertises `probabilistic-satisfaction`. `mode` is
  `bounded` or `unbounded` (ADR-012 §7.1). `BackendId` is the backend
  identity alone (ADR-013 O-19); origin, pairs, domains and evidence kinds
  are descriptor members, not identity components (FR-288-AC-5/6). Any
  manifest digest carried for refusal reporting is not a descriptor-equality
  input.
  The registry's
  uniqueness key is `BackendId` (FR-290 "Candidate set and negotiation": "a
  backend identity is unique within a registry"), so a second registration
  under an already-held `BackendId` is judged against the held descriptor:
  when the equality-compared members are equal, the repeat is one
  registration and is not refused
  (FR-075-AC-7);
  when they differ in any equality-compared member, the identity conflicts
  (FR-290 "Candidate set and negotiation"): the registry refuses the
  identity once, withdraws the held registration, and the identity is
  permanently unregistered (FR-075-AC-4).
- A requested item's capability kind (the `Capability` value FR-057's
  admission recorded for it), its evidence kind when its capability is
  `probabilistic-satisfaction`, and, optionally, a named `BackendId`.
- For the request builder: a checked package's requirement records
  (FR-062-AC-13) and, optionally, one named `BackendId` for the request.
- The registry's current contents at the moment candidates are computed.

## Outputs

- From the request builder: one `RequirementItem` per requirement record,
  in record key order, each with its request index, occurrence key, node,
  kind, extent classification, unbounded domains, result bound, direct
  conversion context and candidate outcome.
- A candidate set per item: zero, one or more `BackendId`s, ordered
  bytewise by identity (FR-290 "Candidate set and negotiation").
- A distinct unknown-backend marker, carrying the named identity, when the
  request names a `BackendId` the registry does not hold -- including an
  identity whose registrations have conflicted (FR-075-AC-4).
- One registration refusal per backend identity whose admitted
  registrations are not all equal, keyed by that identity (FR-075-AC-4);
  every such refusal the registry currently holds is available from
  `Registry::refusals`, ordered bytewise by identity.

## Behavior

### Candidate computation matches probabilistic evidence exactly

For each requested item, the registry SHALL compute the candidate set as
follows, matching a registration to the item's capability kind and, when that
kind is `probabilistic-satisfaction`, the item's evidence kind. A probabilistic
registration is a candidate only when its admitted evidence-kind set contains
the item's exact evidence-kind label. `statistical` SHALL NOT substitute for
`exact`, nor `exact` for `statistical`. If a probabilistic item has no evidence
kind or a label other than `statistical` or `exact`, this stage SHALL produce
no candidates and SHALL preserve the item evidence value for the downstream
negotiation stage to refuse as `absent-evidence-kind` or
`unknown-evidence-kind` (FR-290, "Evidence kinds"); it SHALL NOT convert that
invalid request into a supported candidate.

For non-probabilistic items, matching remains by capability kind alone. In
both cases, candidate matching SHALL NOT use mode, display name or registration
order:

- When the request names a registered `BackendId`, the set SHALL contain that
  backend only when its `advertises` set contains the matching capability and,
  for a probabilistic item, the exact evidence kind; otherwise the set is empty.
- When the request names a `BackendId` the registry does not hold, the
  result SHALL be the unknown-backend marker carrying that identity, and
  SHALL NOT be reported as an empty candidate set.
- When the request names no backend, the set SHALL be every registered
  backend whose `advertises` set contains the matching capability and, for a
  probabilistic item, the exact evidence kind.

Mode (`bounded`/`unbounded`) SHALL NOT affect which backends are candidates;
comparing a candidate's advertised mode against the item's extent is
`negotiate_*`'s work, after candidates are computed (ADR-012 §1.1, §7.2 step
3), and this requirement's registry SHALL NOT perform it.

### Requested items from a package's requirement records

`route`'s request builder (design name
`qsl_route::request::items_from_requirements`; the implementing ticket
chooses the Rust spelling) SHALL take a checked package's requirement
records (`CheckedPackage::graph().requirements()`, FR-062-AC-13), the
registry, and at most one `BackendId` the request names. It SHALL return
one `RequirementItem` (design name) per record, in the records' bytewise
occurrence-key order. Each `RequirementItem` SHALL carry:

- its request index, its position in that order (ADR-012 §13.5);
- the record's occurrence key, and that key's node (`WireNodeId`);
- the record's capability kind;
- its extent classification, as `RequestWriter::item` computes it from the
  record's `ClaimExtent` (FR-097);
- its unbounded domains: the `DomainKey` and kind of each domain of an
  `Unbounded` extent, in key order, and none for a `Bounded` one;
- the record's result bound, as the `WireNodeId` of its FR-092 type node;
- its direct conversion context, as `Option<OccurrenceKey>`: the canonical
  `OccurrenceKey` of the `expression` occurrence for the narrow that
  directly wraps this record's scalar application, or `None` when the
  record's result bound is the application's own result type;
- the record's path condition, as the same ordered sequence of canonical
  `PathGuard` values returned by `RequirementRecord::path_condition()`;
- its candidate outcome, `Registry::candidates(kind, evidence kind when
  probabilistic, named backend)` by this requirement's candidate rules.

An item whose candidate set is empty or whose extent is unbounded is
written like any other and settles at negotiation (`unsupported` or
`requires-bound`). Two records at one node (two occurrences of one
application) are two items with their own request indices and occurrence
keys.

An item's request index is its position in the package order. FR-395's call
association names an item by the package's `package_id` and this request
index (FR-395, "Pairing key"), and the item written at that index carries the member's key.

The path condition is the record's guard sequence, outermost first. Each
guard is the canonical `PathGuard` value: the guard expression's
`OccurrenceKey` and the required Boolean outcome (`holds`).
`RequirementItem::path_condition()` SHALL expose that sequence as
`&[PathGuard]`, preserving its order and members without translating it,
deduplicating it, or deriving a second guard representation. An unguarded
requirement record yields an empty sequence.

`RequirementItem::conversion_context()` SHALL expose the record's direct
conversion context as `Option<&OccurrenceKey>`. This value is carried once
per requirement occurrence, independently of the `WireNodeId` shared by the
scalar application. For each item, the driver SHALL associate the context
with the item by its occurrence key/request index; it SHALL NOT select a
conversion by expression node alone. A record whose result bound comes from
the application's own result type yields `None`. A record whose result bound
comes from a narrow carries exactly the direct (innermost) narrow's
occurrence key; no outer conversion replaces it. The narrow is context only
and does not add a requested item, routed item or claim. The driver's mapping
passes that occurrence's conversion context to IR together with the item's
root. A bare `lower([add])` call remains governed by the Contract IR API and
this requirement does not make QSL discover a reverse narrow for it.

The orchestrating driver (T-13) reads nothing else about items from QSL. It
writes each item's kind, extent classification, named backend and
candidates into the FR-331 request, and joins CG's settlements to items by
request index. For a `requires-bound` item, it answers with a bounded
follow-up, a new FR-331 request (ADR-014 §4 "Bounded request"): it
supplies one `FiniteBound` for each of the item's unbounded `DomainKey`s,
and a fresh `RequestWriter` writes the follow-up as that request's single
item, index 0. `RequestWriter::bounded_item` SHALL take the originating
occurrence key as a parameter (design-level signature
`bounded_item(occurrence: OccurrenceKey, requirements: &Requirements,
bounds: BTreeMap<DomainKey, FiniteBound>) -> Result<RequestIndex,
BoundRefusal>`), and the item it writes SHALL carry that occurrence key,
with the occurrence key's node as its node. The driver writes at most one bounded follow-up per
record, so each record has at most one follow-up settlement, and joins it
to the record by occurrence key. Writing several bounded items for one
domain set with different bounds (FR-097-AC-3) stays a capability of the
writer; the driver uses one and writes no second follow-up for a record.
That driver rule is tested in agent-ix/quire-driver. It
passes the `supported` items to `routing::route`, and hands CG generation
each routed item's request index, occurrence key, node and result bound,
with the package's requirement records keyed by occurrence key (ADR-011
E7), resolving the node in the admitted v2 package.

### One capability type, no local duplicate

The registry SHALL import the capability-kind labels a backend advertises,
and the capability kind a requested item carries, from the one canonical
`Capability` type. The QSL source tree SHALL define no second type, in the
registry module or elsewhere in `#185`'s scope, whose variants name FR-290's
capability-kind labels.

### Probabilistic evidence kinds are checked at descriptor admission

When a `BackendDescriptor` advertises `probabilistic-satisfaction`,
`BackendDescriptor::admit` SHALL require a nonempty evidence-kind set whose
members are each byte-equal to `statistical` or `exact` (FR-290, "Evidence
kinds"). If the set is absent or empty, or any member is not one of those two
labels, `BackendDescriptor::admit` SHALL refuse the registration with
`invalid_capability`/`unknown-evidence-kind`, keyed by the descriptor's
`BackendId`. This check SHALL finish before the descriptor is inserted into the
registry or used for candidate computation. The refused descriptor contributes
no registration, and any registration already held under that identity stands.

A descriptor that advertises no `probabilistic-satisfaction` kind SHALL NOT
require evidence-kind labels for admission. QSL SHALL preserve the accepted
labels on the admitted descriptor for the registered backend contract; this
requirement assigns no new evidence-kind vocabulary or alias.

### Reading a `backend` member

A candidate's wire form is ADR-013 O-19's `backend` member, the backend
identity string alone. When QSL reads one, it SHALL keep the identity
verbatim, with no normalization (ADR-013 C-27). If the member is empty, then
QSL SHALL refuse it.

Writing a candidate's identity and reading it back SHALL give the same
candidate.

### A repeated identity is idempotent when identical, and conflicts otherwise

If a registration names a `BackendId` the registry already holds, the
registry SHALL compare identity, provider origin, advertised pairs, retained
`domains`, and the evidence-kind set. A manifest digest is not part of this
equality comparison. A different identity is a different registry key and is
not a duplicate-registration conflict:

- If the two are equal in every equality-compared member, the
  repeat SHALL be treated as one registration and SHALL NOT be refused
  (FR-075-AC-7).
- If the two differ in any equality-compared member, the registry SHALL refuse the identity
  once with `invalid_capability`/`duplicate-backend`, keyed by the identity,
  SHALL withdraw the held registration, and SHALL treat the identity as permanently
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
| FR-075-AC-1 | For a non-probabilistic item, given a registry holding one backend that advertises the item's capability kind and no named backend, the candidate set is exactly that backend; a backend not advertising the kind is excluded. For `probabilistic-satisfaction`, a backend is a candidate only when it advertises both the kind and the item's exact evidence kind; neither `statistical` nor `exact` substitutes for the other. | Test (TC-193) |
| FR-075-AC-2 | Given two registries built by adding the same set of `BackendDescriptor` values in two different orders, the two registries are equal, they report the same registration refusals in the same order, and computing candidate sets for the same set of items against each yields identical sets in identical order for every item -- including probabilistic items and a set that contains an identical repeat of one identity's complete descriptor and a conflicting pair under another. | Test (TC-194) |
| FR-075-AC-3 | Given a request naming a `BackendId` the registry does not hold, the result is the unknown-backend marker carrying that identity, distinguishable from an empty candidate set (which arises when a registered backend does not advertise the item's kind or exact probabilistic evidence kind, or no registrant advertises that required combination). | Test (TC-195) |
| FR-075-AC-4 | Given a registration naming a `BackendId` already held by the registry with an unequal descriptor, the identity is refused once with `invalid_capability`/`duplicate-backend`, keyed by the identity; the held registration is withdrawn, the identity is permanently unregistered (a later registration of it, even an identical one, is refused on arrival, and a request naming it receives the unknown-backend marker), and registrations of other identities are unaffected. | Test (TC-196, TC-447) |
| FR-075-AC-5 | The registry module's public and internal capability-kind matching uses only the canonical `Capability` type; no enum defined inside `#185`'s scope carries variants named for an FR-290 capability-kind label. | Test (TC-193) |
| FR-075-AC-6 | A `backend` member written from a candidate and read back equals it, an identity with leading and trailing spaces is kept verbatim, and an empty identity refuses. | Test (TC-433) |
| FR-075-AC-7 | Given a registration naming a `BackendId` already held by the registry with equal identity, provider origin, advertised pairs, retained `domains`, and evidence-kind set, the repeat is one registration and is not refused; the registry and its candidate sets are unchanged. A repeat differing only in any of those compared members is unequal and follows FR-075-AC-4; manifest digest does not affect descriptor equality. A different identity is a different registry key. | Test (TC-447, TC-448, TC-982) |
| FR-075-AC-8 | Given the checked package of FR-062's RR-5 (`function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }`) and a registry holding one backend advertising `value-validity` in `bounded` mode, and no named backend, the request builder returns exactly one `RequirementItem` per requirement record, in bytewise occurrence-key order with request indices 0, 1, 2: the two `+` records are two items with the same node and distinct occurrence keys, and every item is `value-validity`, classified `bounded`, with no unbounded domain, its record's result bound, and that one backend as its candidate set. With RR-6 (`n + 1` over `n: Integer`), the one item is classified `unbounded` with `finite_bound_available` true and carries one unbounded domain, `Integer` at `n`'s parameter node, and a bounded follow-up that supplies an `IntegerRange` for that key is a new request whose single item has request index 0, is classified `bounded` and carries the occurrence key passed to `bounded_item`, the original item's, and the follow-up request holds exactly one item for that occurrence key. With RR-16, the two `+` items carry result bounds `Int[0, 10]` and `Int[0, 20]`. With a registry advertising no `value-validity`, every item is still returned, each with an empty candidate set. With a named unregistered backend, every item carries the unknown-backend marker. A package with no records gives no items. | Test (TC-449) |
| FR-075-AC-9 | For the checked function `function f using v(x: Int[0, 9], p: Boolean): Integer pure { if p then x + 1 else x + 1 }`, the two `x + 1` requirement records have the same application node and distinct occurrence keys, and the builder returns two items in occurrence-key order. Each item's `path_condition()` equals its own record's canonical path-condition sequence: both contain the `if p` guard occurrence, with `holds` true for the then occurrence and false for the else occurrence. Neither sequence is replaced by the other, flattened, or deduplicated. An unguarded record's item carries an empty path condition. | Test |
| FR-075-AC-10 | A descriptor advertising `probabilistic-satisfaction` with evidence kinds `{statistical}`, `{exact}` or `{statistical, exact}` is admitted and retains exactly the supplied set on the descriptor. | Test |
| FR-075-AC-11 | A descriptor advertising `probabilistic-satisfaction` with an absent evidence-kind set, an empty set, or any member other than the exact labels `statistical` and `exact` is refused by `BackendDescriptor::admit` with `invalid_capability`/`unknown-evidence-kind`, keyed by `BackendId`, before registry insertion; it contributes no candidate and leaves any earlier registration under that identity standing. | Test |
| FR-075-AC-12 | A descriptor that advertises no `probabilistic-satisfaction` kind is admitted without evidence-kind labels, and its admission is unaffected by whether it advertises any of the other eleven kinds. | Test |
| FR-075-AC-13 | For a probabilistic item naming `statistical` or `exact`, the candidate set contains exactly registrations advertising that same evidence kind; a registration advertising only the other evidence kind is excluded, including when it is the only registration. Missing or unknown evidence remains available to downstream negotiation for the corresponding invalid-request refusal and is not substituted. | Test (TC-980, TC-983) |
| FR-075-AC-14 | Two registrations identical in every descriptor member except that their evidence-kind sets differ are conflicting descriptors: the registry refuses the identity with `invalid_capability`/`duplicate-backend`, withdraws its registration, and reports no candidate for it; equal complete descriptors remain idempotent under FR-075-AC-7. A request naming a registered backend with the nonmatching evidence kind has an empty candidate set. | Test (TC-981, TC-983) |
| FR-075-AC-15 | For every request item, `conversion_context()` returns the record's direct conversion context as `Option<&OccurrenceKey>`, separate from `result_bound()`. In FR-062 RR-16, the two requirement records share the `+` node but have distinct occurrence keys and directly wrapping narrow occurrence keys; the builder returns two items in occurrence-key order, and each item's `conversion_context()` equals its own record's narrow occurrence key while its `result_bound()` is respectively `Int[0, 10]` or `Int[0, 20]`. A record with no directly wrapping narrow returns `None`; when the same application node occurs multiple times, each item retains its own site's context and no context is selected by the shared expression node key. The two narrow contexts add no `RequirementItem`, routed item or claim. | Test (TC-449) |
| FR-075-AC-16 | For `function f using v(a: Int[0, 9]): Int[0, 10] pure { a + 1 }`, the driver's IR call includes roots `[item.node(), conversion_context.node()]` (or the equivalent contextual entry point), with context paired to the item by request index and occurrence key. For RR-16, the driver makes this mapping independently for both items, which share the `+` node but have different request indices and conversion contexts: each item passes its own narrow context root, and neither context is swapped or selected by node key. In both vectors, the narrow is context only; it creates no additional requirement item, routed item or claim, and the routed claim remains the scalar application occurrence. A bare `lower([add])` call still returns the behavior specified by Contract IR and is not changed by this criterion. | Test (TC-449) |
| FR-075-AC-17 | For every `CallRequirement` of FR-395 over fixture K, `items_from_requirements` over the package's records yields at the member's request index an item whose `occurrence()` equals the member's key. | Test (TC-1089) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §7.1
  and §7.2 fix the registry's shape (an ordinary value, built once, passed as
  an argument) and the candidate-computation algorithm this requirement
  implements; §5.2 lists the explicit registry failure cases FR-075-AC-3 and
  FR-075-AC-4 verify.
- [FR-057](FR-057-admit-shared-capability-kinds.md) admits the twelve FR-290
  capability-kind labels into QSL and states the same candidate-set rule from
  the composed-linker admission side; this requirement implements it at the
  registry.
- [FR-290](ix://agent-ix/quire-specification/FR-290) (quire-specification)
  is the normative source for the twelve capability-kind labels, the
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
  twelve labels, not six, "are the single capability-kind vocabulary of the
  Quire ecosystem," from which "`quire-spec-language`'s `Capability` type,
  the backend descriptors registered in its registry... take their
  members." FR-290 governs; this requirement and FR-057 specify against the
  twelve-label vocabulary throughout, and #213's `Capability` type is expected
  to carry all twelve.
- The canonical `Capability` value type and its outcome constructors are
  owned by [quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213);
  this requirement consumes that type and specifies no shape for it.
- [FR-080](FR-080-registry-evidence-and-gates.md) records the permutation
  property test (FR-075-AC-2, evidenced by TC-194) as part of the ADR-012
  §5.3 registry evidence obligations, and the `cargo-deny`/lint gates that
  enforce FR-075-CON-1.
