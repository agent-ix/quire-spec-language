---
id: FR-030
title: "Extract a selected clause through actual Quire extraction"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: references
---
## Description

When a caller selects an authored clause in an original document, the optional Quire consumer shall invoke Quire's Rust extractor (`quire-rs`) and yield the verified body with its document source map, which the spine clause runner (FR-109, FR-031) compiles under the body's own identity and runs.

## Inputs

An immutable original Source, a caller-loaded Quire SemanticContext, a Selection
of the authored clause ID, the authored requirement's package and the
caller-assigned native body identity, and caller-configured original byte and line
limits. The spine clause runner holds the other `ClauseRunRequest` members:
the package and dependency inputs, the provisions, the selection and the
spine and evaluation limits (FR-109).
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
correspondence cause. The spine clause runner yields its `ClauseRunReport` for
the body (FR-031). Quire's unchecked-language advisory and lossy availability
remain observable in the retained ClausesOutcome after a successful compile.
No extraction status is rewritten into a parsing or runtime result.
Preflight checks the context by content. Its failures distinguish the byte
and line limits, and source identity, path and authored-package mismatches
against the original and the binding, with typed causes carrying actual and expected values. Their existing native code
families remain unchanged. The consumer is the `qsl-source` crate. It re-exports
Quire's contracts (ClausesOutcome, SemanticContext and the result and failure
types), so a caller names them through `qsl-source`. The consumer also builds
the validated clause-only Quire context for an original source and authored
package. QSL reaches Quire only through `qsl-source`.

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
The spine clause runner shall compile the verified body under its body identity with the declared language; a language other than QSL native refuses at compile with `unknown_language` (FR-031).
If extraction or correspondence fails, then the caller shall run no compile, return no partial package and retain the actual failure.

The original byte limit has a published default of 1 MiB and the line limit
one of 4096 lines, including a trailing empty line. The caller raises or
lowers each, and the value is used as given. The line bound
limits work in the existing extractor before it builds its clause population;
it is not a new Quire-wide performance guarantee. The spine limits, including the
source-byte limit on the extracted body, remain independent and fresh for each
request and apply at compilation.
The `quire-extraction` Cargo feature of `qsl-source` is optional and is enabled
only through `qsl-replay`'s feature of the same name; it is what lets QSL code
name quire-rs. The root crate has no `quire-extraction` feature (ADR-011 §7.3
X-4), and a build without `qsl-replay`'s feature calls no Quire extraction. quire-rs is
still built transitively in every build, through FCD's
`agent-ix-extraction-frontend` (`model::intake`). C retains changes to existing
repositories, schemas, CLI/wire adoption and broader integration.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-030-AC-1 | Actual Quire extraction of an indented CRLF clause with Unicode surroundings reaches spine compilation and the clause runner's healthy, violating and refused outcomes with exact original and body identity. | Test (TC-108) |
| FR-030-AC-2 | A successful extraction yields an ExtractedSource whose ClausesOutcome equals Quire's actual output unchanged: its availability, its complete clause population, its diagnostics and, for a clause tagged with an unchecked language, Quire's unchecked-language advisory. After a successful and after a failed spine compile of the body, the retained ClausesOutcome is still that one; an unchecked tag alone does not pass the S1 parser. | Test (TC-108) |
| FR-030-AC-3 | Stale original digest, foreign context, missing/duplicate selected clauses and unavailable extraction refuse without a package; each foreign context field retains its distinct typed preflight cause before extraction. | Test (TC-108) |
| FR-030-AC-4 | Verified original/body coordinates remain exact at body EOF and with trailing layout; invalid body identity and out-of-range mapping requests refuse. | Test (TC-108) |
| FR-030-AC-5 | Original byte/line and spine limits return incomplete, with successful fresh retry; a build without `qsl-replay`'s `quire-extraction` calls no Quire extraction. | Test (TC-108) |

## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md) and
  [FR-031](FR-031-run-extracted-native-source.md): the spine clause runner that
  compiles and runs the extracted body.
- [FR-004](FR-004-verify-source-maps.md): exact source correspondence.
