---
id: FR-259
title: "Encode identities and read JSON through quire-canonical at any depth"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
---
# FR-259: Encode identities and read JSON through quire-canonical at any depth

## Description

QSL SHALL produce every canonical byte string through `quire-canonical`, the
one RFC 8785 implementation (ADR-013 §2), and SHALL read every untrusted JSON
document that has no typed reader through `quire-canonical`'s shared reader
(ADR-030 D-4.4, D-4.5). Neither path SHALL bound depth. This requirement
states how QSL's identities, digests and reads use `quire-canonical`.

## Capabilities QSL relies on

`quire-canonical` is a separate repository and specifies its own crate.
QSL relies on these capabilities, which are a `quire-canonical` follow-up
(ADR-030 D-8 O-6):

- canonical encoding bounded by bytes only, whose byte error is distinct
  from every malformed-input error;
- encoding of data whose depth follows the input, driven from the caller's
  explicit stack (the event API);
- a reader of untrusted JSON into a tree whose traits do not recurse,
  refusing malformed input with its byte offset, under an input byte limit,
  and an encoder for that tree;
- a serde encoding path that only fixed-depth types can take.

## Behavior

1. **Fixed-depth identities.** QSL SHALL encode every identity preimage whose
   depth is fixed by its schema (node keys, compound-unit identities, the
   checked and v2 package identities, type and enum preimages) through
   `quire-canonical`'s fixed-depth serde path.
2. **Input-depth identities and digests.** QSL SHALL encode every identity or
   digest over data whose depth follows the input (a simulation state key, a
   replay value, an intake or observation document) through the event API or
   the tree encoder, driven from an explicit heap stack.
3. **Identity byte limit.** QSL SHALL encode each identity under the
   caller's `identity.input_bytes` limit (FR-255), a byte limit only, used
   as given with no ceiling. `IDENTITY_LIMITS` is its published default,
   16777216 bytes. A site whose stage applies its own byte budget SHALL pass
   that budget instead.
4. **Byte error mapping.** When `quire-canonical` returns its byte error for
   an encoding or a read, QSL SHALL report the calling stage's input-bytes
   limit outcome, naming its bound, the count reached and its setting
   (FR-255), and SHALL NOT report it as a malformed value or malformed
   input.
5. **Shared reader.** QSL SHALL read untrusted JSON without a typed reader
   through `quire-canonical`'s shared reader, and SHALL report the reader's
   malformed-input refusal as a malformed-input refusal of the calling site,
   carrying the byte offset. Observation digest admission is exempt: it
   digests the bytes the reader refuses raw and never refuses them as
   malformed (FR-106 check 1.3).
6. **Allocation failure.** When `quire-canonical` cannot reserve memory for
   a read or an encoding, QSL SHALL refuse with
   `resource_exhausted`/`allocation-failed`, carrying `requested`, the size
   in bytes of the reservation that failed. It is not a limit: it names no
   bound and no setting. QSL SHALL NOT report it as a limit outcome, a
   malformed value or malformed input. Domain package intake (FR-056),
   observation digest admission (FR-106 check 1) and package identity
   (FR-261) report this one outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-259-AC-1 | The checked package identity, every node key and the v2 package identity of a package holding a 100,000-term sum, minted on a thread with a 512 KiB stack under limits raised to fit it, equal those minted from the same source on a thread with an 8 MiB stack. | Test (TC-728) |
| FR-259-AC-2 | A declaration whose preimage is longer than `s3.input_bytes` stops with `stage_limit_exceeded`/`input-bytes-exceeded` naming that bound, the preimage length and setting `s3.input_bytes`. When `quire-canonical` returns its byte error to a digest site of FR-260 or FR-261, the site reports its own input-bytes limit with its setting, and no malformed-input cause. | Test (TC-728) |
| FR-259-AC-3 | On a thread with a 512 KiB stack, the simulation state key QSL mints for a 100,000-long recursive list value, and the `sha256-jcs` digest QSL computes for a 100,000-deep JSON package document at intake, each equal the value minted on a thread with an 8 MiB stack, and the digest equals the SHA-256 of the document's RFC 8785 text. | Test (TC-729) |
| FR-259-AC-4 | The deepest JSON array or object nesting is the same at both depths of each pair below, measured over every node preimage of the checked package (node keys, type nodes and nominal preimages) and over the emitted v2 package's identity preimage: each FR-258 expression form at 4 and at 63 levels; a parameter typed with 4 and with 100 nested `Option`s; a chain of 4 and of 30 records. Compound-unit ids, enum declaration and member preimages and the checked package identity are flat term, member or selection lists whose depth is fixed by their types, so no source depth reaches them. | Test (TC-728) |
| FR-259-AC-5 | An allocation failure of 4096 requested bytes from `quire-canonical`'s reader or encoder refuses `resource_exhausted`/`allocation-failed` carrying 4096 at domain package intake, at observation digest admission and at package identity, and none of them reports a limit, a malformed value or malformed input. | Test (TC-729) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.4, D-4.5 and D-8 overlap item O-6.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §2: one RFC 8785 implementation.
- [FR-258](FR-258-check-and-lower-expressions-at-any-depth.md) makes every
  node-key preimage fixed-depth.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) names the
  settings a byte error reports.

## Overlap

The capabilities listed above, and the removal of the crate's depth limit,
are `quire-canonical`'s requirements, a `quire-canonical` follow-up
(ADR-030 D-8 O-6, ruling RU-2). This requirement's tests exercise QSL's
behaviour over them.

## References

- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.
