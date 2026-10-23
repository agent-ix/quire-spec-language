---
id: TC-108
title: "Execute an actually extracted native clause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: verifies
---
## Description

Integration, P1, in `qsl-source`'s and `qsl-replay`'s tests built with their
`quire-extraction` feature: use the real Quire Rust extractor (`quire-rs`)
with immutable licensed Markdown, spine compile and the spine clause runner
(FR-109); compare exact original/body source.

## Test Procedure

Compile selected indented CRLF and LF bodies with Unicode surroundings. Execute
healthy/violating/refused input cases. Compare retained extraction metadata with
the actual producer output: extract a document whose population holds the
selected clause, a second clause and a clause tagged with an unchecked
language, and compare the ExtractedSource's ClausesOutcome with the
ClausesOutcome `quire-rs` returns for the same bytes and context, before and
after a successful and a failed spine compile of the body (FR-030-AC-2). Exercise malformed/unsupported native bodies,
foreign/stale selections, unavailable extraction, EOF mapping and exhausted limits.

## Expected Results

Each foreign source identity, path and package must return its own typed preflight cause and actual/expected values,
without an extraction outcome. Both the default build and the build with
`qsl-replay`'s `quire-extraction` run locally; the default build calls no
Quire extraction.

The ExtractedSource's ClausesOutcome equals Quire's: the same availability,
every clause of the population, the same diagnostics and the
unchecked-language advisory, and it is unchanged after either compile.

The original clause remains identifiable through actual extraction, compilation
and execution. All failed prerequisites retain their stage's data without a
successful package or fabricated truth. Fresh retries use independent budgets.
