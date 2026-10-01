---
id: SR-965
title: "QSL-358 slice 5 code review (with rust-review lane and test-oracle check) of PR 570, the call-surface vocabulary moved into quire-semantic-value"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0ff6644bdafd4b07469b86b08d993c1fd4bfa582; diff 0a399675...0ff6644b; quire-semantic-value/src/{checking,location,call,loss,lib}.rs; qsl-semantics/src/check/{check,refusal,mod,family}.rs; qsl-eval/src/value/{mod.rs,expression/{mod,evaluate,family}.rs}; src/command/output.rs; xtask/src/definition_scan.rs; Cargo.toml; qsl-replay/Cargo.toml; qsl-package/Cargo.toml; repointed callers"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 5). PR: quire-spec-language#570 at 0ff6644b, base main
0a399675 (slice 1, #565). Gate: `make ci` exited 0 at 0ff6644b (log line
`head=0ff6644b... exit=0`, including `cargo build --locked -p
quire-semantic-value --target thumbv7em-none-eabi`). Not re-run.

Checked against the code:

- **Leaf discipline holds.** `quire-semantic-value/Cargo.toml` is unchanged by
  this diff: `quire-exact`, `quire-canonical`, `serde` (alloc), `thiserror` (no
  default features). The four new modules use only `alloc` and `quire_exact`.
  No std, qsl-foundation, qsl-cst or serde_json. The thumbv7em build is still
  in `make ci` (Makefile:199).
- **Moves are byte-faithful.** `CheckMode`, `CheckingLimits`,
  `DepthAboveMaximum`, `MAX_CHECKING_DEPTH`, the three `DEFAULT_CHECKING_*`
  constants, `Origin` (6 variants), `Location`, `InputRefusal` (6 variants),
  `ValueLoss` and `LocatedLoss` match main's definitions. Only doc links were
  reworded. `Typer` now reads `limits.nodes()`/`depth()` instead of the private
  fields, with the same values.
- **`input_refusal_code` maps every variant exactly as main's
  `InputRefusal::code()` did:** UnknownFunction/UnknownClause ->
  MissingDeclaration, Arity/WrongValueKind -> InvalidRuntimeInput,
  DanglingReference -> DanglingReference, ObservationsMismatch ->
  InvalidRuntimeInput. The match is exhaustive, and
  `input_refusal_code_maps_each_refusal_to_its_catalog_code` pins all six.
  The `cause()` strings are unchanged and the SV test pins all six. Every old
  `.code()` caller (qsl-replay execute.rs:170, :259, and one qsl-eval test) was
  repointed.
- **No shims.** `qsl_semantics::check` brings `Location`, `Origin`,
  `CheckMode` and `CheckingLimits` in with a private `use`, not `pub use`.
  `qsl_eval::value` stops exporting `InputRefusal`, `LocatedLoss` and
  `ValueLoss`.
- **`Location::child` is `pub`, and that is safe.** `Location`'s `origin` and
  `path` fields were already `pub`, so callers could already build any
  `Location`. `child` only clones the origin and pushes an index, so it adds no
  way to build a location a caller could not already build.
- **Dependencies.** The root crate's SV edge moves from dev to normal.
  This is needed: shipped `src/command/output.rs` names
  `quire_semantic_value::location::{Origin, Location}`. `qsl-replay` gains a
  normal SV edge for `Location` (the PR body says qsl-bench instead, but
  qsl-bench already had it). qsl-eval's duplicate SV entry (normal plus
  dev-dependency) is **not in this diff**: both lines are on main, from #565
  (slice 1), not from slice 3. See FND-005.

Mutation check of the definition scan (TC-170/TC-173). This ran in a throwaway
worktree at 0ff6644b, using `cargo test -p xtask definition_scan`.
- Copy of `InputRefusal` in `qsl-eval/src/value/expression/mod.rs`: FAILS (good).
- Copy of `Location` in `check/refusal.rs`: FAILS, two tests (good).
- Copy of `CheckingLimits` in `check/check.rs`: FAILS (good).
- Copy of `InputRefusal` in `qsl-eval/src/value/mod.rs`: PASSES. Main's old test
  would have failed on this (it asserted `len()==1` across the whole scan).
- Copy of `CheckMode` in `qsl-semantics/src/value/mod.rs`: PASSES.
- Copy of `ValueLoss` in `evaluate.rs`: PASSES the scan. Only the compiler
  catches it, through an E0255 clash with the import in that one file.

Rust lane: no unsafe, no new panics, no unwrap outside tests, no integer
conversions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | assert_defined_exactly_once_in_semantic_value counts definitions only under check/, value/expression/ and quire-semantic-value/src/, although scan_crate scans all of src, qsl-semantics/src, qsl-package/src, qsl-eval/src and quire-semantic-value/src. A second definition anywhere else in those trees passes. Measured: a copy of InputRefusal in qsl-eval/src/value/mod.rs and a copy of CheckMode in qsl-semantics/src/value/mod.rs both pass all six definition_scan tests. Main's InputRefusal test asserted locations.len()==1 over the whole scan, so it would have failed on the first copy: the oracle got weaker. For a name that now lives in a shared leaf, the claim 'defined exactly once' should mean across every scanned tree. Fix: assert locations.len()==1 over the unfiltered list, and that the one location is under quire-semantic-value/src/. | xtask/src/definition_scan.rs:372-398,536-549 |
| FND-002 | medium | The PR says RT can delete its InputRefusal afterwards. RT's copy (origin/main 5d832677, src/exact/expression.rs:432-470) has code() -> &'static str returning the catalog strings missing_declaration / invalid_runtime_input / dangling_reference. RT cannot depend on qsl-eval (std, layer 5). Its code() becomes a free function there because SV's InputRefusal carries no code, so RT has to rewrite the variant-to-code table as a second copy of input_refusal_code. That table is the copied mapping this program exists to remove, and nothing ties RT's copy to QSL's when a variant or code changes. Fix: give SV the stable catalog code string per variant, for example InputRefusal::code_str() -> &'static str, which needs no qsl-foundation dependency. Have input_refusal_code derive from it, or have its test assert Code::as_str() == code_str() for every variant. Context: RT's CheckingLimits differs in meaning (MAX_CALL_DEPTH guards host recursion, default nodes is u64::MAX, and it has no input_bytes/work_budget), so adopting SV's changes RT's defaults. That is RT's decision, but the PR's 'RT can delete' list should say so. | quire-semantic-value/src/call.rs:7-10,58-67 |
| FND-003 | low | The new doc comments in definition_scan.rs carry the ticket id 'QSL-358 slice 5' four times (372, 405, 477, 536) as 'Amended' history. main's #544 removed ticket-id tracking from code comments. State what the test checks; leave out who moved what in which slice. | xtask/src/definition_scan.rs:372,405,477,536 |
| FND-004 | low | semantic_node.rs is on main (slice 1, #565) and its line 108 names check::MAX_CHECKING_DEPTH, which this PR moves to quire_semantic_value::checking::MAX_CHECKING_DEPTH. That is the same crate, so the doc should be the intra-doc link [`crate::checking::MAX_CHECKING_DEPTH`]. qsl-package/src/emit.rs:39 is in this diff and still says 'a `check::Location`'. check no longer exports Location; name quire_semantic_value::location::Location. qsl-forms/src/spans.rs:6 has the same stale name. | quire-semantic-value/src/semantic_node.rs:108; qsl-package/src/emit.rs:39 |
| FND-005 | low | qsl-eval lists quire-semantic-value in both [dependencies] (24) and [dev-dependencies] (63). The dev entry is redundant. It is on main from #565 (slice 1), not from #567 (slice 3), and this diff does not touch the file, so it merges either way. This PR is the next one to change qsl-eval's SV usage. Delete the dev entry here so the duplicate does not survive. | qsl-eval/Cargo.toml:60-63 |

## Verdict

Changes requested (two medium). The move is correct. The leaf boundary,
behaviour parity of `input_refusal_code` and `cause()`, the no-shim rule, the
dependency edges and `Location::child` visibility all check out. FND-001 makes
the definition-scan oracle weaker than main's. FND-002 is the RT handoff gap
that this slice exists to close. FND-003 to FND-005 are cleanups.

## Dispositions

| ID | Outcome |
| --- | --- |
| FND-001 | fixed 9ca7dbe6: `assert_defined_exactly_once_in_semantic_value` counts every definition across all `scan_crate` trees and requires the one to be under `quire-semantic-value/src/`; the only uncounted name is the unrelated `src/package/view.rs` `Location<'a>`, listed by file. A copy of `InputRefusal` in `qsl-eval/src/value/mod.rs` and of `ValueLoss` in `qsl-semantics/src/value/mod.rs` now fail the scan (measured). |
| FND-002 | fixed 9ca7dbe6: `InputRefusal::code() -> &'static str` in SV names the catalog code; qsl-eval's test asserts `input_refusal_code(r).as_str() == r.code()` for all six variants, and the SV test pins all six strings. |
| FND-003 | fixed 9ca7dbe6: ticket ids and amendment history removed from the definition_scan comments. |
| FND-004 | fixed 9ca7dbe6: `semantic_node.rs` links `crate::checking::MAX_CHECKING_DEPTH`; `emit.rs` and `qsl-forms/src/spans.rs` name `quire_semantic_value::location::Location`. |
| FND-005 | fixed 9ca7dbe6: the redundant SV dev-dependency in qsl-eval is deleted (and the same redundancy in qsl-replay and the root crate after the rebase onto slice 2). |
