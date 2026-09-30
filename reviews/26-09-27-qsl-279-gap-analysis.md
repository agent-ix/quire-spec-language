---
id: SR-764
title: "QSL-279 gap analysis of PR 504 (FR-105 state node emission, FR-108 boundary cases)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-105-emit-state-nodes.md; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/functional/FR-088-clause-name-and-type-identity.md (AC-4); spec/test-cases/TC-462-s4-emits-state-nodes.md; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md; spec/test-cases/TC-466-s6a-evaluates-state-clauses.md; spec/test-cases/TC-469-configversion-spine-corpus-matches-native.md; qsl-replay/src/spine/clause/tests.rs; qsl-semantics/src/check/identity.rs; tests/it/config_version.rs; examples/config-version/cases.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-462
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-463
    type: references
---
## Summary

Ticket: QSL-279. PR: quire-spec-language#504. There is no plan
bundle, so the scope is the ticket (FR-105 with TC-462/TC-463, FR-108 with
TC-469) as the team leader narrowed it. QSL-307 takes the I2 read-back
(FR-105-AC-3), and QSL-308 takes TC-469 and 13 of FR-108's 17 cases.

FR-105 AC coverage at head, measured against the test bodies:

| AC | Status |
| --- | --- |
| AC-1 | Covered by `s4_emits_exactly_the_fr_105_state_nodes`: node set, kinds, anchor operation, frame `modifies`/`creates`/`deletes`, forbidden forms. |
| AC-2 | Partial. The body shape of `state_clause`/`operation_anchor`/`frame` and the frame's `semantic_type` are checked. Not checked: each node's `dependencies` and occurrences; the `state_clause` `semantic_type`; that `VersionUnchanged`'s anchor argument names the anchor node and each invariant's names ConfigVersion; step 3's condition terms (`quire.op.state.pre`, `quire.op.model.reaches_field` with the `parent` field member). |
| AC-3 | `#[ignore]`d, and the ignore is justified: QSL-307. |
| AC-4 | Partial. Double-compile identity only. Not tested: rename changes no node id; `<=` changes the node id and `package_id`; `ParentOrder2` adds a second `claim` occurrence, ordinal 1; a second `post` adds one `state_clause` and no second anchor or frame; the `Sub` subtype shares one anchor. |
| AC-5 | Covered by `triple_mapping_is_total_and_injective_over_all_seven_variants` together with the TC-250 table test. |
| AC-6 | Not implemented: no fault-injection hook and no all-or-nothing test. |

FR-108 (this PR's slice): the four boundary cases are wired into the native
corpus loop and give distinct outcomes (SR-763 item 5). The rest belongs to
QSL-308.

Neither QSL-307 nor QSL-308 names the AC-2, AC-4 and AC-6 remainder. QSL-308
even says "TC-462/463 verified". The PR body calls TC-463 partial only
because of the I2 read.

## Verdict

Changes requested. FND-001 is high: three of FR-105's six ACs are partly or
wholly unverified, and no ticket tracks them. Most of it is cheap to test
with the fixture this PR already builds. FND-002 leaves TC-466 on a stale
"Pending QSL-279" marker that this PR was meant to lift. FND-003 is
traceability hygiene.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-105-AC-2 (dependencies, occurrences, anchor arguments, condition terms), AC-4 (rename, `<=`, `ParentOrder2`, second `post`, `Sub` anchor) and AC-6 (fault-injected all-or-nothing emission) have no tests, and no follow-up tracks them. QSL-307 covers only AC-3, and QSL-308 only TC-469/FR-108. QSL-308's description says "TC-462/463 verified". Scenario: an emitter that gives a renamed clause a new node id, or mints a second anchor for a second `post`, or emits a partial package when the frame arm faults, passes every test at this head. Fix: add the AC-2/AC-4 assertions in this PR (they reuse `config_version_compiled()` with unit edits), and either add AC-6's hook or file a named follow-up. Correct QSL-308's text either way. | qsl-replay/src/spine/clause/tests.rs:2676-2865; spec/test-cases/TC-462-s4-emits-state-nodes.md; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md; spec/functional/FR-105-emit-state-nodes.md |
| FND-002 | medium | TC-466 steps 2 and 3, and step 1's `VersionUnchanged` sub-case, are still marked "Pending QSL-279". QSL-279's own ticket comment says this ticket must unblock them. The PR removes the blocker: `post VersionUnchanged` is in the shared fixture, and the fixture's doc now says it is "reachable through `run_clause` end to end". But no test runs it, and the TC-466 Status section is unchanged. Scenario: a `pre(...)`-reading postcondition that evaluates wrongly through S6a stays undetected while the spec says it is waiting on a merged ticket. Fix: add the TC-466 step 2 run_clause tests (at least `VersionUnchanged` over an unchanged and a changed invocation), or repoint the Pending marker to a real open ticket. | spec/test-cases/TC-466-s6a-evaluates-state-clauses.md:60-77; qsl-replay/src/spine/clause/tests.rs:1587-1599 |
| FND-003 | low | Traceability. (a) FR-088-AC-4 (TC-250) now traces only to the four-variant table test. The seven-variant totality and injectivity test that AC-4 as amended requires carries only `#[trace("TC-462", "FR-105-AC-5")]`. Add `#[trace("TC-250", "FR-088-AC-4")]` to it. (b) The Status sections of FR-105, TC-462 and TC-463 still say "pending STD-111". STD-111 has landed, and this PR implements the emission. Update them to the measured state (AC-3 waits on QSL-307, and what else remains). | qsl-semantics/src/check/identity.rs:625-652; spec/functional/FR-105-emit-state-nodes.md; spec/test-cases/TC-462-s4-emits-state-nodes.md; spec/test-cases/TC-463-s4-state-package-reads-back-and-is-stable.md |

## Coverage

- Tests with new trace tags: TC-462/FR-105-AC-1, AC-2 (emission test),
  TC-462/FR-105-AC-5 (triple test), TC-463/FR-105-AC-4 (stable compile),
  TC-463/FR-105-AC-3 (ignored, QSL-307). TC-250/FR-088-AC-4 is kept on the
  table test.
- Boundary cases: `BoundaryZero`, `BoundaryMax`, `BelowRange` and
  `AboveRange` are in `CASES` and in the exhaustive `expected()` oracle, and
  run in `actual_native_commands_execute_named_semantics_and_distinct_case_identities`
  and in `actual_markdown_and_native_cases_agree_without_reusing_source_identity`.
- Underspecified code: none. Every new production arm traces to FR-105,
  FR-340, FR-341 or FR-342.
