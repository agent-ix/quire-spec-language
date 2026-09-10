---
id: TC-108
title: "Execute an actually extracted native clause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-011
    type: verifies
---
## Description

Integration, P1: use the pinned real Quire Rust extractor with immutable licensed
Markdown and the native compiler/runtime; compare exact original/body source.

## Test Procedure

Compile selected indented CRLF and LF bodies with Unicode surroundings. Execute
healthy/violating/refused input cases. Compare retained extraction metadata with
the actual producer output. Exercise malformed/unsupported native bodies,
foreign/stale selections, unavailable extraction, EOF mapping and exhausted limits.

## Expected Results

Each foreign source identity, path, package, contract version and semantic-core
version must return its own typed preflight cause and actual/expected values,
without an extraction outcome. Both minimal and enabled feature suites run locally.

The original clause remains identifiable through actual extraction, compilation
and execution. All failed prerequisites retain their stage's data without a
successful package or fabricated truth. Fresh retries use independent budgets.
