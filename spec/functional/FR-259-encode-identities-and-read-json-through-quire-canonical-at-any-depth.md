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
states how QSL's identities, keys and reads use `quire-canonical`.

## Capabilities QSL relies on

`quire-canonical` is a separate repository and specifies its own crate.
QSL relies on these capabilities, which `quire-canonical` specifies and
delivers (ADR-030 D-8 O-6):

- canonical encoding bounded by bytes only, whose byte error is distinct
  from every malformed-input error;
- encoding of data whose depth follows the input, driven from the caller's
  explicit stack (the event API);
- a reader of untrusted JSON into a tree whose traits do not recurse,
  refusing malformed input with its byte offset, under an input byte limit,
  and an encoder for that tree;
- a serde encoding path that only fixed-depth types can take.

## Behavior

1. **Structural identities.** QSL SHALL NOT encode a node key or any
   other structural identity to a canonical byte string: a node key is the
   triple `{package, path, ordinal}` and an obligation identity the triple
   (node, model, ordinal), each compared, ordered and hashed componentwise.
2. **Input-depth keys.** QSL SHALL encode every key over data whose depth
   follows the input (a simulation state key, a replay value, an intake
   document) through the event API or the tree encoder, driven from an
   explicit heap stack.
3. **Key byte limit.** QSL SHALL encode each such key under the caller's
   `identity.input_bytes` limit (FR-255), a byte limit only, used as given
   with no ceiling. Its published default is 16777216 bytes. A site whose
   stage applies its own byte budget SHALL pass that budget instead.
4. **Byte error mapping.** When `quire-canonical` returns its byte error for
   an encoding or a read, QSL SHALL report the calling stage's input-bytes
   limit outcome, naming its bound, the count reached and its setting
   (FR-255), and SHALL NOT report it as a malformed value or malformed
   input.
5. **Shared reader.** QSL SHALL read untrusted JSON without a typed reader
   through `quire-canonical`'s shared reader, and SHALL report the reader's
   malformed-input refusal as a malformed-input refusal of the calling site,
   carrying the byte offset.
6. **Allocation failure.** When `quire-canonical` cannot reserve memory for
   a read or an encoding, QSL SHALL refuse with
   `resource_exhausted`/`allocation-failed`, carrying `requested`, the size
   in bytes of the reservation that failed. It is not a limit: it names no
   bound and no setting. QSL SHALL NOT report it as a limit outcome, a
   malformed value or malformed input. Domain package intake (FR-056)
   and the observation document reader (FR-261) report this one outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-259-AC-1 | The checked package of a package holding a 100,000-term sum, checked on a thread with a 512 KiB stack under limits raised to fit it, and its node keys, whose paths nest an operand's node key in each sum node's key, are equal to those checked from the same source on a thread with an 8 MiB stack. | Test (TC-728) |
| FR-259-AC-2 | **RETIRED (node preimages are deleted).** A node key is the structural triple `{package, path, ordinal}`; no declaration preimage is encoded, so no identity byte limit applies to one. | Test (TC-728), retired |
| FR-259-AC-3 | On a thread with a 512 KiB stack, the simulation state key QSL mints for a 100,000-long recursive list value, and the intake read of a 100,000-deep JSON package document, each equal the result on a thread with an 8 MiB stack, and the state key equals the RFC 8785 text of the state. | Test (TC-729) |
| FR-259-AC-4 | **RETIRED (node preimages are deleted).** A node key is the structural triple `{package, path, ordinal}`; there is no node or package identity preimage whose nesting depth could be measured. | Test (TC-728), retired |
| FR-259-AC-5 | An allocation failure of 4096 requested bytes from `quire-canonical`'s reader or encoder refuses `resource_exhausted`/`allocation-failed` carrying 4096 at domain package intake and at the observation document reader, and none of them reports a limit, a malformed value or malformed input. | Test (TC-729) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.4, D-4.5 and D-8 overlap item O-6.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §2: one RFC 8785 implementation.
- [FR-258](FR-258-check-and-lower-expressions-at-any-depth.md) checks and
  lowers expressions at any depth.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) names the
  settings a byte error reports.

## Overlap

The capabilities listed above, and the removal of the crate's depth limit,
are `quire-canonical`'s requirements
(ADR-030 D-8 O-6, ruling RU-2). This requirement's tests exercise QSL's
behaviour over them.

## References

- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.
