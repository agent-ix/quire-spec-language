---
id: SR-1325
title: "Code review of quire-spec-language PR #643: FCD 68c0acba and quire-rs 8ab0d509 bump"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8852006dc4363c875a3d64b267d615116e02fb6b; PR #643 diff against origin/main: Cargo.lock, qsl-semantics/Cargo.toml, qsl-semantics/src/model/intake.rs, qsl-semantics/tests/it/model_intake.rs, qsl-source/Cargo.toml, qsl-source/src/lib.rs, qsl-source/tests/it/quire_source.rs, src/command/output/extraction.rs, tests/it/composed_domain_models.rs"
review_set: subset
---
# Code review of quire-spec-language PR #643

## Summary

Ticket: IR-626 (QSL side). The PR bumps filament-core-data to rev
68c0acba and quire-rs to rev 8ab0d509, then follows the API and fixture
changes. `make ci` exit 0 on this head (coordinator's log); no build re-run.
Rust lane folded in.

Checked:
- **Pins.** Both stay exact `rev =` pins, no `branch =`. quire-rs 8ab0d509
  matches FCD's own pin.
- **One copy each in Cargo.lock.** quire-rs 0.50.1, agent-ix-extraction-frontend
  0.2.1, agent-ix-semantic-ir 0.1.1 and serde 1.0.228 each appear once.
- **Mechanical fixes.** `read_semantic_block` dropped its third argument
  upstream (`contract.rs:131`), both call sites follow. `LiftRequest.provenance`
  removed and the doc comment updated. Fixture module paths and the
  `fixture/domain` construct module follow FCD's renames. No stale
  references to the old names or the fingerprint file remain in code.
- **Architecture-lift test.** It now compares the whole lifted document with
  FCD's committed `expected/semantic-ir.json` as parsed JSON. That is a real
  functional check, stronger than the old digest compare: a wrong field fails
  it with a structural diff. The raw bytes are still digested and fed to
  `admit`, so byte-level identity is still exercised.
- **New transitive packages.** tree-sitter 0.26.13 and its python, rust and
  typescript grammars, tree-sitter-language, streaming-iterator (crates.io),
  quire-rust-extraction (quire-rs git), quire-code-parse (quire-code-rs git)
  and agent-ix-semantic-schema (FCD git). All are lockfile references. The
  diff adds no files, no vendored sources and no build artifacts.
  `.gitignore` carries unanchored `target/` and `*-target/`.

## Verdict

Approve with one medium finding to fix in this PR. The pins, lock, mechanical
fixes and the lift test are correct. `SEMANTIC_CORE_VERSION` is a hand-typed
copy of a value quire-rs owns, and this PR had to edit it and two test
literals only because quire-rs changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `SEMANTIC_CORE_VERSION = "0.3.2"` hand-copies the one semantic-core version quire-rs embeds. quire-rs 8ab0d509 exposes it as `quire_rs::semantic::embedded::embedded_semantic_core_version()` (and `CONTRACT_VERSION` as `embedded::CONTRACT_VERSION`), and `read_semantic_block` already refuses any other value (`contract.rs:158-170`). Measured what breaks without the literal: nothing. `clause_context` must name the embedded version, and quire-rs gives it. The preflight refusal (`preflight.rs:108`) is functional and spec-backed (FR-030-AC-3, a foreign context keeps its typed `SemanticCore` cause), but it needs quire-rs's value, not a copy. Every quire-rs bump that moves the bundle breaks QSL until someone retypes the literal, which is this PR. The test literals `"semantic_core":"0.3.2"` (quire_source.rs:42) and `assert_eq!(..., "0.3.2")` (extraction.rs:247) are version assertions. Fix: take both constants from quire-rs, keep the preflight check, and have the tests use the constant, not a literal. | qsl-source/src/lib.rs:33-35; qsl-source/src/preflight.rs:102-113; qsl-source/tests/it/quire_source.rs:42; src/command/output/extraction.rs:247 |
| FND-002 | low | The lift test parses `document` twice: once as `lifted` for the expected-document compare and again as `package` a few lines later. Reuse `lifted`. | qsl-semantics/tests/it/model_intake.rs:131-139 |

## Dispositions

Round 1, reviewed at eaec92ca69e9f816a4fa243f15c72b4b57134efe (range 8852006dc..eaec92ca6).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4843b3fbe: `CONTRACT_VERSION` re-exports `quire_rs::semantic::embedded::CONTRACT_VERSION`; `semantic_core_version()` returns `embedded_semantic_core_version()`; preflight refusals kept, comparing against these; test literals replaced by the accessors (eaec92ca6 moves the test imports). No `SEMANTIC_CORE_VERSION` or `"0.3.2"` remains in code. |
| FND-002 | fixed | 4843b3fbe: `let package = lifted;` replaces the second parse. |
