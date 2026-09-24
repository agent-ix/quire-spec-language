---
id: SR-621
title: "Integrity review of the QSL-148 FR-065 resolution delta and the FR-067-AC-3 restatement"
type: SpecReview
analysis: integrity
scope: "Uncommitted changes over bd1eba54 on spec/148-fr065-reconcile: FR-067-AC-3, FR-067-CON-4 and the FR-067 Status note; TC-167 step 4; the ADR-012 §14.1 QSL-148 row; docs/family-migration-recipe.md Removal condition; FR-065 and TC-376 Status; comment and trace-tag edits in qsl-forms/src/dispatch.rs, qsl-semantics/src/check/check.rs, check/typing.rs and family.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-167
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-376
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
---

## Summary

The FR-065 side of the delta is consistent.

- The four TC-376 tests now carry `#[trace("TC-376", "FR-065-AC-4")]`
  (`family.rs:1614`, `:1641`, `:1667`, `:1695`). FR-065 Status, TC-376 Status
  and the spec/tests.md row all say the same thing, which closes SR-618
  FND-001.
- The `infer_form` doc (`typing.rs:579-593`), the `check.rs` module doc and
  `Application`'s doc (`family.rs:635`) cite FR-065-CON-3, the amended AC-4
  and AC-5, and ADR-012 §4.3. They no longer claim AC-5 is unmet, which
  closes SR-619 FND-005. The code edits change comments and trace tags only.
- The rewritten ADR-012 §14.1 QSL-148 row names typing and definedness,
  FR-065-CON-2, CON-3 and OQ-1. It calls termination an open owner question
  and does not decide it.

FR-067-AC-3 is now a behavioural criterion. Its no-entry half is backed by
`no_dispatch_entry_refuses_a_clean_cst_with_no_matching_leading_token`,
which fails if `build_form` stops refusing an unmapped token. Its entry half
is backed by `happy_path_builds_a_form_and_recovery_refuses_the_same_entry`.
FR-067-CON-4 restates ADR-012 §4.3's thin-seam rule as a Design/Inspection
constraint, the same way FR-065-CON-3 does. TC-167 step 4 matches both
tests.

The findings are text elsewhere in the spec that still describes the old
shape test or the old wording, a no-entry fixture that FR-091 will turn
into an entry, and the shape test that is still tagged to TC-167.

Not measured: I did not build or run the tests. The worktree has no target
dir, and the code edits change only comments and trace-tag arguments.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-091 still relies on the retired shape test. It says "Each entry makes exactly one call into the `Value` family's production function. FR-067-AC-3's shape test covers every entry in the table." FR-067-AC-3 is now behavioural, and the shape test backs no criterion and is due for deletion. Fix: FR-091:154-155 should read "Each entry makes exactly one call into the `Value` family's production function (FR-067-CON-4, verified by inspection)." If FR-091 wants each of its four entries covered by a behavioural test, add an AC that each leading token (`function`, `type`, `record`, `tuple`) returns its production's form. | FR-091:154-155; FR-067-AC-3, FR-067-CON-4 |
| FND-002 | medium | FR-067-AC-3 and TC-167 step 4 use `record` as the token with no dispatch entry. FR-091's `Value` table makes `record` an entry (`RecordDeclaration`). When FR-091 lands, `no_dispatch_entry_refuses_a_clean_cst_with_no_matching_leading_token` fails, and the AC's example becomes false. "(in M-3a, `record`)" records this but does not prevent it. Fix: use a spelling no family will claim, such as an identifier like `zz_no_entry` that is not a keyword. Change AC-3, TC-167 step 4 and its expected result, and the test fixture in the QSL-148 implementation PR. | FR-067:210; TC-167 step 4; dispatch.rs:455-466; FR-091:147-152 |
| FND-003 | low | `dispatch_entry_is_a_single_thin_call` is still in the tree and still tagged `#[trace("TC-167")]`, but TC-167 has no step it implements. The trace makes a shape test count as TC-167 evidence, which the testing policy rules out. The FR-067 Status note puts its deletion off to a later PR, but it is one test function and needs no other code change. Fix: delete the test in this change. Then update the `qsl-forms/Cargo.toml` comment on the `syn` dev-dependency ("`dispatch`'s TC-167 test parses its own source ..."), because `syn` stays only for TC-398. Remove the "remaining work" sentence from FR-067 Status. If the deletion stays deferred, at least drop the `TC-167` tag now. | dispatch.rs:334-383; qsl-forms/Cargo.toml:24-26; FR-067:289-291 |
| FND-004 | low | The entry half of FR-067-AC-3 does not tell dispatch through the table apart from a hard-coded result. The stub production returns the constant `Expression::Boolean(true)`, so a `build_form` that returned `Expression::Boolean(true)` for every mapped token without calling `dispatch` would still pass. The half also adds nothing to FR-067-AC-1's happy branch. Fix: have the stub derive its result from the CST it receives, for example `Expression::Boolean(cst.tokens().len() % 2 == 0)`, or give the test two stubs over two probe spellings. Assert that each fixture returns its own stub's value. Or drop the entry half and let AC-3 state only the `NoDispatchEntry` behaviour. | FR-067:210; dispatch.rs:280-283, :305-309; FR-067-AC-1 |
| FND-005 | low | The migration recipe's Removal condition says "a migrated form receives the same verdict from every entry point that checks it". This is the wording SR-619 FND-001 found false, and FR-065 has already narrowed it: `Application`'s own verdict (callee, arity, parameter types) is the same everywhere, and each argument follows the entry point's clause kind. Fix: "a migrated form's own family check gives the same verdict from every entry point that reaches it; nested forms are checked under that entry point's clause kind (FR-065's Behavior section)". | docs/family-migration-recipe.md:103-105; FR-065:113-124 |
| FND-006 | low | The split Description SHALLs (SR-620 FND-003) add two grammar warnings that were not in this review set's first validate run: `quire validate` reports `ears:unclassifiable` and `ears:missing-subject` at FR-065 line 29. The sentence "QSL SHALL reach callers only through the checked-package producer" wraps `QSL` and `SHALL` across two lines, and it makes QSL the thing that reaches callers, when the forms are what reach them. Fix: "QSL SHALL expose these forms to callers only through the checked-package producer this requirement builds (Behavior: the public API and #240's precondition)." Keep the subject and `SHALL` on one line, and run validate again until FR-065 has no warnings. | FR-065:22-32 |

## Resolution

- FND-001: FR-091 cites FR-067-CON-4, verified by inspection, in place of the
  shape test.
- FND-002: FR-067-AC-3 names `record` only as the M-3a example of a spelling
  with no entry. FR-067 Status and FR-091 record that FR-091's `record` entry
  moves the test fixture to a spelling no family claims, in the change that
  adds the entry. This PR changes no fixture code.
- FND-003: the `TC-167` tag is removed from `dispatch_entry_is_a_single_thin_call`;
  deleting the test stays remaining work in FR-067 Status (this PR's code
  changes are limited to comments and trace tags).
- FND-004: FR-067-AC-3 states only the `NoDispatchEntry` refusal; building a
  form through an entry is FR-067-AC-1. The `FR-067-AC-3` tag is removed from
  `happy_path_builds_a_form_and_recovery_refuses_the_same_entry`.
- FND-005: the recipe says a form's own family check gives the same verdict
  from every entry point that reaches it, with nested forms checked under the
  entry point's clause kind.
- FND-006: each Description SHALL starts its own line with subject `QSL`, and
  the third reads "expose these forms to callers"; FR-065 has no validate
  warnings.
