---
id: FR-024
title: "Read exact native runtime artifacts"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-023
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: references
---
# FR-024: Read exact native runtime artifacts

## Description

When a caller supplies serialized native runtime bytes and an exact snapshot or invocation reference, the reader shall return a structurally checked artifact bound to those selected bytes.

## Inputs

Borrowed input bytes, a role-specific SnapshotRef or InvocationRef, and existing
ArtifactLimits. The admitted envelope is native-state-input/1, with the exact
fields and value variants defined in the native runtime input contract.

## Outputs

The existing immutable Snapshot or Invocation, or a typed read error retaining
the expected runtime reference, failed stage and original JSON/construction cause.
JSON coordinates are relative to their stated envelope or body decode stage.
Byte-limit, digest, version, artifact-kind and identity refusals have distinct
typed causes with the observed value or effective ceiling. Envelope and body
JSON failures are separate variants retaining the original Serde error. The
cause determines its stage and code through one exhaustive classification.

## Behavior

The reader shall bound input bytes before hashing or decoding using the clamped
artifact-byte ceiling.
The reader shall verify the selected digest before interpreting JSON.
The reader shall use Serde to decode a closed envelope with all fields present.
The reader shall reject an unknown version or foreign artifact kind before typed
body decoding, using unknown_wire or invalid_runtime_input respectively.
The reader shall require envelope identity to equal the selected reference.
The reader shall reject unknown, missing or duplicate fields throughout the body,
invalid identifiers, non-integer numeric values and non-lowercase 64-digit digests.
Each public runtime record's Serde decoder shall enforce its closed object shape
and validated scalar fields when decoded directly or nested, without requiring
the caller to select a field adapter. An invocation result field shall be present;
explicit null decodes to no result. Native digest fields shall admit exactly
64 lowercase hexadecimal digits without an algorithm prefix.
The reader shall preserve explicit null operation results and duplicate vector entries.
The reader shall run the existing structural constructor before admitting an artifact.
The reader shall retain the original input bytes and their digest on success.
The artifact constructor shall own adoption of the original external bytes after structural validation.

Whitespace and object field order may differ from the constructor's encoding.
ArtifactUsage continues to describe the actual construction pass, including its
encoded byte count; it is not a count of the retained external bytes. Input byte
preflight and construction apply the same ceiling independently. Structural
limits retain resource_exhausted; digest/identity mismatches use stale_dependency;
malformed JSON or body data use invalid_runtime_input. No failed read yields a
partial artifact. Serde's default recursion limit remains enabled.

Reading does not validate model roles, population closure, operation frames or
predicate truth. It defines no portable reference/result envelope and performs no I/O.

The compiler shall publish a local Draft 2020-12 schema for native-state-input/1
covering both envelope kinds, every value variant, closed required records,
identifier/digest spelling and representable integer/index/revision ranges.
The schema shall preserve explicit null results and duplicate vector entries.
Schema validation operates on parsed JSON data; it does not establish duplicate
raw-key rejection, lexical integer spelling or precision lost by a caller's JSON
parser. Selected byte identity, structural budgets/arena correspondence and
model-aware execution remain reader/constructor/runtime responsibilities.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-024-AC-1 | Snapshot and invocation bytes round-trip into artifacts with exact selected references, bytes and flat drafts, including whitespace and reordered object fields. | Test |
| FR-024-AC-2 | Stale digests/identities, foreign roles, unknown versions and malformed/unknown/missing/duplicate JSON fields refuse with the actual stage and a distinct typed cause; byte-limit causes retain the actual length and effective ceiling, and envelope/body errors retain their original JSON diagnostics. | Test |
| FR-024-AC-3 | Every admitted value variant decodes exactly; direct and nested decoding of each public runtime record rejects positional arrays and unknown, missing or duplicate fields. Invalid identifier, numeric, digest and local-index values refuse while vector duplicates remain intact; explicit null invocation results are admitted but omitted results refuse. | Test |
| FR-024-AC-4 | Byte and structural limits return incomplete without partial artifacts; fresh reads can succeed and actual execution of reread snapshots/invocations preserves truth and frame refusals. | Test |
| FR-024-AC-5 | The local schema accepts actual snapshot/invocation encodings and all ten value variants, preserves null results and vector duplicates, and rejects wrong envelope/body pairing, malformed/unknown/missing record fields and invalid scalar ranges/spellings. Controls demonstrate schema-valid data still refused by byte selection, exact wire decoding or structural construction. | Test |

## Dependencies

- [FR-018](FR-018-construct-native-runtime-inputs.md): structural constructors and input encoding.
- [FR-023](FR-023-run-native-packages.md): native execution of admitted artifacts.
- [FR-020](FR-020-read-and-rebind-native-packages.md): shared closed JSON record adapter.
- [Input contract](../../docs/native-runtime-inputs.md): complete native-state-input/1 fields.
