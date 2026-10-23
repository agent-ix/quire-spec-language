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
mint the digest in the `quire.ir-proof.obligation-identity/v1` domain
(QSpec FR-201) through ADR-013 section 2's one canonical encoder, hand it
to the claim producer through `qsl_replay::call_site` (FR-121), and
recompute and check it when `qsl_replay::replay_value_parity` replays the
claim. No other layer derives, spells or hashes any member of it. A
claim producer, CG among them, retains the inputs this requirement names
exactly as QSL and the generated-code driver handed them over, and builds
the replay from them alone.

The identity's home is the `qsl-replay` crate: the preimage type
`ValueParityPreimage` with its `ValueParityArgument` entries, the minting
function `value_parity_obligation`, and the `ValueParitySite` that
`FunctionSite` carries. Each identity it mints is a
`qsl_replay::ObligationIdentity` (ADR-013 O-09).

## Preimage

The preimage is an RFC 8785 object, encoded by ADR-013 section 2's one
encoder so that its members sort by the UTF-16 code units of their names,
with exactly these four members:

| Member | Value |
| --- | --- |
| `arguments` | One entry per declared parameter of `f`, in declared parameter order, possibly empty. |
| `node` | The lowercase hexadecimal spelling, 64 digits, of `f`'s checked `function` node id (`node_tag` `function`, FR-092). |
| `occurrence_key` | `{"ordinal": 0, "role": "declaration"}`: the role and ordinal of `f`'s one `declaration` occurrence (ADR-013 O-07). The occurrence key's node is the `node` member and is not repeated. |
| `subject_kind` | The string `function-value-parity`. |

Each `arguments` entry is an RFC 8785 object with exactly these three
members:

| Member | Value |
| --- | --- |
| `parameter` | The lowercase hexadecimal node id of the parameter's `value`/`parameter` node, the same node id `FunctionSite.parameters` pairs with the parameter (FR-121) and an `Input` assignment names. |
| `position` | The parameter's zero-based position in the declared parameter list, a JSON integer. |
| `type` | The lowercase hexadecimal node id of the parameter node's `semantic_type`, its declared type (QSpec FR-322 "signature type nodes"). |

The declared type node is the parameter's declared domain. It covers every
parameter type the S6a admission of FR-098 accepts, bounded or not, scalar
or composite, without a harness bound: the claim is checked on the
signature's own domain, so no drawn or substituted bound enters the
identity.

No other member is in the preimage. The bindings, the generated outcome,
the generated-content identity, the package identity, the function's
qualified name, the accounting and stage limits and every source region are
outside it; they belong to the claim, not to its obligation (Retained claim
inputs). The function's result type, body and owner are bound through
`node`, whose content-addressed id changes whenever any of them changes.

The preimage has no `obligation_kind` member: a value-parity claim is
produced without a CG backend obligation, so it has no CG
`ObligationKind`. The `subject_kind` tag keeps the preimage apart from
every other O-09 form: it differs from the `protocol-refinement-step` and
`process-provider` tags, and an operator-parity or composite parity
preimage carries `obligation_kind` and no `subject_kind`.

Every integer of the preimage is a JSON integer: the ordinal is 0 and a
position is a parameter count, both far below 2^53, so no member uses the
decimal-string spelling of ADR-013 O-09.

## Retained claim inputs

A value-parity claim is replayed from exactly these inputs, which its
producer retains from the moment the claim is observed until it builds the
replay. Each comes from the one source the table names, and the producer
derives none of them.

| Input | Taken from | Carried as |
| --- | --- | --- |
| The proved package reference: `package_id`, the source digests, one `dependencies` entry per dependency selection, and the byte provision | The compiled package the generated code was generated from (`CallSite.package_id`, and the `RawSourceRef` of every source and supplied library it was compiled from) | The FR-071 request's package members |
| The semantic profile selections | The `profile_selections` of the v2 lock of `CallSite.package`, in the lock's order (the empty list when the lock records none) | The request's profile selections |
| The `function` replay target | The `QualifiedName` the producer passed to `call_site` | The request's `replay.function` (QSpec FR-323) |
| The occurrence key | `FunctionSite.declaration` | The request's `replay.occurrence_key` |
| The obligation identity | `FunctionSite.value_parity.obligation_identity` | The request's `obligation_identity` |
| The bindings `b` | The argument vector the generated code was run at, each value keyed by the parameter node id `FunctionSite.parameters` pairs with its parameter | The request's `source`, a `ReplaySource::Input` of one `CanonicalAssignment` per declared parameter |
| The accounting and stage limits | The producer's replay configuration | The request's accounting and `stage_limits` members (FR-255) |
| The proof bounds and declared domains | Fixed by the form: the empty list | The request's proof bounds and declared domains |
| The generator identity | The generated-code driver: the FR-331 `BackendId` of the provider whose `generate` operation produced the artifact it ran | The request's `backend` member (ADR-013 O-19) |
| The generated outcome | The generated-code driver's observation at `b`: a value, an out-of-range outcome, `Undefined` or `OtherRefusal` | `ValueClaim.generated`, a `ScalarOutcome` |
| The generated-content identity | The generated-code driver, which authenticates it against the artifact it ran before it hands the claim over | `ValueClaim.content_identity`, a `DigestRecord` |

The proof bounds and declared domains are empty because the obligation is
checked on the declared signature itself: each argument's `type` binds its
parameter's declared domain into the identity, the S6a admission of FR-098
checks the bindings against that signature, and no proving run drew or
declared a finite bound. A parameter whose declared type has no finite
bound, such as an unbounded `Integer`, therefore needs no `ProofBound`, and
a bounded one gets none either, so one rule covers every parameter type.

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
| `source_digests` | The `RawSourceRef` of every source the package was compiled from, as the request's package reference names them |
| `profile_selections` | The lock's `profile_selections`, as in the table above |
| `run_limits` | The request's accounting limits: there is no proving run, and these are the limits QSL's exact evaluation runs under |
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
effective accounting limits, the generated outcome and the
generated-content identity, and nothing else. The other retained inputs
enter the claim through these members: the source digests, the
dependencies, the byte provision and the profile selections through
`package_id`, which the replay checks against the package it recompiles
from them; the stage limits through the effective accounting limits; the
proof bounds and declared domains are fixed by the form. The generator
identity is provenance, as on every replay request, and is not a member
of the claim. Two claims are one claim exactly when their `claim()` values
are equal, whatever their results read.

## Inputs

- Minting: a `call_site` function selection (FR-121) of a function whose
  declared result is not `Boolean`.
- Replay: an FR-071 `function` replay request built from the retained
  inputs, a `ValueClaim { generated, content_identity }` and the
  `ReplayLimits` (FR-357).

## Outputs

- `qsl_replay::ValueParityPreimage { node: WireNodeId, arguments:
  Vec<ValueParityArgument> }`, with `ValueParityArgument { parameter:
  WireNodeId, position: usize, semantic_type: WireNodeId }`. The occurrence
  key and the `subject_kind` are fixed by the form and are not fields.
- `qsl_replay::value_parity_obligation(&ValueParityPreimage) ->
  Result<ObligationIdentity, IdentityEncodeError>`: the SHA-256 of the
  preimage's canonical bytes.
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
  `value_parity_obligation` digest as `FunctionSite.value_parity`.
- When `call_site` resolves a function selection whose declared result is
  `Boolean`, it SHALL return `FunctionSite.value_parity` as `None`: such a function is a
  predicate, whose claim FR-098 replays, and it has no value-parity claim.
- If `value_parity_obligation` refuses the preimage, `call_site` SHALL
  refuse `CallSiteRefusal::Fault` naming the encoder's cause and return no
  site, since no identity exists for the claim.
- `value_parity_obligation` SHALL encode the preimage of the Preimage
  section and nothing else, through the one canonical encoder, and SHALL
  return the SHA-256 of those bytes. It SHALL NOT sort the arguments: their
  order is the declared parameter order, and an argument whose `position`
  is not its index in the list refuses `IdentityEncodeError` with the
  cause `PositionOrder`, naming the index and the position.
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
  `recomputed`. An encoder refusal SHALL refuse `ScalarIdentity` with the
  cause `Encoding`, whatever identity the request claimed.
- `replay_value_parity` SHALL read no preimage member from the request:
  the request carries the digest alone, and every member is taken from the
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
  `source` as supplied, its effective accounting limits, and the
  `ValueClaim`'s generated outcome and generated-content identity.
- The generated-content identity is carry-and-bind: QSL SHALL bind it into
  the claim identity and carry it unchanged, and SHALL NOT recompute,
  authenticate or refuse it. QSL never sees the generated artifact; the
  driver that ran it authenticates the identity against it before it hands
  the claim over.
- A claim producer SHALL take the obligation identity from
  `FunctionSite.value_parity.obligation_identity` and each other retained
  input from the source the Retained claim inputs table names, and SHALL
  NOT compute, spell or hash a value-parity preimage itself.
- When a producer hands a function-value parity claim on as a
  `WitnessPacket<NoPayload>`, it SHALL fill each packet member with the
  value "The input-arm packet" names, and SHALL NOT choose, default or
  derive any member itself.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-484-AC-1 | `value_parity_obligation` over the preimage with `node` 64 `2` hex digits and the arguments (`ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda`, 0, `652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477`) and (64 `3` digits, 1, 64 `4` digits) encodes exactly the 516 bytes `{"arguments":[{"parameter":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","position":0,"type":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477"},{"parameter":"3333333333333333333333333333333333333333333333333333333333333333","position":1,"type":"4444444444444444444444444444444444444444444444444444444444444444"}],"node":"2222222222222222222222222222222222222222222222222222222222222222","occurrence_key":{"ordinal":0,"role":"declaration"},"subject_kind":"function-value-parity"}` and returns `793909279b4897ab8f03f3c14547fc08861d353d979e471aa423c2bce9d8b92c`; its first argument entry alone, the parameter `x: Int[0, 9]` at level 0 (QSpec FR-322 P4 typed by its T4), encodes to `{"parameter":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","position":0,"type":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477"}` with SHA-256 `34f0d98f1306d5e3720eec3469a6181c73f809ac51c6fe608e97129e7f94270f`. | Test (TC-1252) |
| FR-484-AC-2 | With the same `node`, the empty argument list encodes to `{"arguments":[],"node":"2222222222222222222222222222222222222222222222222222222222222222","occurrence_key":{"ordinal":0,"role":"declaration"},"subject_kind":"function-value-parity"}` with digest `908362fdf1c3d0bb20aadf2249e285964ad024f5fd994d6c01e88024839166b7`, and the first argument of AC-1 alone gives `3333f20dd244bd7043f410893713ed40ffc067b45cae410050898de89834cbc0`; the two parameters of AC-1 declared in the other order (positions swapped) give `f052b4583fc320a5a470209188b911e75fe02d55a28d079a5e614ac0f44e76fd`, so the declared order is part of the identity. | Test (TC-1252) |
| FR-484-AC-3 | Changing `node`, one argument's `parameter`, `position` or `type`, or adding or removing an argument, gives a different digest; an argument whose `position` is not its index refuses `PositionOrder`; and the digest of AC-1 is unequal to the operator-parity digest of a preimage with the same `node` and an `obligation_kind`, since no preimage text of one form equals one of the other. | Test (TC-1252) |
| FR-484-AC-4 | Over a unit declaring `function inc using v(x: Int[0, 9]): Integer pure { x + 1 }`, `function add using v(x: Int[0, 9], y: Int[0, 9]): Integer pure { x + y }` and `function small using v(x: Int[0, 9]): Boolean pure { x < 3 }`, `call_site` returns for `inc` and `add` a `value_parity` whose `preimage.node` is `FunctionSite.function`, whose arguments are `FunctionSite.parameters`' node ids in declared order each with the `semantic_type` the compiled package's parameter node carries, and whose `obligation_identity` equals the SHA-256 of that preimage's canonical text written out independently from the package's node ids; for `small` it returns `value_parity` as `None`. | Test (TC-1252) |
| FR-484-AC-5 | A value-parity request for `inc` built only from AC-4's retained inputs (the `CallSite` package reference, `inc`'s `QualifiedName`, `FunctionSite.declaration`, `value_parity.obligation_identity`, one `Input` assignment of `x` = 3 keyed by its `FunctionSite` node id, the limits) replays with `ValueClaim { generated: 4, content_identity }` to `Agrees` and with `generated: 5` to `Diverged`, each report's `claim()` holding every retained input unchanged. | Test (TC-1252) |
| FR-484-AC-6 | AC-5's request with `obligation_identity` replaced by `add`'s identity, by the all-zero identity, or by an identity minted over `inc`'s preimage with `type` changed to 64 `4` digits refuses `ScalarIdentity` (`stale_dependency`/`content-mismatch`) with cause `Obligation` naming the claimed and the recomputed digest, after the occurrence check and before the argument join (an assignment naming `add`'s `y` added still refuses `Obligation`, not `UnknownParameter`), and settles `Inconclusive(ReplayRefused(stale_dependency))` with the claim identity carried. | Test (TC-1252) |
| FR-484-AC-7 | AC-5's request with the occurrence key replaced by `add`'s `declaration` occurrence, by the `expression` occurrence of `inc`'s `x + 1` application, or by (`inc`'s node, `declaration`, 1) refuses `ClaimOccurrence` (`missing_declaration`/`missing-name`) naming that key and `inc`, before any call and with no charge; with `package_id` stale it refuses `PackageIdMismatch` instead, and for `small` it refuses `NotAValueFunction` instead. | Test (TC-1252) |
| FR-484-AC-8 | Two `inc` claims that differ only in the generated-content identity carry their own identity unchanged on `Agrees`, `Diverged`, `RefusedInput` and an identity refusal, and their `claim()` values are unequal; a content identity that matches no artifact is not refused. Claims that differ only in the occurrence key are likewise unequal. | Test (TC-1252) |
| FR-484-AC-9 | Over AC-4's unit with `function grow using v(n: Integer): Integer pure { n + 1 }` added, the `WitnessPacket<NoPayload>` for `inc` at `x` = 3 and for `grow` at `n` = 41, each filled by "The input-arm packet" from `call_site` (`clause_node` = `FunctionSite.function`, `occurrence_key` = `FunctionSite.declaration`, `obligation_identity` = `value_parity.obligation_identity`, `profile_selections` = the lock's, `run_limits` = the request's accounting limits, `declared_domains` = `[]`, `backend` = the driver's generator identity, `generator-under-test` in the test, `trace_position` = none, `source` = `Input`), reconstructs through `WitnessEnvelope::reconstruct`; every member round-trips exactly; the request built from the admitted envelope replays `grow` against generated 42 to `Agrees` and `inc` against 5 to `Diverged`. | Test (TC-1252) |

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-09: the obligation identity's subject kinds and section 2's one
  encoder.
- [FR-357](FR-357-replay-scalar-parity-claims.md): the value-parity
  replay entry whose claim this identifies.
- [FR-121](FR-121-locate-a-function-call-site-through-the-replay-facade.md):
  the `FunctionSite` that carries the minted identity.
- [FR-098](FR-098-execute-a-replay-request.md): the recompile, selection,
  `ClaimOccurrence` refusal and S6a admission the entry reuses.
- QSpec FR-201: the `quire.ir-proof.obligation-identity/v1` domain.
- QSpec FR-322: the `function` node, `value`/`parameter` nodes and their
  `semantic_type`.
