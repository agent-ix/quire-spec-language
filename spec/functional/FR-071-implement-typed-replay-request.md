---
id: FR-071
title: "Implement the typed replay request"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
---
# FR-071: Implement the typed replay request

## Description

QSL SHALL implement a typed replay request, defined in the layer-6 `replay`
module and part of its public API (ADR-011 §2.1 E9, §6.1; ADR-013 O-26),
that is the ADR-013 O-25 counterexample packet plus the FR-070
envelope's members — the state environment, the `quire.value.accounting/v1`
limits, the S1-to-S4 stage limits copied from the proving run, and the
optional object environment below — and nothing else. For observed temporal
replay, the request SHALL additionally carry the typed common
`WitnessEnvelope` and the observed-arm `TemporalCounterexample` payload
defined by QSpec FR-364. This is a typed request arm, not an untyped
extension map. The type SHALL make it structurally impossible to obtain a
recompilation input other than by lookup in the request's byte provision.
The request SHALL carry a byte provision (QSpec FR-323 `byte_provision`,
ADR-013 QC-1) whose source and definition entries are keyed by their FR-001
labels (authority and identity) and whose domain package entries are keyed by
package identity. The type SHALL have
no field, accessor or constructor argument typed as a filesystem path,
environment variable name, or search location.

For a function, frame or state-clause replay, the request selects the item by
exactly one
QSpec FR-323 replay target member, typed as FR-323 types that member (a
function's `QualifiedName`, ADR-013 O-11 and the OQ-5 ruling; a frame's
`{object, operation}`, a `QualifiedName` and an `Identifier`; a state
clause's own declared `QualifiedName`), never by a bare string. This requirement builds
the request type and its round trip only; it does not build the executor
that consumes it (ADR-013 C-13, TK-01 executor entry, is #243's), and it
performs no recompilation and no function
call. The executor is [FR-098](FR-098-execute-a-replay-request.md).

The FR-170 `ModelGraph` source is the sole exception: it carries no generic
replay-target member, because its `GraphEvidenceEnvelope` selects the checked
claim by claim-node and occurrence identity. Its decoder accepts an absent
target only for that source arm and refuses a target, arguments, occurrence
key or request `over` member on that arm.

## Inputs

- An ADR-013 O-25 counterexample packet.
- The FR-070 envelope's state environment, accounting limits, and S1-to-S4
  stage limits copied from the proving run.
- Optionally, the object environment of a replay whose function takes a
  `Reference`: the reference to one `quire.state.snapshot/v1` document
  (FR-106) in the byte provision.
- The byte provision (QSpec FR-323 `byte_provision`): every source, definition
  and domain-package input the replay would recompile. A source or definition
  entry is keyed by its FR-001 labels (authority, identity); a domain package
  entry by its package identity. An observed-trace or object-environment
  snapshot document entry is keyed by the document's `identity` label set. No
  entry carries or is keyed by a content digest.
- For observed temporal replay, the common `WitnessEnvelope` and QSpec
  FR-364 `TemporalCounterexample` observed arm, including its `{document}`
  trace steps, fairness, interval and `trace_position`; its `ReplaySource`
  SHALL be `Witness`, and every observed document is in the byte provision.

## Outputs

- A typed replay request carrying exactly the members above, or a
  structured refusal when the input is malformed or names an
  out-of-domain profile identifier.

## Behavior

- The request type SHALL carry exactly the ADR-013 O-26 members (the O-25
  packet plus the FR-070 envelope members) and SHALL invent no additional
  member; a construct → serialize → read round trip SHALL preserve the
  package reference (`package_id`, every source
  (authority, identity) pair), the replay members (`ReplaySource`, `QualifiedName`, a `function`
  request's occurrence key, arguments
  keyed by parameter node id, profile selections, proof bounds and declared
  domains, trace position, `backend`), the byte provision, and the limits
  exactly.
- Every recompilation input SHALL be reachable from the request only by
  lookup in its byte provision, by source labels or package identity; the
  request type SHALL define no path-typed, environment-variable-typed, or
  search-location-typed member, and a byte-provision entry whose key is
  neither a source label pair, a package identity nor a document `identity`
  label set SHALL refuse at decode. Two entries under
  one key refuse `invalid_package`/`conflicting-definition`, naming both.
- The package reference SHALL carry QSpec FR-323's `sources` and
  `dependencies`: `sources` names the proved package's own source
  references, and `dependencies` one entry per entry of the proved
  package's `dependency_selections`, in order, each
  `{identity, package_id, sources}` (ADR-015 D-4). `identity` is a
  `LibraryName`, and the entry names the dependency package by that identity
  and QSpec FR-323's `package_id` member; an empty identity refuses at decode. Every source of every entry
  is a source reference (authority, identity) the byte provision SHALL cover,
  as the proved package's are.
- The package reference and each `dependencies` entry SHALL carry an
  optional `refinement_internals` member (QSpec FR-323), a JSON value kept
  exactly as decoded. The decoder SHALL NOT validate it; an absent member is
  the empty list, and its defects surface from the recompile (FR-098,
  FR-147).
- Each package-reference source entry SHALL carry FR-001's two source labels
  (authority and identity) and no digest, so the executor (FR-098) recompiles
  a source under exactly the labels the proving run used (ADR-013 §7 S-4b).
  This requirement checks no source byte.
- The request decoder SHALL NOT read the bytes of any byte-provision entry:
  every entry is keyed by labels or an identity. A domain package document is
  read by intake (FR-056) when the executor recompiles, and a refusal of that
  read is a `ReplayRefusal::Recompile`, never a decode refusal.
- A present `object_environment` member SHALL be a closed object with exactly
  `identity`, in the form FR-106's invocation uses for its `pre` and `post`
  members: the identity labels of a snapshot document. The `identity` is the
  label set the result retains (FR-122); admission selects the document by it.
  The member names the document, whose bytes are an entry of the byte provision
  keyed by that `identity`, like every other recompilation input. A request without the member carries no object environment.
- The request reader SHALL NOT read the `object_environment` document nor
  require its byte-provision entry at construction: the executor admits the
  document by FR-106 (FR-098), and the request defines no second document
  grammar.
- The reader SHALL refuse a malformed `object_environment` at decode, naming
  the member or the member inside it, with `invalid_runtime_input`:
  `wrong-value-kind` for a member that is not an object, or an `identity` of
  the wrong JSON kind; `missing-member` for a missing `identity`; and
  `unknown-member` for any other member, naming the first in document order.
- The byte provision SHALL be complete at construction: every source
  (authority, identity) the package reference names SHALL have a matching
  byte-provision entry; a request whose package reference names a source
  with no corresponding entry SHALL refuse at construction rather than
  admitting an incomplete provision for a later consumer to discover.
- The closed known set of semantic profiles is exactly the one profile
  identifier `quire.profile.v1`; a package's clause-profile definition
  identities (such as `quire.temporal.infinite-trace/v1` or
  `quire.protocol.finite-global/v1`) are not members, since those bind
  through `package_id` (ADR-013 O-25).
- When a semantic profile selection names a profile outside the closed
  known set, the reader SHALL refuse the request at decode with
  `unknown_profile`/`unsupported-selection`, keeping the supplied selection
  and the role it was supplied for, and SHALL read no byte-provision entry
  for it.
- The reader SHALL refuse a request whose encoded size exceeds the
  configured reader bound, and SHALL NOT return a truncated or
  partially-populated request in that case.
- The observed temporal request arm SHALL carry the common `WitnessEnvelope`
  whose family payload is the shared typed `TemporalCounterexample` sum
  (FR-331, FR-128). The sum SHALL contain `Model` and `Observed` variants.
- The `Model` variant SHALL retain FR-128's existing model payload and typed
  fields.
- The `Observed` variant SHALL carry QSpec FR-364's typed observed-arm
  payload with `trace: "observed"`, `initial_state: null`,
  `over_binding: null` and document-only steps.
- The common envelope's trace position SHALL equal the observed payload's
  `trace_position`; a mismatch SHALL refuse
  `invalid_runtime_input`/`invalid-value` at
  `/replay/observed_counterexample/trace_position` before document admission.
- An observed temporal arm SHALL refuse `ReplaySource::Input` because it
  carries a typed counterexample envelope, not canonical input assignments,
  with `invalid_runtime_input`/`invalid-value` at `/replay/source` before
  document admission.
- The request decoder SHALL refuse a model payload in the observed arm with
  `invalid_runtime_input`/`invalid-value` at
  `/replay/observed_counterexample/trace`, and SHALL refuse the observed arm
  on a function or frame target with `invalid_runtime_input`/`unknown-member`
  at `/replay/observed_counterexample`.
- A missing observed-document entry in the byte provision SHALL refuse at
  construction with `missing_import`/`missing-selection`, naming the document
  identity and its `observed_counterexample` member.
- The request decoder SHALL preserve the observed envelope and payload on a
  construct → encode → decode round trip.
- The request decoder SHALL refuse a model variant in an observed request
  arm.
- The request decoder SHALL require each observed document `identity` to name a
  byte-provision entry.
- The request decoder SHALL refuse an observed-counterexample arm on a
  function or frame target.
- The replay target SHALL hold typed names only (`QualifiedName`,
  `Identifier`); the request's public constructors and decoders SHALL NOT
  accept a bare `&str` or `String` in any replay target position.
- A request whose source is `ModelGraph` SHALL carry no generic replay
  target. The decoder SHALL accept that absence for `ModelGraph` and no other
  source, and SHALL refuse target members, arguments, occurrence key or
  request `over` data on it; the model-graph claim identity comes from its
  evidence envelope (FR-170).
- The request type SHALL provide no implicit string-to-`QualifiedName` or
  string-to-`Identifier` conversion.
- The reader SHALL refuse a request by QSpec FR-323's decode rule 1 (no
  replay target member, more than one, or a non-function target with
  `arguments` or an occurrence key), with FR-323's codes and pointers,
  before any byte-provision rule.
- The request type SHALL carry, for a `function` replay target, QSpec
  FR-323's `occurrence_key`: the occurrence key of the counterexample's failing node
  (ADR-013 O-25), its node a `WireNodeId`, its role and its ordinal. The
  executor selects the claim the request decides by it (FR-098), and the
  reader does not resolve the key: its node is looked up in the recompiled
  package.
- If a `function` request carries no `occurrence_key`, then the reader SHALL
  refuse it `invalid_runtime_input`/`missing-member` at
  `/replay/occurrence_key`, as QSpec FR-323's decode rule 1 states
  (FR-323-AC-22), before any byte-provision rule.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-071-AC-1 | A function, frame or state-clause request carries exactly its O-26 members; an observed temporal request additionally carries its typed counterexample arm. Construct → encode → decode preserves the applicable package reference, replay members, byte provision and limits. | Test (TC-185) |
| FR-071-AC-2 | The request type has no path-, environment-variable-, or search-location-typed field or accessor; a byte-provision entry keyed by anything other than a source label pair, a package identity or a document `identity` label set refuses at decode, and two entries under one source label pair or package identity refuse `invalid_package`/`conflicting-definition`. | Test (TC-186) |
| FR-071-AC-3 | No public constructor or decoder of the request accepts a bare `&str` or `String` for function selection, and no implicit conversion from a string to `QualifiedName` exists; an attempted call site passing a string literal in that position fails to compile. | Test (TC-187) |
| FR-071-AC-5 | A request whose package reference names a source (authority, identity) with no matching byte-provision entry refuses at construction with `missing_import`/`missing-selection`, naming the source labels and where the package reference names it; no incomplete request is returned for a later consumer to discover the gap. | Test (TC-186) |
| FR-071-AC-6 | **RETIRED (source digest removed).** A byte-provision entry carries no digest, so decode checks no entry's bytes. | Test (TC-186), retired |
| FR-071-AC-7 | A request whose encoded size exceeds the configured reader bound refuses, and no truncated or partially-populated request is returned. | Test (TC-186) |
| FR-071-AC-8 | An observed temporal request round trip preserves the shared `WitnessEnvelope<TemporalCounterexample>` and its observed payload, including `ReplaySource::Witness`, clause identity, document identities, fairness, interval and trace position; a missing document entry refuses `missing_import`/`missing-selection` naming its document identity, a trace-position mismatch refuses `invalid_runtime_input`/`invalid-value` at `/replay/observed_counterexample/trace_position`, `ReplaySource::Input` refuses `invalid_runtime_input`/`invalid-value` at `/replay/source`, a model variant refuses `invalid_runtime_input`/`invalid-value` at `/replay/observed_counterexample/trace`, and a non-state-clause target refuses `invalid_runtime_input`/`unknown-member` at `/replay/observed_counterexample`; each refusal occurs before document admission and returns no partial request. | Test (TC-847) |
| FR-071-AC-9 | A request whose package reference carries two `dependencies` entries round-trips them exactly, in order; an entry whose source (authority, identity) has no byte-provision entry refuses at construction, as AC-5 states; an entry with an empty identity, refuses at decode. | Test (TC-186) |
| FR-071-AC-10 | A request whose semantic profile selection names a profile outside the closed known set refuses at decode with `unknown_profile`/`unsupported-selection`, keeping the supplied selection and its role, and refuses so before the byte provision is read. | Test (TC-186) |
| FR-071-AC-11 | **RETIRED (domain-package digest removed).** Withdrawn with no replacement. | Test (TC-186), retired |
| FR-071-AC-12 | A request whose `object_environment` is `{"identity": {...}}` round-trips it exactly. Each of these refuses at decode with `invalid_runtime_input`, naming the member: a string for `object_environment` (`wrong-value-kind`); an object without `identity` (`missing-member` naming `object_environment.identity`); an `identity` that is not an object (`wrong-value-kind`); a second member of any name (`unknown-member`). A request whose `object_environment` identity has no byte-provision entry constructs: the entry is read at admission, not at decode. | Test (TC-1042) |
| FR-071-AC-13 | A request whose package reference carries `refinement_internals` `[{"refinement": "CasTwice", "internal": ["Spec::Twice::finish"]}]` and whose `dependencies` entry carries its own value round-trips both exactly; a request with the member absent round-trips with it absent; the values `{}`, `null` and `7` decode and round-trip unchanged. | Test (TC-186) |
| FR-071-AC-14 | A request whose replay target is `function`, one whose replay target is `frame_operation` (`Config::ConfigVersion`, `attemptUpdate`) and one whose replay target is `state_clause` (`VersionUnchanged`) each round-trip with that one target unchanged. The reader refuses a request with no target `invalid_runtime_input`/`missing-member` at `/replay`, one with `function` and `frame_operation` `invalid_runtime_input`/`invalid-value` at `/replay/frame_operation`, one with `function` and `state_clause` the same at `/replay/state_clause`, and the frame request carrying `arguments` `invalid_runtime_input`/`unknown-member` at `/replay/arguments`, each with a byte-provision entry removed changing none of these refusals (QSpec FR-323-AC-19). | Test (TC-185) |
| FR-071-AC-15 | A `function` request carrying the `declaration` occurrence key of its function, and one carrying an `expression` occurrence key at ordinal 1, each round-trip with the key unchanged; the same request without `occurrence_key` refuses `invalid_runtime_input`/`missing-member` at `/replay/occurrence_key`, and refuses so with a byte-provision entry removed. | Test (TC-185) |
| FR-071-AC-16 | A `ModelGraph` request with no target members decodes with no replay target and uses its envelope identity; the same source with a function target, arguments, occurrence key or request `over` refuses `invalid_runtime_input`/`unknown-member`; a non-`ModelGraph` request with no target still refuses `missing-member` at `/replay`. | Test (TC-595) |

## Dependencies

- **Upstream**: [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md);
  ADR-013 O-26, O-11 (`QualifiedName`), QC-1 (byte
  provision); ADR-011 §4 (dependency binding, E4/E9), §2.1 E9; QSpec
  [FR-323](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/interfaces/FR-323-native-runtime-envelope.md)
  (Accepted complete-V1 boundary value contract) AC-5 (`byte_provision`) and
  AC-6 (`replay` property shape) supply the normative wire this type
  represents.
- **Shared types**: #213 (ARCH-20) owns the O-11 `QualifiedName` type this
  request's function-selection member reuses; this requirement adds no
  parallel name type.
- **Downstream**: agent-ix/quire-contract-codegen#50 builds this request
  from the IR packet (ADR-013 C-12); the QSL replay executor (#243, TK-01,
  ADR-013 C-13) consumes it to recompile and select the function; recompilation and the
  function call itself are explicitly out of this
  requirement's scope. Its QSpec FR-323 `byte_provision` entries
  are keyed by labels or document identity.
