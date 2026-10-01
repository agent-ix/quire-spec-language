---
id: SR-923
title: "QSL-346 code review (with rust-review lane) of PR 546, TC-166 typed QualifiedName selection tests"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6260464f3408e75aea2126a681f638b933690474; qsl-replay/src/execute.rs (replay doc: compile_fail doctest and compiling twin); qsl-replay/src/call_site.rs (call_site doc: compile_fail doctest and compiling twin; tc_166_call_site_refuses_an_unresolvable_qualified_name); qsl-replay/src/execute/tests.rs (tc_166_an_unresolvable_qualified_name_refuses_unknown_function); qsl-replay/src/execute.rs select and call_site.rs Locate for QualifiedName (unchanged, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/TC-166
    type: reviews
---
## Summary

Ticket: QSL-346. PR: quire-spec-language#546 at 6260464f. Test-only change plus spec status.

Doctests. Each `compile_fail` block differs from its compiling twin in exactly one
line, checked by extracting both blocks (hidden lines included) and diffing them:
`selected_function: "small"` vs `selected_function: small` in `replay`'s doc, and
`"f"` vs `&f` in `call_site`'s doc. The coder's standalone compiler logs show the only
errors are the `&str` ones: E0308 "expected `QualifiedName`, found `&str`" for
`replay`, and E0277 `str: CallSiteSelection` / `str: sealed::Locate` / `str: Sized` for
`call_site`. All three E0277s come from the same `"f"` argument. qsl-replay sets no
`#![doc(test(attr(deny(warnings))))]`, so the unused `f` in the call_site
`compile_fail` block is a warning and cannot be what makes the block fail. Stable rustdoc does not
enforce the `E0308`/`E0277` header codes, so the compiling twins are what bound the
failure. Both twins compile and pass.

Refusal tests, mutation-checked in a throwaway worktree (since removed):
- Case-insensitive lookup (`callable(&segment.as_str().to_lowercase())` in both
  `select` and `Locate`): both new TC-166 tests fail. The older
  `tc_444_a_selection_naming_no_function_refuses` and
  `call_site_refuses_an_unknown_function_name` still pass under this mutant. So the
  `Small` case is what the PR adds.
- Last-segment lookup (`segments().last()` in place of `let [segment]`): both new
  tests fail, and tc_444 fails too.

Focused runs on 6260464f: both `tc_166` unit tests and the four doctests pass.
Gate: coder's make ci on 6260464f reports exit=0 (scratchpad/qsl-346-ci.log, not
re-run).

Rust-review lane: test-only code. Uses let-else with a `panic!` that names the
input. Asserts the exact selection, package and `Code::MissingDeclaration`. No
tautology, no production `unwrap`, no new pub surface.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `tc_166_an_unresolvable_qualified_name_refuses_unknown_function` repeats `tc_444_a_selection_naming_no_function_refuses` almost exactly. Two of its three inputs (`large`, `module::small`) and its two assertions are the same; only the `Small` case and the `code()` assertion are new. `tc_166_call_site_refuses_an_unresolvable_qualified_name` overlaps `call_site_refuses_an_unknown_function_name` in the same way. Fix: add `Small` (and the code assertion) to the existing tests and add the TC-166/AC tags there, or drop the duplicated inputs from the new tests. | qsl-replay/src/execute/tests.rs:507-527, 536-560; qsl-replay/src/call_site.rs:625-679 |

## Verdict

Correct. The doctests fail only because of the `&str`, and the refusal tests kill both a
case-folding regression and a module-path-ignoring regression. One low duplication
finding. Mergeable as is; FND-001 is optional cleanup.

## Dispositions

Round 1, reviewed at dd5bad9eddc2028a16c1be66346fecc922797ace (fix commit dd5bad9e). Coder's make ci on dd5bad9e: exit 0 (not re-run). I checked the folded tests line by line against the deleted ones. `tc_444_a_selection_naming_no_function_refuses` now takes `large`, `Small` and `module::small` and asserts the selection, the recompiled package and `Code::MissingDeclaration`. `call_site_refuses_an_unknown_function_name` takes `nope`, `F` and `module::f` and asserts the selection and package. `nope` replaces the deleted test's `g`, and both are undeclared names. No case-sensitivity or module-path assertion was lost. The deleted test's positive `replay(small(7))` check is still covered by `tc_444_an_input_counterexample_replays_and_agrees`. Mutation re-check at dd5bad9e: with a case-folding `select`/`Locate`, both folded tests fail.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | dd5bad9e: the tc_166 duplicates are deleted; their `Small`/`F` inputs, the code assertion and the TC-166 (and FR-062-AC-10, FR-065-AC-6 on the execute test) traces moved into the existing tests |
