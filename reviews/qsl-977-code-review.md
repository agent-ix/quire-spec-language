---
id: SR-12333
title: "Code review (Rust lane) of QSL-977 plugin result readers and plugin replay context (PR #1222)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR #1222 head c0c037a7e; git diff origin/main...HEAD over qsl-replay (proof_result/plugin.rs, plugin_read.rs, plugin_replay.rs, plugin_tests.rs, execute/parity_identity.rs, spine/lifecycle.rs, witness/value_text.rs, lib.rs, proof_result.rs, execute.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-12333: Code review (Rust lane) of QSL-977 plugin result readers and plugin replay context (PR #1222)

## Summary

Ticket: QSL-977. PR quire-spec-language#1222 at head c0c037a7e. Only the `origin/main...HEAD` diff was reviewed. This is a code review with the rust-review lane folded in.

Commands run through the build lane: `cargo test -p qsl-replay --lib -- proof_result::` (all pass, including the ten TC-774 tests), `make string-edge`, `make arch-lint-api-surface-qsl`, `make seam-probe` and `make check-spec-validation`. Their exit status is in the PR comment and the Linear review comment. No full gate was run.

Soundness, measured with temporary probe tests. The probes were removed afterwards and the tree is clean.

- **No request for a non-function-contract item.** `request` reads only the routed occurrence key and the proof bounds that the driver supplies. The plugin supplies only the assignments, so plugin input cannot pick the item. A key at a non-`declaration` origin never matches a function. A function whose result is not `Boolean` is `NotReplayable`. The planner's regression unit is `p(x: Int[0, 9]): Boolean { x + 1 > 3 }`. Its `x + 1` value-validity item with `x = 0` settles `inconclusive`/`ReplayRefused(unknown_required_feature)`, not `refuted`. `p`'s `declaration` item with `x = 0` settles `refuted`. On the model unit, every state-clause `claim` item and the frame's `generated` `operation-contract` item are `NotReplayable` (measured with the `config_version` fixture).
- **No core-only basis and no non-`trusted` certification.** `read_plugin_basis` accepts only the five `BASIS` spellings, and none of them is core-only. `exact-value`, `value-bounds`, `witnessed` and `closed-form`, each with extra members, refuse `CoreOnlyBasis`. `read_plugin_certification` accepts only the string `trusted`. `null` gives `Malformed`.
- **No `Refuted` without replay.** `PluginReplayRefusal::record` gives only `Inconclusive(ReplayRefused)` or `Failed`.
- **Reader refusal order matches FR-290 steps 1 to 8 and the Assignments steps.** Each probe below breaks several rules at once, and the first rule in FR-290 order is the one reported:
  - `{"type":"bounded-solver","jumps":-1,"extra":1}` → `MissingMember /basis/horizon`.
  - The same basis with every member present, plus `extra` → `UnknownMember /basis/extra`.
  - A bad `horizon` with a bad `jumps` and `method` `dreal` → `Malformed /basis/horizon`.
  - `certificate-rejected` with an extra member → `Unsupported`.
  - `{"transcript":{},"extra":1}` → `MissingCounterexample`.
  - An undefined member on the entry and another on the counterexample → the entry's member is reported.
  - A bad value inside a repeated parameter → `Malformed .../1/value`, so the value check runs before the duplicate check.
- **`request` order matches FR-290.** A non-replayable item with two bounds at one key gives `NotReplayable`, not `DuplicateBoundKey`.

Mutation probes. Each was applied alone and the TC-774 tests were run; the source was restored afterwards.

- Killed: the Boolean guard flipped; `CertificationNotTrusted` dropped (any string accepted); `NotReplayable` settled `failed`.
- Survived:
  - `DuplicateBoundKey` checked first (M1).
  - `UnknownMember` checked before `MissingMember` (M4).
  - The `declaration`-origin check dropped (M5).
  - `exact-value` removed from the core-only list (M6).
  - `UnknownOccurrence`'s diagnostic cause changed (M8).

Rust idioms:

- Both refusal enums are exhaustive and derive `thiserror`.
- The `PluginResultRefusal` change (loses `Copy`, gains six variants) and the new public items are all in the PR body's changelog.
- `#[string_edge]` marks every string match.
- There is no `unwrap` or panic in shipped code, no new integer conversion, and no `unsafe`.
- The table-driven `check_member` keeps the shared ordered check in one place.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The code follows the FR-290 refusal order, but no test pins it. The planner's correctness fix and FR-290 Behavior both require that order. Two of the mutations that broke it survived. M1: computing `BoundEntries` (DuplicateBoundKey) before the occurrence checks survives. M4: the `UnknownMember` check moved before the `MissingMember` check (step 7 before step 6) survives. Fix: add `request(q_application, [bound, bound])` → `NotReplayable` to AC-14's test, and add `{"type":"inductive","extra":1}` → `MissingMember /basis/depth` to AC-6's cases. | qsl-replay/src/proof_result/plugin_replay.rs:186-225; plugin_read.rs:197-212; plugin_tests.rs:739-800 |
| FND-002 | medium | The regression test is not the one the planner asked for. The bug case is a value-validity item inside a **Boolean** function (`p(x) = x + 1 > 3`, `x = 0`), where replaying the owning function wrongly settles `refuted`. The test uses `q`, an `Integer` function. Because `q` is not Boolean, an implementation that resolved the requirement's owner function and replayed it when that owner is Boolean would pass the test. That is exactly the #1208 bug. The current code is correct: I measured that item settling `ReplayRefused`. Fix: add a Boolean function whose body holds an application (or change `p` to `x + 1 > 3`) and assert that its application item is `NotReplayable` and settles `inconclusive`. | plugin_tests.rs:633-706 |
| FND-003 | low | No test covers the `occurrence.origin() == declaration` guard (M5 survives). Measured: `(p's node, declaration, 1)` gives `UnknownOccurrence`. Fix: add one case. | plugin_replay.rs:187-190 |
| FND-004 | low | AC-6 lists all six core-only spellings, but the test covers only four. `exact-value` and `value-bounds` are missing, so M6 survives. Soundness holds because a removed spelling falls through to `UnknownSpelling`, but the AC row is not fully backed. | plugin_tests.rs:119-129 |
| FND-005 | low | The diagnostic assertions are loose. AC-14's test accepts either of two codes and checks no cause (M8 survives). AC-11's test checks only `result item 3`, not the pointer or the received spelling that AC-11 requires in the detail. | plugin_tests.rs:478-535, 790-799 |
| FND-006 | low | Defensive paths that are unreachable or misreported. `Encoding` is a public variant for a path that cannot happen: the ordinal is a `u32`, `Limits::new(u64::MAX)` is used, and `DuplicateKey` is caught first. That puts a match arm on every exhaustive consumer for nothing. A `QualifiedName::new` failure is reported as `UnknownOccurrence`. A function whose name fails `Identifier::new` is silently dropped in `new`, so its item settles `failed` (UnknownOccurrence), not `NotReplayable`. Prefer an `InternalFault`-style failure, or drop the variant (see SR-12335 FND-002). | plugin_replay.rs:93-106, 220-233, 299-304 |
| FND-007 | low | `WitnessValue::from_wire_json` re-reads the plugin value with `read(.., u64::MAX)` and `Limits::new(u64::MAX)`. It relies on the driver's FR-462 frame bound for resource limits, which is acceptable, but the rustdoc should say so. | qsl-replay/src/witness/value_text.rs:672-684 |

Verdict: no major findings. Mergeable once the gates are green. FND-001 and FND-002 are cheap test additions, so fix them in this PR.
