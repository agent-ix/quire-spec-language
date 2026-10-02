---
id: SR-333
title: "Code and Rust review of native protocol emission"
type: SpecReview
analysis: code-review
scope: "src/protocol_artifact/native/; src/protocol_artifact/mod.rs; tests/native_protocol_emission.rs; tests/support/native_protocol/mod.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Recheck of this review at `89bc4e3`, covering the corrections in `8cba7c6..89bc4e3`
(six source/spec files, 403 added test lines) against the initial findings and the
seams they touch. Skills applied: `agent-skills/code-review`, which dispatches the
Rust lane to `agent-skills/rust-review`, with `agent-skills/rust-style` as the
portable idiom default because the repository documents no Rust idiom skill of its
own. The high finding is resolved by executing tests, not by assertion count: the
four added tests take real source through parser, binding, typing, definedness,
`native::admit`, `native::emit` and the independent `read`, and each newly covered
behavior has an assertion that a plausible wrong implementation would fail.

## Verdict

**CONDITIONAL** — no high finding. The previously uncovered Choice, Await and
positive `Repeat` lowerings now execute with load-bearing assertions, the checked
integer conversion is single-sourced, and the remaining items are evidence-
granularity and style notes with no failing scenario against the delivered code.

## Disposition of the initial findings

- FND-001 (high, uncovered Choice/Await/`Repeat`/`NeedsAuthority`/non-literal
  Boolean lowering): **resolved**. Four tests now execute those paths — see the
  verification notes below.
- FND-002 (duplicate divergent `index()` helpers): **resolved**. `layout.rs` no
  longer defines one; every caller reaches `types::index`, which uses
  `u32::try_from` after the 1,048,576 bound (`types.rs:467`).
- FND-003 (bare narrowing `as` casts and a shadowed `index`): **resolved**.
  `controls.rs:74` binds `position` and converts through `index()`; `layout.rs:80`
  does the same for anchors; `layout.rs:465`/`layout.rs:474` use `checked_sub` with
  a typed `Invalid::Locus`. No narrowing `as` cast remains in the module; the
  surviving casts are `u32 -> usize` widenings.
- FND-004 (inconsistent charge-before-work on generated names): **resolved**. The
  three late sites now reserve `work.bytes(..)` before `format!`
  (`controls.rs:160`, `controls.rs:181`, `runtime.rs:507`), matching the
  `channels::name`/`instance_name`/`clock_name` pattern. The second charge inside
  `text()` is not double accounting: `byte_work` is defined as cumulative
  traversed/copied bytes, and the name is both formatted and copied.
- FND-005 (containment guard reads as a check it is not): **resolved as
  documented**. `layout.rs:515-517` now states that `ScopeReport` is the authority
  for capture/event regions and that the guard is a lexical check for authored
  phase/`let` regions.
- FND-006 (pass-through `value_scope`, raw indexing in `metadata.rs`):
  **resolved for the cited sites**. `value_scope` is deleted and `values.rs:92`
  calls `scope` directly; `metadata.rs:293-303` uses `.get().ok_or(Invalid::Owner)`
  for the source index, the source and the proof binding. Residual raw indexing
  elsewhere is carried forward as FND-003 below.

## What the recheck verified in the new coverage

- **Owned choice, coverage and non-overlap.** The refusal table exercises both
  halves of the rule with distinct inputs: two simultaneously true guards
  (`true or false` / `not false`) hit `selected.replace(..).is_some()`, and two
  simultaneously false guards (`false and true` / `true implies false`) hit
  `selected.ok_or(..)` (`families.rs:196-203`). Each row pins one operator result:
  flipping `or(T,F)`, `not(F)`, `and(F,T)` or `implies(T,F)` turns a refusal into a
  successful admission and fails the test. Three further rows pin the refusal of a
  non-constant guard, a non-constant `visible` premise and a non-constant `if`
  condition as `Unsupported::FamilyProof`.
- **Non-literal closed Boolean evaluation.** The success case forces the
  interpreter's short-circuit arms to produce constants that a naive
  both-operands-required implementation would not: `true or view.plain.ready`,
  `false and view.plain.ready` and `if false then view.plain.ready else true` all
  contain a non-constant operand, so any regression to a strict evaluation yields
  `None` and the admission refuses.
- **Positive bounded repetition and the max-zero bypass.** Three cases separate the
  branches at `families.rs:221-231`: an observable `event` body under `max 2`, a
  non-progressing `check` body under `max 0`, and a non-progressing body under a
  constant-false guard. Removing either bypass turns rows two and three into
  `Invalid::Control`. Each case asserts the emitted `maximum`, whether the body is
  an `Event`, the `exhausted` target name, and exactly one `RepeatProgress` edge
  with its owner, both endpoints, both ports and its own `maximum`.
- **Await clock, progress/closure and causal edges.** The test reads the authored
  clock token back out of the parser namespace and asserts the emitted clock
  binding's locus span equals that original span — not a value copied from the
  payload. It also pins `AwaitAnchor::Event` to the preceding `Started` node, the
  selected `EventPosition` profile identity, the `[0,2]` interval, the `Clock`
  binding kind and control subject, and that both `Progress` and `Closure`
  bindings require the clock handle and share its anchor. Six causal edges are
  checked by kind, owner, endpoint and port.
- **`Progress::NeedsAuthority` refusal.** The same await node is admitted inside a
  `sequence` and refused inside a `repeat` in one test. That pairing is what makes
  the `Unsupported::FamilyProof` assertion meaningful: the refusal cannot be
  explained by the await node itself, only by the repeat arm at
  `families.rs:229-231`.
- **Round trip.** Every success case ends with `native::emit` and an independent
  `read`, and the reader's expectations in `tests/support/native_protocol/mod.rs`
  are rebuilt from parser spans and authored mappings rather than read back from
  the emitted payload. `discharged()` asserts real `Typed`/`Discharged`
  dispositions rather than tolerating a partial report.
- **Hygiene, re-confirmed at `89bc4e3`.** No `unwrap`, `expect`, `panic!`,
  `todo!`, `unimplemented!`, `dbg!`, `#[allow]`, `TODO` or `FIXME` anywhere in
  `src/protocol_artifact/native/`; SPDX header and `//!` requirement citation on
  all ten files; `#![forbid(unsafe_code)]` intact.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The choice test does not pin case-to-guard correspondence. `case.guard.declaration` is asserted to be `0`, which is the only possible value in a single-declaration package, and nothing asserts `case.guard.index`. An emitter that attached the `no` guard to the `yes` case and vice versa still admits (exactly one guard is true either way), still emits the asserted labels and body names, and still round-trips through `read`, so the test passes. Asserting the two guard handles are distinct and resolve to the authored guard spans would close it | tests/native_protocol_emission.rs:1138; tests/native_protocol_emission.rs:1142; src/protocol_artifact/native/controls.rs:104 | correct-requirement-no-evidence |
| FND-002 | low | The interpreter's `Equal`/`NotEqual` results are proved constant but not proved correct. The success case pairs guard `G` with `not (G)`, so the exactly-one-true rule holds for any self-consistent valuation; inverting `BinaryOp::Equal` to `left != right` only flips which case is selected and the test still passes. `and`, `or`, `not` and `implies` are pinned by the refusal rows; `=` and `!=` are not. A third case whose admission depends on the computed value would close it | src/protocol_artifact/native/families.rs:353; src/protocol_artifact/native/families.rs:356; tests/native_protocol_emission.rs:1121 | correct-requirement-no-evidence |
| FND-003 | low | Style note, no failing scenario. Raw slice indexing survives in library code where the module's own idiom is `.get().ok_or(..)`: `metadata.rs:36` indexes `definitions` with a position found in `registered` (safe only because `definitions()` builds both from one loop over `selected`), and `layout.rs:416`/`514`/`544`/`549`/`552` plus `runtime.rs:68`/`151`/`176`/`343`/`553` index tables with module-internal handles. Each is in range for every reachable input; the inconsistency is that a future edit to either side of one of these pairings turns a typed error into a panic | src/protocol_artifact/native/metadata.rs:36; src/protocol_artifact/native/layout.rs:416; src/protocol_artifact/native/runtime.rs:151; src/protocol_artifact/native/runtime.rs:176 | missing-requirement |
| FND-004 | low | Style note, no failing scenario. `native_owned_choice_proves_nonliteral_constant_guards_and_preserves_both_branches` is written as unwrapped single-line statements — several assertions exceed 200 columns — while the three sibling tests added in the same commit are rustfmt-normalized. `cargo fmt --all -- --check` is clean because rustfmt leaves over-width macro arguments alone, so the gate does not catch the divergence | tests/native_protocol_emission.rs:1135; tests/native_protocol_emission.rs:1143; tests/native_protocol_emission.rs:1144 | missing-requirement |
| FND-005 | low | Style note, no failing scenario. Overlapping and uncovered constant choices both return `Error::Invalid(Invalid::Control)`, so the two refusal rows assert the same value and neither can tell which rule fired. The pairing with the success case keeps the suite honest — an emitter that refused every choice would fail the success test — but a distinct cause per rule would make the refusals self-describing in the typed vocabulary the wire contract publishes | src/protocol_artifact/native/families.rs:199; src/protocol_artifact/native/families.rs:203; tests/native_protocol_emission.rs:1163 | missing-requirement |

## Gates

Root-supplied local logs for this exact correction source at `89bc4e3`; inspected,
not re-run. Commands as recorded by root; the logs themselves do not echo them.

- `cargo fmt --all -- --check` — `/tmp/quire-native-emission-corrections-fmt.log`, empty.
- `cargo clippy --locked --all-targets --no-default-features -- -D warnings` —
  `-clippy-minimal.log`, no diagnostics.
- Same with `--all-features` — `-clippy-all.log`, no diagnostics.
- `cargo test --locked --no-default-features -- --test-threads=1` —
  `-test-minimal.log`, 51 suites plus doctests, 528 passed, 0 failed, 4 ignored.
- Same with `--all-features` — `-test-all.log`, 544 passed, 0 failed, 4 ignored
  (3 `fixture_audit` IT-004 private-packet lane, 1 `native_backend` LC04 activation
  gate — both inherited, both carrying a named reason), `native_protocol_emission`
  15/15, 5 doctests including the `native::emit` `compile_fail,E0308` case.
- Focused — `-focused.log`, `native_protocol_emission` 15/15,
  `protocol_artifact` 24/24, `protocol_number` 9/9.
- No `deny.toml` exists, so `cargo deny` does not apply.
- No gate was re-run: every unresolved finding above is an evidence-granularity or
  style note that source inspection settles.
