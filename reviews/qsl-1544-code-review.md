---
id: SR-1544001
title: "code-review and rust-review of QSL-1544 checked_views (quire-spec-language PR 2156)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@51f6c0934b0bd7a187652e693e804a2d62432f05; qsl-package/src/checked_views/**, qsl-package/src/emit/tests/checked_views.rs, tests/it/checked_views.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1310
    type: references
---
# SR-1544001: code-review and rust-review of QSL-1544 checked_views (PR 2156)

## Summary

Ticket: QSL-1544. Reviewed `git diff origin/main...HEAD` (merge base 2556b4b59, head 51f6c0934)
against FR-515 AC-1..AC-17 and TC-1310 rows 1..17.

Gates run on the PR head, all green: `cargo fmt --check`; clippy `-D warnings` over the workspace
(default, `--no-default-features`) and the root crate with `--features checked-handoff`;
`arch-lint qualified-core` and `api-surface --qsl-only`; the 13 `checked_views` unit tests and the
recipe test in `tests/it`; 16 qsl-package doctests; the root `protocol_artifact` doctests (AC-9
control compiles, both `use` lines fail E0432). Full `cargo test -p qsl-package --no-fail-fast`:
30 failures on the PR and the same 30 on origin/main 934b25c3f (diff of the failure lists empty;
QSPEC_DIR=/Users/peter/dev/quire-specification at b1feb4841). The brief expected 27; the measured
baseline is 30 on both sides.

Gated cases: AC-13 and AC-14 are `#[ignore]`d with real bodies and cite IR-1279 and IR-1278. Run
with `--ignored`, AC-13 fails at `admit` with `InvalidPackage`/`TimedNodeReference` at
`/temporal_bindings/1` (the stated reason) and AC-14 fails at the `unimplemented!` helper
(`view_name`, `predicate_name`). The helpers are called only from the ignored test, so no
non-ignored test reaches them. The pattern is acceptable under the planner's ruling.

Mutation testing of the view logic (19 mutants, each run against the 13 unit tests): killed 16.
Killed: requirements empty, clock None, interval dropped, guard None, `belongs_to` always true,
one occurrence only, `predicate_leaves` deduplicated, kind order swapped (holds before guard),
LeafHasNoSource returns another variant, parent_operation None, binders empty, boolean check off,
reachability check off, `over` wrong, reachable-term scope widened. Survived: (a) `leaf_source`
returning the first entry (FND-001), (b) the activation guard removed from the reachability roots
(FND-003), (c) `temporal_subject` accepting any `Temporal(_)` kind; (c) is equivalent in effect
because the argument-shape reads that follow still refuse a formula node, so it is not a finding.

No forked reader (the view reuses `resolve_clause_root`, `wire_graph`, `role_spelling` from
`checked_v2`), no IR code, no pins, no `#[allow]`, no unwrap or expect on a production path (the
only `unwrap` in the diff is inside doctests). Shared-file edits are minimal: helpers made
`pub(super)` in `tests/it/checked_handoff.rs` and `emit/tests/temporal_clause.rs`, a one-line
module in `emit/tests.rs`, and `mod checked_views` behind `checked-handoff` in `tests/it/main.rs`.
The AC-12 seam is sound: `checked_predicate` is a one-line pass-through of
`package.source_map()` to `read_predicate`, and the test removes the leaf's entries from a copy of
the real map, so the real `leaf_source` -> `LeafHasNoSource` path runs. Compile-fail doctests all
have compiling controls and fail for the stated codes (E0451, E0599, E0505, E0432); each passes
under rustdoc's expected-code check.

Ticket: QSL-1544.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The selection of a leaf's source entry "inside the clause's formula root" (AC-4, AC-16, AC-17) is not pinned by any test. Mutating `leaf_source` to return the first entry for the leaf leaves all 13 tests green. Every test uses a single clause or a leaf with one entry, so the discriminating case (a leaf node shared by two clauses, one entry per reading, picking the entry inside this clause's formula) never runs. Add a two-clause package whose clauses share a `holds` operand and assert each view's span lies inside its own formula root and differs between the two. | qsl-package/src/checked_views/predicate.rs:236-256; qsl-package/src/emit/tests/checked_views.rs:452-520 | correct-requirement-no-evidence |
| FND-002 | medium | `formula_reads` expands the clause's formula tree once per occurrence, with no visited set and no budget, and `temporal_subject` does it eagerly. FR-337 makes equal terms share one node, so a package whose formula nodes reference a child twice at each of k levels yields 2^k `OperatorUse`/`predicate_leaves` entries from O(k) nodes. The caller holds a package from a reader, so this is reachable with hostile bytes. FR-515 states no bound or charge. Either charge/bound the expansion (refuse with a variant) or state the pre-order-per-occurrence expansion and its cost in FR-515. Not reproduced with a crafted package; derived from the code. | qsl-package/src/checked_views/subject.rs:402-440 | missing-requirement |
| FND-003 | low | AC-5 defines `LeafNotInClause` as a leaf "reachable from neither the formula root nor the activation guard", and the code adds the guard as a reachability root, but no test selects a Boolean sub-term of the guard (for example `when (a and b)` and select `a`). Removing the guard from the roots leaves every test green. | qsl-package/src/checked_views/predicate.rs:158-163 | correct-requirement-no-evidence |
| FND-004 | low | Each `temporal_subject` call rebuilds a `Graph` position map and a `wire_graph` of the whole package, and `checked_predicate` builds a second `Graph`, so a caller reading every leaf of every clause does O(clauses x leaves x nodes) work. No test or requirement states the cost; a cached index on the package would remove it. | qsl-package/src/checked_views/subject.rs:222-226; qsl-package/src/checked_views/predicate.rs:130-134 | missing-requirement |

## Verdict

No high finding. The views are correct against the ACs on the paths tests exercise, and the gated
pattern is acceptable. Two medium findings need a fix in this PR: FND-001 (a missing
discriminating test, small) and FND-002 (decide and state or bound the expansion). FND-003 and
FND-004 are low.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-005 | low | The `FormulaTooLarge` refusal exists (public `ViewRefusal` variant, `FORMULA_VISITS` = 1,000,000) but FR-515's `ViewRefusal` list ("the variants of this requirement are") and ACs do not name it or the budget; the only test is tagged AC-3. State the variant and the budget in FR-515 (and an AC, or tag the test to a new one) so the closed enum and the spec agree. | qsl-package/src/checked_views.rs:46-50; spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md | missing-requirement |
| FND-006 | low | The budget counts `OperatorUse` visits, not memory: each visit retains an `OperatorUse` and each `holds` clones a `CheckedNodeId` (a boxed package name plus a heap path vector), so the ceiling at 1,000,000 visits is linear but large (order hundreds of MB, estimated, not measured), and 1,000,000 is the reader's read-work default, which bounds reading the package, not expanding it. Sound against exponential growth (the check precedes each expansion and exits early), but the ceiling is generous. Non-blocking. | qsl-package/src/checked_views/subject.rs:19-23,458-462 | missing-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `a_shared_operand_reads_the_span_inside_its_own_clause` (two clauses sharing a `holds` operand, one entry per reading, each view's span inside its own formula root, spans differ) is red under the first-entry `leaf_source` mutant (re-run at c5611aa24). after_excerpt: `entries.find(|entry| entry.regions.iter().all(inside)).or(first)` is now pinned by that test. |
| FND-002 | fixed | `formula_reads` checks `uses.len() >= visits` at the top of every pop, before the node is expanded or pushed, and returns `FormulaTooLarge`; pending grows by one node's arguments per visit, so work and memory are linear in the budget. `a_formula_whose_expansion_exceeds_the_budget_is_refused` (8 entries ok at budget 8, refused at 7) is red with the check disabled, with `>` for `>=`, and with the check removed. The spec side is FND-005; the memory ceiling is FND-006. |
| FND-003 | fixed | `a_sub_term_of_the_activation_guard_is_a_clause_leaf` selects each guard sub-term as a `ClauseLeaf` and refuses it for another clause; red under the guard-root-removed mutant. |
| FND-004 | accepted-no-change | `checked_predicate` now builds the node index once per call (`subject_in` shares the `Graph`). The remaining cost (a `Graph` and a `wire_graph` per call, so O(clauses x leaves x nodes) for a caller that reads every leaf) is linear per call, FR-515 states no complexity bound, and a per-package cache would need interior state in a borrowed read-only package. Acceptable. |
