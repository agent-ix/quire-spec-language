---
id: FR-073
title: "Implement redacted, safe diagnostic rendering for the proof, witness and replay envelopes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: traces_to
---
# FR-073: Implement redacted, safe diagnostic rendering for the proof, witness and replay envelopes

## Description

#231's Scope explicitly lists "redaction/safe-diagnostic behavior" beside
canonical serialization, version rejection and resource bounds. ADR-011
FB-02 already establishes the general principle this requirement makes
concrete for these four types: a diagnostic's rendered text is output for
people, never a channel a consumer reads meaning from (ADR-011 FB-02, and
ADR-013 R-05's "no consumer derives semantic identity from display text").
This requirement is the converse obligation FB-02 and R-05 do not by
themselves cover: not that a consumer must not *read* rendered text as
meaning, but that the four envelope types' own `Debug`/`Display` rendering,
and the `Display`/`Debug` rendering of any refusal cause naming their
content, must not *place* bulk or sensitive content — the witness
transcript, a byte-provision entry's raw bytes, or a full concrete-value
payload — into that rendered text at all. Without this, a log line or
panic message built from an ordinary `{:?}` of a witness envelope or a
replay request would reproduce the full transcript or dependency source
bytes verbatim, which is exactly the kind of uncontrolled diagnostic
channel FB-02's rule exists to keep out of the system, only in the
opposite direction (writing instead of reading).

This requirement changes no decoding, admission or comparison behavior
built by FR-069 through FR-072; it constrains only what those types' own
`Debug`/`Display` implementations, and their refusal causes' rendering,
are allowed to contain. It does not remove or gate the typed accessors
(`transcript()`-derived methods, byte-provision entry lookup, concrete
value accessors) those requirements already define: redaction applies to
implicit rendering only, never to an explicit, typed call a caller makes
on purpose.

## Inputs

- A constructed or refused instance of the proof-result envelope (FR-069),
  the counterexample/witness envelope (FR-070), the replay request
  (FR-071), or the replay result (FR-072).

## Outputs

- That instance's `Debug` and `Display` rendering, and the `Debug`/
  `Display` rendering of any structured refusal cause naming its content.

## Behavior

- The witness envelope's (FR-070) `Debug` implementation SHALL NOT include
  the full transcript text in its rendered output, and its `Display`
  implementation, if any, SHALL NOT include the full transcript text
  either; either implementation SHALL render a bounded descriptor instead
  (a digest, byte length, or a fixed-length excerpt below the reader's
  configured bound) wherever it names the transcript.
- The replay request's (FR-071) and replay result's (FR-072) `Debug`
  implementations SHALL NOT include a byte-provision entry's full raw
  bytes or a concrete argument/value payload's full content in their
  rendered output, and their `Display` implementations, if any, SHALL NOT
  include that content either; each implementation SHALL render such an
  entry's digest or a bounded descriptor only.
- FR-069 through FR-072 SHALL type every refusal-cause field they define
  (including the fields of the `stale_dependency`/`byte-digest-mismatch`,
  `stale_dependency`/`digest-domain-mismatch`, and bound-exceeded causes)
  to hold only a digest, a locus, a category, or another bounded
  descriptor, so no refusal cause's `Debug`/`Display` rendering can expose
  unredacted transcript, byte-provision, or full concrete-value content.
- Redaction SHALL apply only to `Debug`/`Display` rendering. Every typed
  accessor FR-069 through FR-072 already define (`transcript()`-derived
  methods, byte-provision entry lookup by digest, concrete-value
  accessors) SHALL continue to return its full, unredacted content when a
  caller invokes it directly; this requirement adds no access control and
  makes no content unreachable, only unrendered by default.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-073-AC-1 | Given a constructed witness envelope (FR-070) whose transcript names a specific concrete value, neither its `Debug` nor its `Display` rendering contains the full transcript text; both contain only a bounded descriptor. | Test (TC-209) |
| FR-073-AC-2 | Given a constructed replay request (FR-071) whose byte provision carries a multi-kilobyte source entry, neither its `Debug` nor its `Display` rendering contains the entry's full raw bytes; both contain only the entry's digest or a bounded descriptor. | Test (TC-210) |
| FR-073-AC-3 | Given a refusal produced by any of FR-069 through FR-072 that names transcript, byte-provision, or concrete-value content in its typed cause, the cause's `Debug`/`Display` rendering contains no unredacted instance of that content, and the same content remains fully readable through its typed accessor called directly on the pre-refusal or referenced value. | Test (TC-211) |

## Dependencies

- **Upstream**: [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md),
  [FR-071](FR-071-implement-typed-replay-request.md),
  [FR-072](FR-072-implement-typed-replay-result.md); ADR-011 FB-02 (no
  consumer branches on, or is fed by, rendered diagnostic text); ADR-013
  R-05 (no consumer derives semantic identity from display text); #231
  Scope ("redaction/safe-diagnostic behavior").
- **Downstream**: none; this requirement constrains rendering only and is
  not itself a dependency of any other #231 requirement.
