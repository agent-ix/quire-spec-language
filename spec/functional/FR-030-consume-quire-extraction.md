---
id: FR-030
title: "Compile clauses from actual Quire extraction"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-022
    type: references
---
## Description

When a caller selects an authored clause in an original document, the optional Quire consumer shall invoke the pinned Quire Rust extractor and yield the verified body with its document source map, and the extracted-command caller shall compile that body through the existing mapped native compiler.

## Inputs

An immutable original Source, a caller-loaded Quire SemanticContext, a Selection
of the authored clause ID, the authored requirement's package and the
caller-assigned native body identity, and caller-lowered original byte and line
limits. The extracted-command caller holds the one authored ClauseBinding, the
formal body identity, admitted models and the native compiler stage limits.
The caller selects and verifies the original document's
digest before admission. The selected Quire heading ID equals the authored
binding's clause ID; the body contains a complete single-clause native unit.
The context explicitly names the original source identity/path and authored
package. The consumer selects Quire contract 1.0.0 / semantic-core 0.1.0.

## Outputs

The consumer yields an ExtractedSource: the verified body in its document
SourceMap, the selected clause's declared language and the unchanged complete
ClausesOutcome. Otherwise it yields a typed failure retaining original source,
selection, any completed extraction outcome and the actual preflight or
correspondence cause. The extracted-command caller yields a mapped native package
with the unchanged ClausesOutcome, or a typed failure retaining original source,
authored binding, any completed extraction outcome and the actual preflight,
correspondence or native/mapped compiler cause. Quire's unchecked-language
advisory and lossy availability remain observable after successful native compilation.
No extraction status is rewritten into a parsing or runtime result.
Preflight failures distinguish byte and line ceilings, contract and semantic-core
version skew, and source identity, path and authored-package mismatches with
typed causes carrying actual and expected values. Their existing native code
families remain unchanged. The consumer is the `qsl-source` crate. Its re-exports of
the pinned Quire-owned contracts (ClausesOutcome, SemanticContext and the result and
failure types the extracted-command caller renders) are deliberate; upgrades require
compatibility review. The consumer also builds the validated clause-only Quire
context for an original source and authored package. The extracted-command caller
reaches Quire only through `qsl-source`.

## Behavior

The consumer shall check original size and line count before invoking extraction.
The consumer shall distinguish each original-context mismatch before invoking extraction.
The consumer shall call Quire extract_clauses over the original bytes, without a second Markdown or expression parser.
The consumer shall require available extraction and the selected clause/body.
The consumer shall validate the reported identity, path and fence coordinates against the original source index.
Quire's closing endColumn is one past the closing line's byte length, excluding
the line terminator (FR-071); native diagnostic columns remain Unicode scalar
positions. Trailing Unicode fence whitespace does not change body correspondence.
The consumer shall verify the body with SourceMap::verify before any offset translation.
The consumer first verifies Quire's exact body bytes. If Quire omitted the final
LF of an original CRLF pair, the compiler body also drops the remaining terminal
CR. Indentation and interior CR bytes stay intact; the source map admits only
the resulting final LF or CRLF deletion. Quire's raw body remains in its unchanged
extraction outcome. No header, import or newline is inserted.
The extracted-command caller shall invoke mapped::compile with the extracted source map, the declared language, the authored binding and the formal body identity.
If extraction, correspondence or native compilation fails, then the extracted-command caller shall return no partial package and retain the actual failure.

The original byte ceiling is 1 MiB and the line ceiling is 4096, including a
trailing empty line. Caller limits may only lower these ceilings. The line bound
limits work in the existing extractor before it builds its clause population;
it is not a new Quire-wide performance guarantee. Existing compiler stage limits,
including the native source-byte limit on the extracted body, remain independent
and fresh for each request and apply at compilation.
The quire-extraction Cargo feature is optional; default native builds do not
call Quire extraction. The root crate's feature enables `qsl-source`'s own
feature of the same name, which is what lets QSL code name quire-rs. quire-rs is
still built transitively in every build, through FCD's
`agent-ix-extraction-frontend` (`model::intake`). C retains changes to existing
repositories, schemas, CLI/wire adoption and broader integration. This is A's native compiler consumer.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-030-AC-1 | Actual Quire extraction of an indented CRLF clause with Unicode surroundings reaches mapped compilation and existing healthy/violating/refused runtime outcomes with exact authored/source identity. | Test |
| FR-030-AC-2 | Successful and failed native compilation retain Quire's original availability, complete clause population and diagnostics; an unchecked tag alone does not pass the native parser. | Test |
| FR-030-AC-3 | Stale original digest, foreign context, missing/duplicate selected clauses and unavailable extraction refuse without a package; each foreign context field retains its distinct typed preflight cause before extraction. | Test |
| FR-030-AC-4 | Verified original/body coordinates remain exact at body EOF and with trailing layout; invalid body identity and out-of-range mapping requests refuse. | Test |
| FR-030-AC-5 | Original byte/line and native stage limits return incomplete, with successful fresh retry; minimal builds retain their existing behavior. | Test |

## Dependencies

- [FR-011](FR-011-integrate-opaque-extraction.md): actual extraction integration.
- [FR-022](FR-022-compile-mapped-native-clauses.md): existing mapped compiler.
- [FR-004](FR-004-verify-source-maps.md): exact source correspondence.
