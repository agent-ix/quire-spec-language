---
id: SR-1228
title: "QSL-484 gap analysis of PR #607 (FR-262-AC-2, ValueType half)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3147f80640a171fe6e30ce049c0751eca382e92c; PR #607 diff against origin/main; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md; spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md (D-1 items 2-3, D-4.7)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: reviews
---
## Summary

Ticket: QSL-484. PR: quire-spec-language#607. This was a manual AC-to-test
check. The PR has no plan bundle, so there is no Test Matrix to reconcile.

Units examined:

- **FR-262-AC-2, ValueType half.** The AC reads: "A `ValueType` of 100,000
  nested `Option`s around `Boolean` clones, compares equal to its clone,
  hashes equal to its clone, formats for debug and drops on the same thread."
  `tc_735_a_deep_option_type_clones_compares_hashes_formats_and_drops` covers
  every verb at `DEEP = 100_000` on a 512 KiB thread. The `Debug` check counts
  100,000 opening and 100,000 closing parentheses, so the output is not
  truncated. It also asserts `value_type != Option(copy)`, so equality
  depends on depth. Trace `#[trace("TC-735", "FR-262-AC-2")]`: correct. The
  Value half of AC-2 (the recursive list value and its state key) belongs to
  another slice, and this PR does not claim it.
- **FR-262 Behavior 3** (ValueType clones, compares, hashes, formats and
  drops without native recursion, including on no_std). Covered by the
  tests above, and `make quire-exact-no-std` builds.
- **Ticket AC "Each new heap stack's growth is covered by an existing node
  or byte charge (test)"** (ADR-030 D-1 item 3). The only new heap stack is
  the `Debug` worklist.
  `tc_735_the_debug_worklist_grows_by_a_constant_per_type_node` bounds its
  peak at 3 steps per type node plus 4, measured at 100,000 links for both
  nesting variants. I checked the upstream charge in the code. A source type
  is bounded by S1's `Limits::nodes`, `tokens` and `source_bytes`
  (qsl-cst/src/lexer.rs:21-36). A checker `Option` wrap is charged by
  `Typer::enter`'s node limit. A replayed type is bounded by
  `MAX_ENCODED_BYTES`. The AC is met.
- **Ticket AC "No compatibility layer, shim, re-export or fallback".** There
  is none. The derives are replaced in place.
- **Ticket AC "The tests in this PR back FR-262 and pass; nothing beyond
  them".** All four tests trace FR-262-AC-2. The two support tests (Debug
  matches the derive, and equality/hash per variant) protect the replaced
  derive's behaviour for this AC's verbs. They are not extra scope.

Code traced to spec: every new item in value_type.rs and the two
`CollectionType` accessors serve FR-262 Behavior 3. No code is left without
an owning requirement.

Test-oracle strength: the Hash oracle is weak. That is recorded as FND-002
in the code review (SR-1227), not repeated here.

## Verdict

Clean. Every examined AC has a test that would fail if its behaviour broke.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
