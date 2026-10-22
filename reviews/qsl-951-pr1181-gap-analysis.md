---
id: SR-13761
title: 'Gap analysis of PR #1181: migrate the string_edge marker to ix_trace_rs::string_edge (QSL-951), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@9f36fcf4d; git diff origin/main...9f36fcf4d (PR #1181, branch qsl-951-migrate, merge base 7286be314) against FR-064-AC-12..AC-15 and TC-162 steps 13-16: qsl-string-edge/src/detector.rs, qsl-string-edge/tests/it/{library,command,marker}.rs, xtask/src/string_edge.rs, tests/it/string_edge_marker.rs; ticket QSL-951'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-064
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-162
  type: reviews
---

## Summary

Ticket: QSL-951. PR: quire-spec-language#1181, head 9f36fcf4d. Method: quoin gap-analysis, planless (plan completion not assessed). Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

`quire matrix --scope .` at head shows all fifteen FR-064 criteria tagged. AC-12 is bound by detector.rs:548, command.rs:422, library.rs:553 and xtask string_edge.rs:288; AC-13 by marker.rs:59 and :84; AC-14 by string_edge_marker.rs:102; AC-15 by detector.rs:566, command.rs:422, library.rs:577, string_edge_marker.rs:135 and xtask string_edge.rs:288. All production changes fall under FR-064 (the marker attribute and the scan) or are bookkeeping that follows from deleting the crate.

Mutation evidence:

- AC-14 guard. A stray `src/zz_guard_mutant.rs` was added to the root package and `--test it string_edge_marker` was rerun. `pub use ix_trace_rs::string_edge;` and `pub use ix_trace_rs::string_edge as edge;` fail the guard. These pass it: `use ix_trace_rs::string_edge as edge;`, `pub(crate) use ix_trace_rs::string_edge;`, `pub use ix_trace_rs::{string_edge, trace};`, `#[proc_macro_attribute] pub fn string_edge(..)`, `use qsl_attrs::string_edge;` and `#[qsl_attrs::string_edge]`.
- AC-13. A local copy of ix-trace-rs e29cc07 was changed so `string_edge(anything)` returns only the `compile_error!` and drops the item. The test's consumer body then gives exactly the same single error line as the real crate. rustc 1.98.1 does not cascade unresolved-name errors after an expansion error.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-064-AC-12 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-13 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-14 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-15 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 "The marker attribute" SHALL statements | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:59-86 | examined |
| TC-162 steps 13-16 and expected results | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| FR-064-AC-1..AC-11 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-064-AC-14 guard cannot fail for most of what it claims. It checks whether a trimmed source line *starts with* each forbidden string. So `proc_macro_attribute` never matches `#[proc_macro_attribute]`, `string_edge as ` never matches a `use ... string_edge as x;` line, and `qsl_attrs` never matches `use qsl_attrs::...` or `#[qsl_attrs::...]`. Only an unrestricted, unbraced `pub use [::]ix_trace_rs::string_edge` is caught. A private alias `use ix_trace_rs::string_edge as edge;`, a `pub(crate) use` re-export and a braced `pub use ix_trace_rs::{string_edge, ..}` all compile and pass the guard. That contradicts the AC's "no re-export or alias of one". `sources` also reads only `<package>/src`, so `tools/arch-lint` (no `src/`) is never searched. A defining proc-macro crate is still caught by the `proc_macro` target check. Fix: parse each production file with `syn` (the scanner and xtask already depend on it) and reject any `use` tree that renames, or re-exports at any visibility other than private, a `string_edge` leaf, and any `#[proc_macro_attribute]` fn. Walk the files of every member target from `cargo metadata`, not `src/` alone, and add the fixtures above as negative tests. | tests/it/string_edge_marker.rs:72-89,104-127 |
| FND-002 | medium | The FR-064-AC-13 argument test does not show that the item is retained. `an_argument_is_one_error_and_the_item_is_retained` asserts that the one error line is `error: #[string_edge] takes no arguments`. A marker that drops the item gives the same single line, because rustc reports no unresolved-name cascade after an expansion error, so the test passes either way. Fix: add a fixture whose marked body holds a deliberate type error, for example `#[ix_trace_rs::string_edge(anything)] fn marked() -> u8 { "not a u8" }`. Assert that both the marker error and `error[E0308]` are reported (two errors with ix-trace-rs e29cc07, one with an item-dropping marker), and keep the current fixture for the one-message assertion. | qsl-string-edge/tests/it/marker.rs:84-104 |
| FND-003 | low | For the command and the xtask adapter, FR-064-AC-12's "an unmarked function in the same fixture is reported" is not exercised. Their marked fixtures hold only the three marked functions and assert a clean result. The command checks the reported case on a separate `lookalike/` tree, and the adapter on a rewritten look-alike file. Only the detector and library tests put an unmarked `plain` beside the marked ones. Add an unmarked function to the command's `marked/src/lib.rs` and the adapter's first fixture, and assert that exactly it is reported. | qsl-string-edge/tests/it/command.rs:422-469, xtask/src/string_edge.rs:288-328 |

## Verdict

Not mergeable as is: FND-001 is high, because the AC-14 test cannot detect the alias and re-export forms the AC rules out. FND-002 leaves AC-13's "retaining the item" half unproven. AC-12 and AC-15 are met: the detector, library, command and adapter all accept the three paths and report `my_macros::string_edge`, `// string_edge` and `/// string_edge`, and the cargo-metadata check finds `ix-trace-rs` among the normal dependencies of every marker-using package. FND-003 is a low coverage gap in AC-12's command and adapter evidence.
