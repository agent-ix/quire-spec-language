---
id: SR-16323
title: "Code review of quire-spec-language PR #1394 hybrid emission and read-back"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1017 PR #1394 against qsl-1017-pr2; qsl-forms/src/{hybrid_declaration,syntax}.rs; qsl-package/src/{checked_v2,emit,lib}.rs and checked_v2/hybrid.rs and emit/tests/hybrid.rs; qsl-semantics/src/check/{assemble,check,hybrid,lowering,mod,node_key}.rs and value/member.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Code review of PR #1394

## Summary

Ticket: QSL-1017. Reviewed the PR's own Rust diff, its call sites and tests, FR-447-AC-5, AC-12 through AC-16, and QSpec FR-481's hybrid node contract. The code review includes the rust-review lane. The final aggregate gate belongs to the code lead.

## Verdict

**FAIL.** A `fair S` claim whose transitions all target one mode emits the wire member reserved by QSpec FR-481 for `fair S::T`. The package reader can admit this member because it denotes the same transition set, so successful admission does not establish fidelity to the source form. The edit to FR-447 attempts to permit this encoding, but the cited QSpec contract still distinguishes the two spellings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `hybrid_fairness` chooses a mode-node/target member whenever all transitions have one target, including source `fair S`. QSpec FR-481 requires model-node/mode for `fair S` and mode-node/target only for `fair S::T`. Because `HybridFairness` retains only transitions, the checker cannot tell these forms apart. Preserve the distinction in the checked form or obtain and land a QSpec contract change before claiming FR-481 conformance. | qsl-semantics/src/check/lowering/hybrid.rs:433; qsl-semantics/src/check/hybrid.rs:348; QSpec FR-481:186-192 |

## Coverage

Examined FR-447-AC-5, AC-12, AC-13, AC-14, AC-15, AC-16 and the FR-447 S4 emission clause; QSpec FR-481 hybrid model nodes, hybrid claims, fairness members and reader order. Inspected the changed emitter, read-back, checked-form, wire-member, and tagged-test paths for the introduced behavior, including API exposure, malformed-input handling, typed boundaries, resource limit, and trace tags. No CI workflow file changed. No full aggregate gate was run in this review lane.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `03ed8c629` records `named_target: Option<ModeIndex>` in `HybridFairness`, sets it from the source `::` form, emits the model-node/mode member for `None`, and reconstructs it on read-back. The two source spellings now have distinct asserted wire members. |
| FND-002 | still-open | Found in this pass at `1511a4e09`. No fix has landed. `cargo fmt --all` must be applied to `qsl-package/src/lib.rs`. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The merge resolution of the `checked_v2` re-export in `qsl-package/src/lib.rs` is not rustfmt-clean: `UnsupportedWire,` is left alone on one line and `V2ReadLimits, V2ReadRefusal,` on the next. `cargo fmt --all -- --check` reports `Diff in qsl-package/src/lib.rs:34`, so the repo gate (Makefile lines 42 and 206) fails at head `1511a4e09`. Run `cargo fmt --all`; no semantic change is needed. | qsl-package/src/lib.rs:33-42 |

## Disposition pass 2 scope

This was a narrow re-review of head `1511a4e09` after the base was retargeted to `main` (`90294374b`, the #1392 squash). `git diff origin/main...HEAD` covers 25 files, all of them #1394's own. The `origin/main` tree is identical to the final pr2 tip `4e7bdb23`. The diff from `3c2b04a1b` to `1511a4e09` contains exactly STD-449 (#1422): `checked_v2/tests.rs`, `Cargo.lock` and two spec lines. STD-449's hunks reverse-apply cleanly at HEAD, so nothing from main was lost. The `qsl-semantics` and `qsl-forms` files are byte-identical to `3c2b04a1b`. The conformance test's main-side trace tag (TC-685, FR-230-AC-36/37, QSpec TC-367, FR-447-AC-13) is kept. The `admission_corpus` change replaces the old QSL-900 "not emitted" class for `CheckedNodeKind::Hybrid` with admitted rows for all three IR `HybridForm` variants (Model, Mode, Transition) over a `Braking2` fixture. That leaves no kind classified twice or left unclassified. The `HybridCause::NotEmitted` and `with_unwritten_hybrids` removals belong to the PR and were already present at `ea936f7f0`. FR-447 trace tag counts are unchanged from `3c2b04a1b`. `quire matrix` shows FR-447-AC-5, AC-12, AC-13, AC-14, AC-15 and AC-16 as tagged. `cargo test -p qsl-package --lib` passed 259 tests and `cargo test -p qsl-semantics --lib -- hybrid` passed 16, both with QSPEC_DIR set.
