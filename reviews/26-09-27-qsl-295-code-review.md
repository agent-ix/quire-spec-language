---
id: SR-755
title: "QSL-295 code review of PR 502 (run_clause I3 extracted source)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6a78d6c9843c419b5a94eed41654a22e4b11bbcc; Cargo.toml; Cargo.lock; qsl-replay/Cargo.toml; qsl-replay/src/spine.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/clause/tests.rs; context read: qsl-source/src/lib.rs, src/command/extraction.rs, src/mapped.rs, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, Makefile"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
---
## Summary

Ticket: QSL-295. PR: quire-spec-language#502 at 6a78d6c9, base a62bd8b4.
Code review with the rust-review lane folded in.

Sound:
- The extracted body feeds `compile`. `ClauseRunSource::unit` hands
  `map.body()`'s identity, path and text to `compile` (clause.rs:120-131,
  :481-485). `ExtractedSource` has private fields and only `qsl_source::extract`
  builds one, after `verify_body_bytes` and `SourceMap::verify`
  (qsl-source/src/lib.rs:144-148, :273-300, :335). So the body is verified
  against the original's bytes before `run_clause` can hold it. This matches
  the root SEAM-1 join in src/command/extraction.rs:186-231.
- Provenance: `source` is the body identity, `source_digest` is the body
  digest, and `extraction` is the original's identity and `digest()`
  (clause.rs:125-128). This is what FR-109 Outputs :81-83 names. Both report
  builders carry it (clause.rs:466-472, :695-701).
- Feature off: the enum has one variant and `extraction` is always `None`.
  `ExtractionOrigin` is not cfg-gated. Nothing serializes
  `ClauseRunProvenance`, so no cfg changes wire output.
- ADR-011:717 does read "I3 under feature `quire-extraction`" for layer-6
  `replay`. The edge is allowed.
- No allow-list, arch-lint, deny or seam file is in the diff.
- `make ci` runs `ci-all-features` (Makefile:89-91). The gate log
  scratchpad/qsl-295-ci-r1.log shows both `extracted::` tests `ok`, then `exit=0`.
- No `unwrap`, `expect` or `panic` in production code. No new string
  dispatch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `run_clause` drops the extracted clause's declared language. `ExtractedSource::language()` is never read, and `qsl_source::extract` accepts a fence of any language. The root SEAM-1 caller passes the language to `mapped::compile`, which refuses anything but `ix:native` with `unknown_language` (src/mapped.rs:159-194). qsl-source's own contract says the caller "feeds the map's body and the declared language into the native compiler" (qsl-source/src/lib.rs:139-141). Failure: an `ix:formal` or foreign fence is parsed as native. It then either compiles or refuses with a parse code, never `unknown_language`, so the same document gives different results through `run_clause` and through the CLI. Fix: in the `Extracted` arm, check the language through a `#[string_edge]` decode (or reuse the root's `NativeLanguage` rule) and report stage `compile`, `unknown_language`, with the extraction still in provenance. Add a test with a non-`ix:native` fence. | qsl-replay/src/spine/clause.rs:119-132 |
| FND-002 | low | The `Extracted` variant is cfg-gated on a public enum without `#[non_exhaustive]`. Cargo features unify, so a downstream crate that matches `ClauseRunSource` exhaustively on `Program` stops compiling when any other crate in the graph turns on `qsl-replay/quire-extraction`. No such consumer exists today. Fix: mark `ClauseRunSource` `#[non_exhaustive]`. | qsl-replay/src/spine/clause.rs:76-95 |

## Verdict

Request changes. Fix FND-001 in this PR. FND-002 is a one-line change to make
in the same round. Everything else (compile feed, verification, provenance,
feature edge, gate coverage, code rules) is sound.

## Dispositions

Round 2, reviewed 39837943e97327e49c203501188a556651439211. Gate log
scratchpad/qsl-295-ci-r2.log: first line is the head, last line `exit=0`, and
`string-edge` is clean.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 39837943. `run_clause` decodes `extracted.language()` through `NativeLanguage::of` before compiling. A non-`ix:native` fence reports `ClauseDisposition::UnknownLanguage` at stage `compile`, `Refusal`, with exit `Code::UnknownLanguage.exit_code()` = 20. The extraction stays in provenance (qsl-replay/src/spine/clause.rs:506-517). The decoder moved unchanged, still `#[string_edge]`, from src/mapped.rs to qsl-foundation/src/source_map.rs:6-20. `mapped::compile` makes the same `NativeLanguage::of(language).is_none()` call, so the CLI path is unchanged. No Cargo.toml or Cargo.lock change: qsl-foundation already depends on qsl-attrs, and qsl-replay and the root already depend on qsl-foundation. |
| FND-002 | fixed | 39837943. `ClauseRunSource` is `#[non_exhaustive]` (qsl-replay/src/spine/clause.rs:82-83). |
