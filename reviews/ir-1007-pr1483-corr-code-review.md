---
id: SR-1483001
title: "IR-1007 PR #1483 code-review of root-location correction commit"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; IR-1007; PR #1483 head 6ee417f64 vs base e2e7883ca; correction commit 6ee417f64 (qsl-eval/tests/it/checked_package_call.rs, qsl-eval/tests/it/total_functions.rs) plus clippy commit 3e4d547e9 (qsl-replay/src/application.rs) and rebase range-diff"
review_set: subset
---

## Summary

Ticket: IR-1007. Narrow code-review with its Rust-review lane over the correction commit 6ee417f64, which replaces two `location == None` assertions in qsl-eval with the exact checked body root `Location` that merged quire-eval #6 (a464a64f, pinned in Cargo.lock) now supplies for a denied root `function.call` charge. `git range-diff b4d665b66..af7bf0bf8 e2e7883ca..6ee417f64` shows commits 1-10 patch-identical (`=`) to the previously reviewed head af7bf0bf; commit 11 (3e4d547e9, Clippy) and commit 12 (6ee417f64) are new. Commit 11 was also read here.

## Verdict

**PASS.** The correction commit is test-only (two files under `qsl-eval/tests/it/`, no production, Cargo or CI change). The new assertions use full `assert_eq!` on `Option<Location>` against `Some(Location::root(Origin::Body { function, index: 0 }))`. `Location`'s `PartialEq` compares the origin and the whole path. So the assertions fail for `None`, for a non-root (child) path, for a different origin variant (`Expression`, `Measure`), for the wrong function name, and for the wrong declaration index. Both functions are at declaration index 0 in their fixtures (`one` is the sole declaration; `f` precedes `divide`), so the expected values are correct. Index-0 coincidence alone is not discriminating on the index, but the facade tests in `qsl-replay/src/application.rs` (reviewed earlier) already distinguish own-package from imported-package root locations. No `#[ignore]`, no loosened matcher, and trace tags stay `#[trace(...)]` and unchanged. The doc comment on `a_second_call_on_a_spent_meter_is_denied_at_its_function_call` was updated to match. The Completed-path `None` assertions (first call, `x = 0.5`, `divide`) are kept, so the completed/denied distinction is still tested. Commit 11 collapses one nested `if` into `&&`; short-circuit order is unchanged, so `other_owner.replace` still runs only when `has_node` is true. It also replaces `.err().expect(..)` with `.expect_err(..)` in a test. Neither changes behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Rust-review lane: test conventions (`#[trace]`, no `Trace:` doc lines), tautology/guarded-assertion checks, import hygiene (`quire_semantic_value::location::{Location, Origin}` added where it was missing; already imported in total_functions.rs), panic surface (test-only), and Clippy-commit semantics. No CI workflow files changed. Remote PR head was confirmed to be 6ee417f64. The coder's local rebase 67ab6cb63 (onto main 6c36ef911, not pushed) was not reviewed. Focused tests: 10 qsl-replay application::tests facade tests passed and the 2 modified qsl-eval tests passed (RUST_TEST_THREADS=4, QSPEC_DIR = read-only export of quire-specification origin/main 37a78bca).
