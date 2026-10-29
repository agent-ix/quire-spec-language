---
id: FR-484
title: "Identify a function-value parity claim and retain its replay inputs"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-201
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-322
    type: traces_to
---
# FR-484: Identify a function-value parity claim and retain its replay inputs

## Description

A function-value parity claim says that the generated code of a
non-`Boolean` function `f` returns QSL's exact `f(b)` at the parameter
bindings `b` it was run at (FR-357, function level). Its obligation is the
whole function: on every argument vector the declared signature of `f`
admits, the generated outcome equals QSL's exact one. A counterexample is
one such vector together with the generated outcome observed there.

QSL SHALL own this obligation's identity. It SHALL define the preimage,
mint it as the preimage value itself, compared by structural equality
(ADR-013 O-09), hand it to the claim producer through
`qsl_replay::call_site` (FR-121), and recompute the preimage and compare it
for equality when `qsl_replay::replay_value_parity` replays the claim. No
other layer derives or spells any member of it. A
claim producer, CG among them, retains the inputs this requirement names
exactly as QSL and the generated-code driver handed them over, and builds
the replay from them alone.

The identity's home is the `qsl-replay` crate: the preimage type
`ValueParityPreimage` with its `ValueParityArgument` entries, the minting
function `value_parity_obligation`, and the `ValueParitySite` that
`FunctionSite` carries. Each identity it mints is a
`qsl_replay::ObligationIdentity` (ADR-013 O-09).

## Preimage

The preimage is a structural value with exactly these four members, compared
by equality:

| Member | Value |
| --- | --- |
| `arguments` | One entry per declared parameter of `f`, in declared parameter order, possibly empty. |
| `node` | The structural key of `f`'s checked `function` node (`node_tag` `function`, FR-092): its owner, declaration path, role and ordinal. |
| `occurrence_key` | `{"ordinal": 0, "role": "declaration"}`: the role and ordinal of `f`'s one `declaration` occurrence (ADR-013 O-07). The occurrence key's node is the `node` member and is not repeated. |
| `subject_kind` | The string `function-value-parity`. |

Each `arguments` entry is a structural value with exactly these three
members:

| Member | Value |
| --- | --- |
| `parameter` | The structural key of the parameter's `value`/`parameter` node, the same node key `FunctionSite.parameters` pairs with the parameter (FR-121) and an `Input` assignment names. |
| `position` | The parameter's zero-based position in the declared parameter list. |
| `type` | The structural key of the parameter node's `semantic_type`, its declared type (QSpec FR-322 "signature type nodes"). |

The declared type node is the parameter's declared domain. It covers every
parameter type the S6a admission of FR-098 accepts, bounded or not, scalar
or composite, without a harness bound: the claim is checked on the
signature's own domain, so no drawn or substituted bound enters the
identity.

No other member is in the preimage. The bindings, the generated outcome,
the package identity, the function's
qualified name, the accounting and stage limits and every source region are
outside it; they belong to the claim, not to its obligation (Retained claim
inputs). The function's result type, body and owner are bound through
`node`, whose structural key changes whenever its owner, declaration path, role or ordinal changes.

The preimage has no `obligation_kind` member: a value-parity claim is
produced without a CG backend obligation, so it has no CG
`ObligationKind`. The `subject_kind` tag keeps the preimage apart from
every other O-09 form: it differs from the `protocol-refinement-step` and
`process-provider` tags, and an operator-parity or composite parity
preimage carries `obligation_kind` and no `subject_kind`.

## Retained claim inputs

A value-parity claim is replayed from exactly these inputs, which its
producer retains from the moment the claim is observed until it builds the
replay. Each comes from the one source the table names, and the producer
derives none of them.

| Input | Taken from | Carried as |
| --- | --- | --- |
| The proved package reference: `package_id`, the `sources`, one `dependencies` entry per dependency selection, and the byte provision | The compiled package the generated code was generated from (`CallSite.package_id`, and the `RawSourceRef` of every source and supplied library it was compiled from) | The FR-071 request's package members |
| The semantic profile selections | Fixed by the form: the empty list | The request's `profile_selections` (FR-071) |
| The `function` replay target | The `QualifiedName` the producer passed to `call_site` | The request's `replay.function` (QSpec FR-323) |
| The occurrence key | `FunctionSite.declaration` | The request's `replay.occurrence_key` |
| The obligation identity | `FunctionSite.value_parity.obligation_identity` | The request's `obligation_identity` |
| The bindings `b` | The argument vector the generated code was run at, each value keyed by the parameter node id `FunctionSite.parameters` pairs with its parameter | The request's `source`, a `ReplaySource::Input` of one `CanonicalAssignment` per declared parameter |
| The accounting and stage limits | The producer's replay configuration | The request's accounting and `stage_limits` members (FR-255) |
| The supplied-value ceilings: admission work A, conversion work H and the numeric magnitude bound B | The producer's replay configuration, each omitted ceiling resolved to its published default (A and H 16777216 units, B 4096 bits, QSpec FR-323, QSL FR-277) | The request's complete effective `limits.supplied` object |
| The state environment | Fixed by the form: the empty state environment (no anchors, values, populations, frames or captures) | The request's `state_environment` |
| The object environment | Fixed by the form: none | The request's object environment, absent |
| The proof bounds and declared domains | Fixed by the form: the empty list | The request's proof bounds and declared domains |
| The generator identity | The generated-code driver: the QSpec FR-331 `BackendId` of the provider whose `generate` operation produced the artifact it ran | The request's `backend` member (ADR-013 O-19) |
| The generated outcome | The generated-code driver's observation at `b`: a value, an out-of-range outcome, `Undefined` or `OtherRefusal` | `ValueClaim.generated`, a `ScalarOutcome` |

The proof bounds and declared domains are empty because the obligation is
checked on the declared signature itself: each argument's `type` binds its
parameter's declared domain into the identity, the S6a admission of FR-098
checks the bindings against that signature, and no proving run drew or
declared a finite bound. A parameter whose declared type has no finite
bound, such as an unbounded `Integer`, therefore needs no `ProofBound`, and
a bounded one gets none either, so one rule covers every parameter type.

The request's `profile_selections` are FR-071 semantic profile selections:
`ProfileSelection { profile, value }` entries of the proving run, filling
the role `replay.semantic_profile_selections` and each drawn from FR-071's
closed known set. A value-parity claim has no proving run, so no semantic
profile selection is in effect for it, and the form fixes the list empty
for every package. They are not the entries of the package's
`profile_selections` (QSpec FR-322). Each such entry is a clause-profile law,
`{"definition": {"authority", "identity"}, "role"}` with `role`
`temporal_profile` or `protocol_profile`, that the package's temporal or
protocol clauses select. Such an entry is no semantic profile selection under
any mapping: FR-071's decode refuses every profile outside its closed set
`unknown_profile`/`unsupported-selection`. The entries are bound into the claim through
`package_id`, which the replay recomputes from the sources it recompiles
(BLOCKED-BY QSpec #510/STD-583 (owner: planner titan:QSL:plan)). No producer reads, maps or copies
such an entry into the request or the packet, and QSL defines no accessor for
one: a package that selects temporal or protocol profiles gives its
value-parity claims the same empty list as a package that selects
none.

The supplied-value ceilings are retained because they decide whether the
replay reaches the call: A and H bound admission and conversion work, and B
bounds the bit length of each operand of a supplied value-in-domain
comparison (QSpec FR-323 rule 5: the value and the declared lower and upper
endpoints of a bounded `Int` or `Rational` parameter). A binding whose
operand exceeds B stops before the call with the FR-277 limit cause for
`supplied.numeric_magnitude_bits`, which `replay_value_parity` settles
`Incomplete` (`PreCallIncomplete`), not as an input refusal. A parameter of
the unbounded type `Integer` has no value-in-domain comparison, so B does
not limit its bindings; B changes the outcome only of a claim with a
bounded numeric parameter. All three ceilings are bound into the claim
whatever the parameter types, since they are members of the request's
identity (QSpec FR-323).
The state and object environments are empty because the claim is about a
`pure` function: its call reads no state and no object population, and
`replay_value_parity` evaluates it under the empty object environment.

The generator identity is the claim's `backend` because the provider whose
generated code the claim is about produced the counterexample; no
verification backend ran. QSL records it and does not interpret it
(ADR-013 O-19).

### The input-arm packet

A producer that hands the claim on as an FR-070 `WitnessPacket<NoPayload>`
before the request is built (CG FR-024's input-arm parity producer) SHALL
fill every packet member from the inputs above, with exactly these values,
and nothing else:

| Packet member | Value for a function-value parity claim |
| --- | --- |
| `obligation_identity` | `FunctionSite.value_parity.obligation_identity` |
| `occurrence_key` | `FunctionSite.declaration`: (function node id, `declaration`, 0) |
| `clause_node` | `FunctionSite.function`, the function node id, which is also the preimage's `node` |
| `replay_target` | `function`, the `QualifiedName` passed to `call_site`, and no other target member |
| `package_id` | `CallSite.package_id` |
| `sources` | The `RawSourceRef` of every source the package was compiled from, as the request's package reference names them |
| `profile_selections` | The empty list, as in the table above, whatever entries the package's `profile_selections` holds |
| `run_limits` | The request's accounting limits (`ScalarLimitsV1`): there is no proving run, and these are the limits QSL's exact evaluation runs under. The supplied-value ceilings are not a packet member; they travel in the request's `limits.supplied` only |
| `declared_domains` | The empty list |
| `backend` | The generator identity |
| `trace_position` | Present with the value none (`Some(None)`): a function claim has no trace position |
| `source` | `ReplaySource::Input` of the bindings `b` |
| `family_payload` | `NoPayload` |

`WitnessEnvelope::reconstruct` admits such a packet, and the request built
from the admitted envelope carries the same members.

### What the claim identity holds

`replay_value_parity` takes the request, the `ValueClaim` and the
`ReplayLimits`. Its `claim()` holds the obligation identity, the
`package_id`, the `function` target, the occurrence key, the `source`, the
effective accounting limits, the effective supplied-value ceilings (A, H
and B) and the generated outcome, and
nothing else. The other retained inputs
enter the claim through these members: the sources, the
dependencies, the byte provision and the package's clause-profile entries through
`package_id`, which the replay checks against the package it recompiles
from them; the semantic profile selections are fixed empty by the form,
and the replay refuses a request carrying any (Behavior); the stage limits through the effective accounting limits; the
proof bounds and declared domains and the empty state and object
environments are fixed by the form, and no environment is read by the
call, so neither can change an outcome. The generator
identity is provenance, as on every replay request, and is not a member
of the claim. Two claims are one claim exactly when their `claim()` values
are equal, whatever their results read.

## Inputs

- Minting: a `call_site` function selection (FR-121) of a function whose
  declared result is not `Boolean`.
- Replay: an FR-071 `function` replay request built from the retained
  inputs, a `ValueClaim { generated }` and the
  `ReplayLimits` (FR-357).

## Outputs

- `qsl_replay::ValueParityPreimage { node: NodeKey, arguments:
  Vec<ValueParityArgument> }`, with `ValueParityArgument { parameter:
  NodeKey, position: usize, semantic_type: NodeKey }`. The occurrence
  key and the `subject_kind` are fixed by the form and are not fields.
- `qsl_replay::value_parity_obligation(&ValueParityPreimage)`: the preimage
  itself as the identity, after the position check, or the position-order
  refusal of the Behavior section. BLOCKED-BY STD-583 (owner: planner titan:QSL:plan).
- `FunctionSite.value_parity: Option<ValueParitySite>`, with
  `ValueParitySite { preimage: ValueParityPreimage, obligation_identity:
  ObligationIdentity }`: `Some` exactly when the selected function's
  declared result is not `Boolean`.
- `ValueParityReport.claim()`, a `ValueIdentity` holding every retained
  input (FR-357).

## Behavior

- When `call_site` resolves a function selection whose declared result is
  not `Boolean`, it SHALL build the function's `ValueParityPreimage` from
  the compiled package, with `node` the function node id it returns as
  `FunctionSite.function` and one argument per declared parameter whose
  `parameter` is the node id it pairs with that parameter in
  `FunctionSite.parameters` and whose `semantic_type` is that parameter
  node's `semantic_type`, and SHALL return it with its
  `value_parity_obligation` identity as `FunctionSite.value_parity`.
- When `call_site` resolves a function selection whose declared result is
  `Boolean`, it SHALL return `FunctionSite.value_parity` as `None`: such a function is a
  predicate, whose claim FR-098 replays, and it has no value-parity claim.
- If `value_parity_obligation` refuses the preimage, `call_site` SHALL
  refuse `CallSiteRefusal::Fault` naming the check's cause and return no
  site, since no identity exists for the claim.
- `value_parity_obligation` SHALL return the preimage of the Preimage
  section and nothing else as the identity. It SHALL NOT sort the
  arguments: their order is the declared parameter order, and an argument
  whose `position` is not its index in the list refuses with a position-order
  refusal naming the index and the position.
- `replay_value_parity` SHALL, after FR-357's replay-target check and
  before the recompile, refuse a request whose `profile_selections` is not
  the empty list with `ReplayRefusal::UnusedProfileSelections`
  (`invalid_runtime_input`/`unknown-member`), holding the request's
  selections in request order. It SHALL refuse so whatever the package and
  the bindings are, read no byte-provision entry and make no call and no
  charge. The refusal's display text SHALL be exactly
  ``invalid_runtime_input/unknown-member: `profile_selections` is supplied
  for a function-value parity claim, which has no proving run`` on one
  line. It settles `Inconclusive(ReplayRefused(invalid_runtime_input))` with
  the claim identity carried unchanged.
- When `replay_value_parity` replays a claim, it SHALL, after FR-357's
  recompile, `package_id`, replay-target, `UnknownFunction` and
  `NotAValueFunction` checks and before the argument join, compare the
  request's occurrence key with the selected function's `declaration`
  occurrence (its node, role `declaration`, ordinal 0) by node, role and
  ordinal. A different key SHALL refuse `ReplayRefusal::ClaimOccurrence`
  (`missing_declaration`/`missing-name`) naming the request's occurrence
  key and the selected function, before any call and with no charge.
- `replay_value_parity` SHALL then build the selected function's
  `ValueParityPreimage` from the recompiled package by the rule
  `call_site` uses, recompute its identity with `value_parity_obligation`
  and compare it with the request's `obligation_identity`. A mismatch SHALL
  refuse `ReplayRefusal::ScalarIdentity`
  (`stale_dependency`/`content-mismatch`) with the cause `Obligation`,
  naming the request's identity as `claimed` and the recomputed one as
  `recomputed`. A preimage refusal SHALL refuse `ScalarIdentity` with the
  cause `Preimage`, whatever identity the request claimed.
- `replay_value_parity` SHALL read no preimage member from the request:
  the request carries the identity alone, and every member is taken from the
  recompiled package, so an identity minted for another function, another
  parameter list or another declared type never passes, and an identity
  that passes names exactly the selected function's obligation.
- Each refusal of these checks SHALL settle
  `Inconclusive(ReplayRefused(code))` with the claim identity carried
  unchanged, as every FR-357 refusal does.
- `replay_value_parity` SHALL carry every retained input into the report's
  `claim()`: the request's obligation identity, its `package_id` as
  supplied, its `function` target as sent (absent for another target), its
  occurrence key as sent (absent when the request carries none), its
  `source` as supplied, its effective accounting limits, its effective
  `limits.supplied` ceilings (each omitted ceiling resolved to its
  published default, so an omitted B and a stated 4096 are one claim), and
  the `ValueClaim`'s generated outcome.
- A `ValueClaim` holds the generated outcome only. The generated-code driver compares the artifact it ran with the one it generated before it hands the claim over (quire-driver FR-009); QSL never sees the artifact.
- A claim producer SHALL take the obligation identity from
  `FunctionSite.value_parity.obligation_identity` and each other retained
  input from the source the Retained claim inputs table names, and SHALL
  NOT compute or spell a value-parity preimage itself.
- When a producer hands a function-value parity claim on as a
  `WitnessPacket<NoPayload>`, it SHALL fill each packet member with the
  value "The input-arm packet" names, and SHALL NOT choose, default or
  derive any member itself.
- A producer SHALL fill the request's and the packet's
  `profile_selections` with the empty list, and SHALL NOT read the package's
  `profile_selections` to build either.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-484-AC-1 | `value_parity_obligation` over the preimage with `node` `F` and the arguments (`P1`, 0, `T1`) and (`P2`, 1, `T2`), where `F` is a `function` node key and `P1`, `T1`, `P2`, `T2` are distinct node keys, returns an identity equal by tuple equality to `{node: F, arguments: [(P1, 0, T1), (P2, 1, T2)], occurrence_key: (declaration, 0), subject_kind: function-value-parity}`; its first argument entry alone, the parameter `x: Int[0, 9]` at level 0 (QSpec FR-322 P4 typed by its T4), is the tuple (P4 node key, 0, T4 node key). | Test (TC-1252) |
| FR-484-AC-2 | With the same `node`, the empty argument list gives an identity equal to `{node: F, arguments: [], occurrence_key: (declaration, 0), subject_kind: function-value-parity}`, and the first argument of AC-1 alone gives an identity equal to its own tuple; the two parameters of AC-1 declared in the other order (positions swapped) give an unequal identity, so the declared order is part of the identity. | Test (TC-1252) |
| FR-484-AC-3 | Changing `node`, one argument's `parameter`, `position` or `type`, or adding or removing an argument, gives an unequal identity; an argument whose `position` is not its index refuses with the position-order refusal naming the index and the position; and the identity of AC-1 is unequal to the operator-parity identity of a preimage with the same `node` and an `obligation_kind`, since no preimage of one form equals one of the other. | Test (TC-1252) |
| FR-484-AC-4 | Over a unit declaring `function inc using v(x: Int[0, 9]): Integer pure { x + 1 }`, `function add using v(x: Int[0, 9], y: Int[0, 9]): Integer pure { x + y }` and `function small using v(x: Int[0, 9]): Boolean pure { x < 3 }`, `call_site` returns for `inc` and `add` a `value_parity` whose `preimage.node` is `FunctionSite.function`, whose arguments are `FunctionSite.parameters`' node ids in declared order each with the `semantic_type` the compiled package's parameter node carries, and whose `obligation_identity` equals, by tuple equality, that preimage written out independently from the package's node keys; for `small` it returns `value_parity` as `None`. | Test (TC-1252) |
| FR-484-AC-5 | A value-parity request for `inc` built only from AC-4's retained inputs (the `CallSite` package reference, `inc`'s `QualifiedName`, `FunctionSite.declaration`, `value_parity.obligation_identity`, one `Input` assignment of `x` = 3 keyed by its `FunctionSite` node id, the limits) replays with `ValueClaim { generated: 4 }` to `Agrees` and with `generated: 5` to `Diverged`, each report's `claim()` holding every retained input unchanged. | Test (TC-1252) |
| FR-484-AC-6 | AC-5's request with `obligation_identity` replaced by `add`'s identity, by an identity with an empty argument list, or by an identity over `inc`'s preimage with `type` changed to another node key refuses `ScalarIdentity` (`stale_dependency`/`content-mismatch`) with cause `Obligation` naming the claimed and the recomputed identity, after the occurrence check and before the argument join (an assignment naming `add`'s `y` added still refuses `Obligation`, not `UnknownParameter`), and settles `Inconclusive(ReplayRefused(stale_dependency))` with the claim identity carried. | Test (TC-1252) |
| FR-484-AC-7 | AC-5's request with the occurrence key replaced by `add`'s `declaration` occurrence, by the `expression` occurrence of `inc`'s `x + 1` application, or by (`inc`'s node, `declaration`, 1) refuses `ClaimOccurrence` (`missing_declaration`/`missing-name`) naming that key and `inc`, before any call and with no charge; for `small` it refuses `NotAValueFunction` instead. | Test (TC-1252) |
| FR-484-AC-8 | Two `inc` claims that differ only in the occurrence key carry their own key unchanged on `Agrees`, `Diverged`, `RefusedInput` and an identity refusal, and their `claim()` values are unequal. (The generated-content identity half of this criterion is removed: a `ValueClaim` holds the generated outcome only.) | Test (TC-1252) |
| FR-484-AC-9 | Over AC-4's unit with `function grow using v(n: Integer): Integer pure { n + 1 }` added, the `WitnessPacket<NoPayload>` for `inc` at `x` = 3 and for `grow` at `n` = 41, each filled by "The input-arm packet" from `call_site` (`clause_node` = `FunctionSite.function`, `occurrence_key` = `FunctionSite.declaration`, `obligation_identity` = `value_parity.obligation_identity`, `profile_selections` = `[]`, `run_limits` = the request's accounting limits, `declared_domains` = `[]`, `backend` = the driver's generator identity, `generator-under-test` in the test, `trace_position` = none, `source` = `Input`), reconstructs through `WitnessEnvelope::reconstruct`; every member round-trips exactly; the request built from the admitted envelope replays `grow` against generated 42 to `Agrees` and `inc` against 5 to `Diverged`. | Test (TC-1252) |
| FR-484-AC-10 | Over AC-9's unit with `function wide using v(m: Int[0, 2199023255552]): Integer pure { m + 1 }` added (the upper bound is 2^41), the request for `wide` at `m` = 2^40 (1099511627776, a 41-bit value), retained with B 4096, replays against generated 1099511627777 to `Agrees`; the same request with B 40 settles `PreCallIncomplete`, which is `Incomplete(ResourceExhausted)`, with the limit cause naming `supplied.numeric_magnitude_bits` and limit 40, and no outcome compared, and its `claim()` differs from the first only in B; the same request with B omitted has a `claim()` equal to the first; AC-9's `grow` request at `n` = 2^40 with B 40 is admitted (an unbounded `Integer` has no value-in-domain comparison) and replays against 1099511627777 to `Agrees`; and two `wide` claims differing only in A or only in H are unequal. | Test (TC-1252) |
| FR-484-AC-11 | Over the unit `UC` (TC-1252: the ConfigVersion model, the temporal profile `quire.temporal.infinite-trace/v1` and the protocol profile `quire.protocol.finite-global/v1` selected, the temporal clause `ReachesTwo`, the protocol `Flow` and `inc`), the emitted package's `profile_selections` holds exactly the two entries whose RFC 8785 texts are `{"definition":{"authority":"agent-ix","identity":"quire.temporal.infinite-trace/v1"},"role":"temporal_profile"}` and `{"definition":{"authority":"agent-ix","identity":"quire.protocol.finite-global/v1"},"role":"protocol_profile"}`; the `WitnessPacket<NoPayload>` for `inc` at `x` = 3, filled by "The input-arm packet", has `profile_selections` `[]`, reconstructs through `WitnessEnvelope::reconstruct`, and the request built from the admitted envelope decodes (no `unknown_profile`/`unsupported-selection` refusal) and replays against generated 4 to `Agrees` and against 5 to `Diverged`, each `claim()` holding the `package_id` of `UC`; the same request for `inc` over AC-4's unit, whose package `profile_selections` is `[]`, carries the same `profile_selections` `[]`. | Test (TC-1252) |
| FR-484-AC-12 | AC-5's request for `inc` with `profile_selections` `[ProfileSelection { profile: "quire.profile.v1", value: "x" }]` decodes, and `replay_value_parity` refuses it `UnusedProfileSelections` (`invalid_runtime_input`/`unknown-member`) holding that one selection, with the display text ``invalid_runtime_input/unknown-member: `profile_selections` is supplied for a function-value parity claim, which has no proving run``, before the recompile, with no call and no charge, and settles `Inconclusive(ReplayRefused(invalid_runtime_input))` with the claim identity carried unchanged; it refuses so with the generated outcome 4 and 5 alike, with `obligation_identity` replaced by an identity with an empty argument list (no `Obligation` refusal is reached); with `quire.profile.v1` the one member of FR-071's closed known set, a request whose selection names `quire.profile.unknown/v1` refuses at decode `unknown_profile`/`unsupported-selection` (FR-071-AC-10) and never reaches the entry; and the request carrying either profile entry of AC-11 mapped to a `ProfileSelection` (`profile` the entry's definition identity, `value` its role) refuses at decode `unknown_profile`/`unsupported-selection`. | Test (TC-1252) |
| FR-484-AC-13 | AC-12's request with the `[("quire.profile.v1", "x")]` selection and its replay target replaced by `frame_operation` (`Config::ConfigVersion`, `attemptUpdate`) or by `state_clause` (`VersionUnchanged`) refuses the FR-357-AC-20 replay-target refusal (`invalid_runtime_input`/`unknown-member` at `/replay/frame_operation` or `/replay/state_clause`, naming the target, the entry and the member `function`), not `UnusedProfileSelections`, before the recompile. | Test (TC-1252) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-09: the obligation identity's subject kinds.
- [FR-357](FR-357-replay-scalar-parity-claims.md): the value-parity
  replay entry whose claim this identifies.
- [FR-121](FR-121-locate-a-function-call-site-through-the-replay-facade.md):
  the `FunctionSite` that carries the minted identity.
- [FR-098](FR-098-execute-a-replay-request.md): the recompile, selection,
  `ClaimOccurrence` refusal and S6a admission the entry reuses.
- [FR-071](FR-071-implement-typed-replay-request.md): the request's
  semantic profile selections and their closed known set.
- QSpec FR-201: the `quire.ir-proof.obligation-identity/v1` domain.
- QSpec FR-322: the `function` node, `value`/`parameter` nodes and their
  `semantic_type`.
