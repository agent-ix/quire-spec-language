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
that is exactly the ADR-013 O-25 counterexample packet plus the FR-070
envelope's members — the state environment, the `quire.value.accounting/v1`
limits, the S1-to-S4 stage limits copied from the proving run, and the
optional object environment below — and nothing else. The type SHALL make it structurally impossible to obtain a
recompilation input other than by digest: it SHALL carry a digest-addressed
byte provision (QSpec FR-323 `byte_provision`, ADR-013 QC-1) and SHALL have
no field, accessor or constructor argument typed as a filesystem path,
environment variable name, or search location.

The request selects the item to replay by its replay target: exactly one
QSpec FR-323 replay target member, typed as FR-323 types that member (a
function's `QualifiedName`, ADR-013 O-11 and the OQ-5 ruling; a frame's
`{object, operation}`, a `QualifiedName` and an `Identifier`; a state
clause's own declared `QualifiedName`), never by a bare string. This requirement builds
the request type and its round trip only; it does not build the executor
that consumes it (ADR-013 C-13, TK-01 executor entry, is #243's), and it
performs no recompilation, no `package_id` recomputation, and no function
call. The executor is [FR-098](FR-098-execute-a-replay-request.md).

## Inputs

- An ADR-013 O-25 counterexample packet.
- The FR-070 envelope's state environment, accounting limits, and S1-to-S4
  stage limits copied from the proving run.
- Optionally, the object environment of a replay whose function takes a
  `Reference`: the reference to one `quire.state.snapshot/v1` document
  (FR-106) in the byte provision.
- The digest-addressed byte provision (QSpec FR-323 `byte_provision`): every
  source, definition and domain-package input the replay would recompile,
  each keyed by its declared digest domain and digest.

## Outputs

- A typed replay request carrying exactly the members above, or a
  structured refusal when the input is malformed or names an
  out-of-domain digest or profile identifier.

## Behavior

- The request type SHALL carry exactly the ADR-013 O-26 members (the O-25
  packet plus the FR-070 envelope members) and SHALL invent no additional
  member; a construct → serialize → read round trip SHALL preserve the
  package reference (`package_id`, every `RawSourceRef`
  digest), the replay members (`ReplaySource`, `QualifiedName`, arguments
  keyed by parameter node id, profile selections, proof bounds and declared
  domains, trace position, `backend`), the byte provision, and the limits
  exactly.
- Every recompilation input SHALL be reachable from the request only by
  digest lookup in its byte provision; the request type SHALL define no
  path-typed, environment-variable-typed, or search-location-typed member,
  and a byte-provision entry naming a digest domain outside the closed
  FR-201 domain set SHALL refuse at decode.
- The package reference SHALL carry QSpec FR-323's `sources` and
  `dependencies`: `sources` names the proved package's own source
  references, and `dependencies` one entry per entry of the proved
  package's `dependency_selections`, in order, each
  `{identity, package_id, sources}` (ADR-015 D-4). `identity` is a
  `LibraryName` and `package_id` a `quire.package.semantic/v2`
  `DigestRecord`; an empty identity, or a `package_id` in
  another domain, refuses at decode. Every source reference of every entry
  is a `RawSourceRef` the byte provision SHALL cover, as the proved
  package's are.
- The package reference and each `dependencies` entry SHALL carry an
  optional `refinement_internals` member (QSpec FR-323), a JSON value kept
  exactly as decoded. The decoder SHALL NOT validate it; an absent member is
  the empty list, and its defects surface from the recompile (FR-098,
  FR-147).
- Each package-reference source entry SHALL carry FR-001's two source labels
  (authority and identity) and its digest (QSpec STD-150), so the executor
  (FR-098) recompiles a source under exactly the labels the proving run
  used (ADR-013 §7 S-4b).
- A byte-provision entry SHALL be verified under its declared domain: a
  raw-byte-addressed domain (source and definition documents) against the
  SHA-256 of its bytes, and `sha256-jcs` (a domain package document)
  against the SHA-256 of the document's RFC 8785 bytes, the digest I1 keys
  its package input by. Any other FR-201 domain refuses as ineligible.
  A `sha256-jcs` entry is read through intake's one read
  (`PackageDocument::parse`, FR-260), and a refusal of that read keeps its
  cause: bytes it cannot read refuse
  `invalid_model_binding`/`malformed-declaration`; bytes over an intake
  limit refuse with intake's limit outcome,
  `resource_exhausted`/`intake-limit-exceeded`, naming the limit and its
  bound (FR-056); a read or digest that cannot reserve memory refuses
  `resource_exhausted`/`allocation-failed`, carrying the bytes requested
  (FR-259 B6); and a document holding a number with no exact RFC 8785
  spelling refuses `noncanonical_wire` with intake's cause
  (`inexact-integer` or `inexact-number`) and `document_pointer` (FR-056).
  **Amended**: `sha256-jcs` used to refuse as ineligible, which
  left no way for a domain package to reach the executor.
- A present `object_environment` member SHALL be a closed object with exactly
  `identity` and `digest`, in the form FR-106's invocation uses for its `pre`
  and `post` members: the identity labels of a snapshot document and its
  `sha256-jcs` digest. The `identity` is the label set the result retains with the
  digest (FR-122); admission selects the document by digest alone and does not
  compare it with the document's own `identity`. The member names the document, whose bytes are a
  `sha256-jcs` entry of the byte provision like every other recompilation
  input. A request without the member carries no object environment.
- The request reader SHALL NOT read the `object_environment` document nor
  require its byte-provision entry at construction: the executor admits the
  document by FR-106 (FR-098), and the request defines no second document
  grammar.
- The reader SHALL refuse a malformed `object_environment` at decode, naming
  the member or the member inside it, with `invalid_runtime_input`:
  `wrong-value-kind` for a member that is not an object, or an `identity` or
  `digest` of the wrong JSON kind; `missing-member` for a missing `identity`
  or `digest`; `unknown-member` for any other member, naming the first in
  document order; and `invalid-value` for a `digest` that is not in the
  `sha256-jcs` domain or whose hex is not 64 lowercase digits.
- The byte provision SHALL be complete at construction: every
  `RawSourceRef` digest the package reference names SHALL have a matching
  byte-provision entry; a request whose package reference names a digest
  with no corresponding entry SHALL refuse at construction rather than
  admitting an incomplete provision for a later consumer to discover.
- A byte-provision entry's stored bytes SHALL hash, under its declared
  digest domain's algorithm, to exactly its declared digest; an entry whose
  bytes do not match their own declared digest SHALL refuse at construction
  with cause `stale_dependency`/`byte-digest-mismatch` carrying the declared digest and the digest of the raw bytes, and a `sha256-jcs` entry whose document recomputes to another digest SHALL refuse with `stale_dependency`/`content-mismatch` carrying both digests. This is this
  ticket's own half of "stale package identity": it is a decode-time byte
  vs. declared-digest integrity check that needs no recompilation. The
  complementary half — recomputing `package_id` from recompiled source and
  requiring it to equal the request's declared `package_id` — needs an
  actual compile and is #243's (TK-01 executor, ADR-013 C-13), not this
  requirement's.
- When a semantic profile selection names a profile outside the closed
  known set, the reader SHALL refuse the request at decode with
  `unknown_profile`/`unsupported-selection`, keeping the supplied selection
  and the role it was supplied for, and SHALL read no byte-provision entry
  for it.
- The reader SHALL refuse a request whose encoded size exceeds the
  configured reader bound, and SHALL NOT return a truncated or
  partially-populated request in that case.
- The replay target SHALL hold typed names only (`QualifiedName`,
  `Identifier`); the request's public constructors and decoders SHALL NOT
  accept a bare `&str` or `String` in any replay target position.
- The request type SHALL provide no implicit string-to-`QualifiedName` or
  string-to-`Identifier` conversion.
- The reader SHALL refuse a request by QSpec FR-323's decode rule 1 (no
  replay target member, more than one, or a non-function target with
  `arguments` or an occurrence key), with FR-323's codes and pointers,
  before any byte-provision rule.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-071-AC-1 | The request carries exactly the O-26 members, invents none, and a construct → serialize → read round trip preserves the package reference, replay members, byte provision and limits exactly. | Test (TC-185) |
| FR-071-AC-2 | The request type has no path-, environment-variable-, or search-location-typed field or accessor; a byte-provision entry whose digest domain is outside the closed FR-201 set refuses at decode. | Test (TC-186) |
| FR-071-AC-3 | No public constructor or decoder of the request accepts a bare `&str` or `String` for function selection, and no implicit conversion from a string to `QualifiedName` exists; an attempted call site passing a string literal in that position fails to compile. | Test (TC-187) |
| FR-071-AC-5 | A request whose package reference names a `RawSourceRef` digest with no matching byte-provision entry refuses at construction with `missing_import`/`missing-selection`, naming the requested digest record (domain and digest) and where the package reference names it; no incomplete request is returned for a later consumer to discover the gap. | Test (TC-186) |
| FR-071-AC-6 | A byte-provision entry whose stored bytes do not hash to its own declared digest, under its declared digest domain's algorithm, refuses at construction with cause `stale_dependency`/`byte-digest-mismatch` carrying the declared digest and the raw bytes' own digest (an entry under a raw-byte domain) or `stale_dependency`/`content-mismatch` (a `sha256-jcs` entry, by its recomputed digest); this is this requirement's own decode-time half of "stale package identity" and is distinct from #243's execution-time recompiled-`package_id` check. | Test (TC-186) |
| FR-071-AC-7 | A request whose encoded size exceeds the configured reader bound refuses, and no truncated or partially-populated request is returned. | Test (TC-186) |
| FR-071-AC-9 | A request whose package reference carries two `dependencies` entries round-trips them exactly, in order; an entry whose source digest has no byte-provision entry refuses at construction, as AC-5 states; an entry with an empty identity, or a `package_id` in the `quire.source.bytes/v1` domain, refuses at decode. | Test (TC-186) |
| FR-071-AC-10 | A request whose semantic profile selection names a profile outside the closed known set refuses at decode with `unknown_profile`/`unsupported-selection`, keeping the supplied selection and its role, and refuses so even when a byte-provision entry's bytes do not match its digest: the byte provision is not read first. | Test (TC-186) |
| FR-071-AC-11 | A `sha256-jcs` byte-provision entry holding a repeated member name refuses `invalid_model_binding`/`malformed-declaration`; an entry intake refuses at one of its limits refuses `resource_exhausted`/`intake-limit-exceeded`, naming that limit and its bound; an allocation failure of 4096 requested bytes while reading or digesting an entry refuses `resource_exhausted`/`allocation-failed` carrying 4096; and an entry holding `18446744073709551616` at `/package/count` refuses `noncanonical_wire`/`inexact-integer` with `document_pointer` `/package/count`, and one holding `0.1000000000000000000001` at `/a~1b/0` refuses `noncanonical_wire`/`inexact-number` with `document_pointer` `/a~1b/0`. | Test (TC-186) |
| FR-071-AC-12 | A request whose `object_environment` is `{"digest": <sha256-jcs digest>, "identity": {...}}` round-trips it exactly. Each of these refuses at decode with `invalid_runtime_input`, naming the member: a string for `object_environment` (`wrong-value-kind`); an object without `digest` (`missing-member` naming `object_environment.digest`); a third member (`unknown-member`); a `digest` in the `quire.source.bytes/v1` domain, and a `sha256-jcs` digest of 63 hex digits (each `invalid-value`). A request whose `object_environment` digest has no byte-provision entry constructs: the entry is read at admission, not at decode. | Test (TC-1042) |
| FR-071-AC-13 | A request whose package reference carries `refinement_internals` `[{"refinement": "CasTwice", "internal": ["Spec::Twice::finish"]}]` and whose `dependencies` entry carries its own value round-trips both exactly; a request with the member absent round-trips with it absent; the values `{}`, `null` and `7` decode and round-trip unchanged. | Test (TC-186) |
| FR-071-AC-14 | A request whose replay target is `function`, one whose replay target is `frame_operation` (`Config::ConfigVersion`, `attemptUpdate`) and one whose replay target is `state_clause` (`VersionUnchanged`) each round-trip with that one target unchanged. The reader refuses a request with no target `invalid_runtime_input`/`missing-member` at `/replay`, one with `function` and `frame_operation` `invalid_runtime_input`/`invalid-value` at `/replay/frame_operation`, one with `function` and `state_clause` the same at `/replay/state_clause`, and the frame request carrying `arguments` `invalid_runtime_input`/`unknown-member` at `/replay/arguments`, each with a byte-provision entry removed changing none of these refusals (QSpec FR-323-AC-19). | Test (TC-185) |

## Dependencies

- **Upstream**: [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md);
  ADR-013 O-26, O-11 (`QualifiedName`), QC-1 (digest-addressed byte
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
  ADR-013 C-13) consumes it to recompile, recompute `package_id` from the
  recompiled source, and select the function — that recompilation-time
  staleness check, and the function call itself, are explicitly out of this
  requirement's scope; this requirement owns only the decode-time
  byte-vs-declared-digest integrity check (AC-6).
