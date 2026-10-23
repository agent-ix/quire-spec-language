---
id: FR-030
title: "Extract a selected clause body through Quire"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
---
## Description

When a caller selects an authored clause in an original document, the optional Quire consumer shall invoke Quire's Rust extractor (`quire-rs`) and yield the verified body with its document source map. `run_clause` takes that body as its I3 source and S1 parses it (FR-109, FR-108-AC-5).

## Inputs

An immutable original Source, a caller-loaded Quire SemanticContext, a Selection
of the authored clause ID, the authored requirement's package and the
caller-assigned native body identity, and caller-configured original byte and line
limits.
The caller selects and verifies the original document's
digest before admission. The selected Quire heading ID equals the authored
binding's clause ID; the body contains a complete single-clause native unit.
The context explicitly names the original source identity/path and authored
package.

## Outputs

The consumer yields an ExtractedSource: the verified body in its document
SourceMap, the selected clause's declared language and the unchanged complete
ClausesOutcome. Otherwise it yields a typed failure retaining original source,
selection, any completed extraction outcome and the actual preflight or
correspondence cause. Quire's unchecked-language advisory and lossy
availability remain observable in the ClausesOutcome.
No extraction status is rewritten into a parsing or runtime result.
Preflight checks the context by content. Its failures distinguish the byte
and line limits, and source identity, path and authored-package mismatches
against the original and the binding, with typed causes carrying actual and expected values. Their existing native code
families remain unchanged. The consumer is the `qsl-source` crate. It re-exports
Quire's contracts (ClausesOutcome, SemanticContext and the result and failure
types its callers render), so a caller names them through
`qsl-source`. The consumer also builds the validated clause-only Quire
context for an original source and authored package. A caller
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
If extraction or correspondence fails, then the consumer shall return no partial body and retain the actual failure.

The original byte limit has a published default of 1 MiB and the line limit
one of 4096 lines, including a trailing empty line. The caller raises or
lowers each, and the value is used as given. The line bound
limits work in the existing extractor before it builds its clause population;
it is not a new Quire-wide performance guarantee. The spine's stage limits
apply to the extracted body at S1, independent and fresh for each run.
The quire-extraction Cargo feature is optional; default builds do not
call Quire extraction. `qsl-replay`'s feature enables `qsl-source`'s own
feature of the same name, which is what lets QSL code name quire-rs (ADR-011
§7.3 X-4). quire-rs is
still built transitively in every build, through FCD's
`agent-ix-extraction-frontend` (`model::intake`). C retains changes to existing
repositories, schemas, CLI/wire adoption and broader integration.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-030-AC-3 | Stale original digest, foreign context, missing/duplicate selected clauses and unavailable extraction refuse without a body; each foreign context field retains its distinct typed preflight cause before extraction. | Test (TC-108) |
| FR-030-AC-4 | Verified original/body coordinates remain exact at body EOF and with trailing layout; invalid body identity and out-of-range mapping requests refuse. | Test (TC-108) |
| FR-030-AC-5 | Original byte and line limits return incomplete, with successful fresh retry; minimal builds retain their existing behavior. | Test (TC-108) |

## Dependencies

- [FR-004](FR-004-verify-source-maps.md): exact source correspondence.
- [FR-109](FR-109-run-a-state-clause-through-the-spine.md): `run_clause`, which takes the body as its I3 source.
