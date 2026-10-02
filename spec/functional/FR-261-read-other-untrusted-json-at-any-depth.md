---
id: FR-261
title: "Read library preimages and observation documents at any depth"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: traces_to
---
# FR-261: Read library preimages and observation documents at any depth

## Description

Every QSL read of untrusted JSON that has no typed reader SHALL go through
`quire-canonical`'s shared reader and its arena tree (FR-259), under the
calling stage's input byte limit, and SHALL report the document on its
content whatever its nesting (ADR-030 D-4.10). The sites are:

- the library package identity read, which validates an identity preimage
  and projects its declarations, under `library.single_artifact_bytes`;
- observation digest admission, which reads a snapshot or invocation
  document's digest, under `observation.input_bytes`;
- the observation document reader (FR-106), under `observation.input_bytes`,
  `observation.objects` and `observation.values`.

## Behavior

1. **Shared reader.** Each site SHALL read its document through
   `quire-canonical`'s shared reader into its arena tree.
2. **Malformed versus limit.** When the reader refuses the bytes as
   malformed, the site SHALL return its malformed-input refusal carrying the
   reader's byte offset. When the bytes exceed the site's input byte limit,
   the site SHALL return its limit outcome naming the limit, its bound, the
   length and the setting (FR-255). A site SHALL NOT report either as a
   document that is not an object, or as a document with no digest.
3. **Content.** When the reader admits the document, the site SHALL judge it
   by its members and values, through the checks its own requirement
   defines.
4. **Observation counts.** The observation document reader SHALL bound a
   document by `observation.objects` and `observation.values`, each charged
   as the reader's tree is walked over an explicit heap stack.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-261-AC-1 | On a thread with a 512 KiB stack, the library package identity read of a canonical preimage object whose members are valid except that `edition` holds an array nested 100,000 deep refuses with `PreimageDefect::MemberType("edition")`, not `PreimageDefect::NotObject`. Bytes that are not JSON refuse with the malformed-input defect carrying the byte offset of the first malformed byte, distinct from the not-an-object defect, which a top-level array gives. | Test (TC-733) |
| FR-261-AC-2 | On a thread with a 512 KiB stack, observation digest admission over a snapshot document holding a field value nested 100,000 deep, within `observation.input_bytes`, reads the document's digest and proceeds to FR-106's later checks; the same document with a digest that differs from the selection refuses `stale_dependency`/`content-mismatch`. | Test (TC-733) |
| FR-261-AC-3 | On a thread with a 512 KiB stack, the observation document reader admits a snapshot whose population's field holds a recursive value 100,000 levels deep, with `observation.input_bytes` and `observation.values` raised to fit and the field's declared type admitting that value. With `observation.values` one below the document's value count, it refuses naming `observation.values`, its bound and the count reached, and admits once the setting is raised through `ObservationLimits`' builder. | Test (TC-733) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.10.
- [FR-259](FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
  defines the shared reader QSL consumes.
- [FR-106](FR-106-admit-snapshots-and-invocations.md) defines observation
  admission and its checks.
- [FR-111](FR-111-link-a-complete-v1-definition-bundle.md) defines the
  library limits.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) names
  each setting.

## References

- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.
