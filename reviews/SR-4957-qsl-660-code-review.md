---
id: SR-4957
title: "QSL-660 PR 677 code-review of bounded Debug counting"
type: SpecReview
analysis: code-review
scope: "QSL-660; FR-258-AC-1 and AC-5; PR 677 at 98c1d96fda5f449a944091bc4371057c913b86ed"
review_set: subset
---

## Summary

Independent static review of the final bounded Debug sink and strengthened clone oracle found no defects. Existing exact-head execution evidence supports the preserved full-depth test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for the scoped PR change. No open findings; mergeable from independent review subject to the separately owned aggregate gate.

## Evidence

- Exact local and GitHub head: `98c1d96fda5f449a944091bc4371057c913b86ed`; clean worktree. Against GitHub base `b24dbda01d56843cd14d8cab9233ec3cff67535f`, exactly one test file changes: `qsl-package/src/emit/tests/deep_bodies.rs` (55 insertions, 5 deletions). `git diff --check` passes. No production, workflow, dependency or spec changes.
- `DebugByteCount` owns only a usize. `write_str` adds each fragment's UTF-8 length with checked arithmetic and returns Ok; it has no retained String, cutoff, truncation, early-success return, or swallowed formatter error. `write!` drives the same complete derived CheckedPackage Debug implementation, and the assertion requires success and positive output length. Overflow intentionally panics in test instrumentation rather than silently wrapping.
- The UTF-8 control independently expects 13 bytes for the representative formatted text plus a three-byte character; the error control asserts fmt::Error and six prefix bytes. These are supporting instrumentation checks, not standalone proof of the deep-package criterion. The genuine 100,000-depth owner test remains the acceptance binder.
- All three sum/else-if/let cases, 100,000 depth, 512 KiB thread stack, real parse/check/lower/emit path, emitted count and stratum/key assertions, checked-body cloning/equality, body Debug and same-thread drops remain. Separate FR-264 tests are untouched.
- Full `semantic.nodes().eq(semantic_clone.nodes())` compares iterator lengths and complete SemanticNode values, whose derived Eq covers key, preimage, recursion, content and nominal data. This resolves the earlier key/body-only oracle limitation. References are keyed graph links; equality does not follow source-depth graph edges recursively.
- No mock seam, sleeps, ignored tests, warning suppression, new unsafe, production panic surface or new public API. Test-only scope and standard fmt::Write trait are appropriate.

## Existing execution evidence

Inspected the complete existing `logs/batches/qsl-l0-660-equality.log`: head receipts name this exact head before and after; fmt/compile/lint/list return 0; named owning test reports 1 passed, 0 failed, 0 ignored; terminal test_rc and time exit status are 0. This reviewer launched no build, test, Cargo command or build-lock waiter. Final aggregate gate remains lead-owned.

The log reports harness 464.73s, command wall 7:27.25 and RSS 6,924,788 KiB. The inconsistent time fields are not used to claim a speed improvement. The sink bounds retained output only; it does not bound all allocations inside Debug or the whole test. Residual roughly 6.60 GiB is explicitly outside this sink repair, not claimed fixed. PR prose limits its measured gain accordingly.

## Method scope

Code review includes the repo-required Rust review, conventional idioms, panic/unsafe, test integrity and requirement alignment. Scoped gap review examines the changed instrumentation and preserved owning acceptance test. Plan completion: not assessed. No applicable AssuranceProfile was found. Spec review/subanalyses are inapplicable because no requirements or plans changed; other language lanes are inapplicable. This is a scoped PR verdict, not a full repository assurance claim.

## Scope and bindings

```yaml
clean: true
scope:
- id: FR-258-AC-1
  path: spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md
  role: examined
  excerpt: On a thread with a 512 KiB stack, a function whose body is a 100,000-term sum, one whose body is a 100,000-long `else if` chain and one whose body is 100,000 nested `let`s each check, lower and emit under S1 and S3 limits raised to fit them. Each emitted node's key recomputed from its preimage equals its `node_id`. Every checked function body and the lowered semantic graph clone and compare equal to their clones, the checked package formats for debug, and all of them drop on the same thread.
- id: FR-258-AC-5
  path: spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md
  role: examined
  excerpt: 'Every node body lowering writes is in QSpec FR-322''s five-stratum v2 body grammar (Leaf, Group, Tuple, Member, Body): across the checked packages of TC-415 and of AC-1, no `application` argument, `aggregate` member or `binding` value holds a term of its own stratum or a higher one, no `binding` holds a `binding`, and every composite subterm is a `reference` to its own node.'
findings: []
bindings:
- test_id: a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack
  ac_id: FR-258-AC-1
  trace: correct
- test_id: a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack
  ac_id: FR-258-AC-5
  trace: correct
- test_id: debug_sink_counts_complete_output_without_retaining_it
  ac_id: FR-258-AC-1
  trace: correct
  role: supporting instrumentation control; not standalone acceptance
- test_id: debug_sink_preserves_a_formatters_error
  ac_id: FR-258-AC-1
  trace: correct
  role: supporting instrumentation control; not standalone acceptance
```
