---
id: FR-031
title: "Run selected Markdown clauses from files"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
---
## Description

When a native-run/1 request selects program extraction in an enabled build, the command shall compile its selected Markdown clause through FR-030 and execute the existing runtime workflow.

The native-run/1 code still reads and emits the body record's old
`revision_namespace`, `revision` and `formal_revision` fields and no `digest`;
it moves to the `{authority, identity, document, digest}` record above.

## Inputs

With the quire-extraction feature, program accepts an optional closed extraction
record containing body: {authority, identity, document, digest}. The
ordinary program source selects the digest-verified original Markdown document;
its identity must satisfy Quire's existing SourceLocus schema.
The body record assigns its derived source a native identity, `authority` and
`identity` with `digest`, and a formal identity, `document` with `digest`. Each
is a document identity plus a content digest, the key QSpec FR-202 gives an
authored source (STD-150): `digest` is the raw-artifact digest of the derived
body's complete bytes (QSpec FR-018). The command recomputes that digest over
the extracted body and refuses a body record whose `digest` differs with
`stale_dependency`/`content-mismatch`, naming both digests. Exactly one authored clause binding selects the Quire heading and native
clause. Omission preserves ordinary native-source execution. Supplied null or
positional extraction/body records, duplicate and unknown fields refuse.
The decoded native/formal pair is explicit, and each identity is a document
identity plus the content digest of that document's bytes.

This local clause-only mode constructs a Quire context, validated by content,
for the explicitly selected binding's package and original path/identity, with empty exports/imports and a Markdown target. It does
not load installed archetype schemas or infer model meaning from Markdown.
Native model imports retain their existing authority. C's installed-module and
existing-system adoption remains separate.

## Outputs

The existing native-run-result/1 report includes an extraction member retaining
original source identity/digest/formal identity, unchanged Quire availability,
clauses, raw clause_text and diagnostics, plus verified byte-map region/segments.
The ordinary source and native diagnostic/event coordinates name the body; the
map links those bytes to the original. Only completed execution has truth.
Extraction/compilation failures retain original source, authored selection,
completed Quire output, native diagnostic and verified original spans when present.
Other file/model/runtime-input intake failures keep FR-026's existing envelope.
Unsupported combinations have distinct catalogued codes: extraction-package-conflict,
extraction-clause-count (with actual count), and extraction-requires-run. A Quire
context failure uses invalid-quire-context and retains the unchanged producer
diagnostics. The extraction report is
constructed through typed serialization before the immutable native result is exposed.

## Behavior

The command shall use existing bounded file intake, model frontend, FR-030 consumer and runtime execution.
The command shall validate the fixed clause-only context through Quire's public semantic-block validator.
The command shall validate and retain exactly one authored binding before dependent file I/O.
The command shall retain original byte correspondence and producer availability separately from native truth.
If the request combines extraction with selected-package execution or source-only compile/lower export, then the command shall refuse the unsupported combination before dependent files are read.
If extraction is supplied to a build without quire-extraction, then the request decoder shall refuse the unknown field.
The command shall preserve existing byte/file/runtime limits and fresh retries.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-031-AC-1 | Actual binary execution of LF/CRLF Markdown aggregate and operation fixtures returns healthy, violating and frame-refused outcomes with exact original/body mapping and unchanged extraction metadata. | Test |
| FR-031-AC-2 | Invalid/unsupported native bodies and unavailable extraction retain original identity, authored selection and actual producer/compiler failure data. | Test |
| FR-031-AC-3 | Malformed extraction descriptors, multiple bindings, unsupported mode combinations, stale original bytes and disabled-feature requests refuse without successful execution; mode failures have distinct codes and clause-count details before file I/O. | Test |
| FR-031-AC-4 | Original line and runtime limits return incomplete, a fresh request succeeds, and ordinary native command behavior remains unchanged. | Test |
| FR-031-AC-5 | A native-run/1 extraction request whose body record names `authority` `agent-ix`, `identity` `b`, `document` `B` and, as `digest`, the raw-artifact digest of the extracted body's bytes runs, and its result renders the native identity (`agent-ix`, `b`, digest) and the formal identity (`B`, digest). The same request without the body's `authority`, or without its `digest`, refuses at the request stage with `invalid-request` and exits 20; with a `digest` of other bytes it refuses `stale_dependency`/`content-mismatch`, naming both digests. | Test (TC-430) |

## Dependencies

- [FR-026](FR-026-run-standalone-native-workflow.md): existing bounded command intake and runtime reports.
- [FR-030](FR-030-consume-quire-extraction.md): actual optional Quire consumer.

