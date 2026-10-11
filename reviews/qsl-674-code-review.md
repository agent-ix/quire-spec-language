---
id: SR-4962
title: "Independent code and Rust review of QSL PR #721"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@288503ee7cbc7c6a1bc98ffe5dff94bdbfd3848e; base f6adf1fbccca4b2a8db00ba142300537228bebd7; qsl-eval/tests/it/model_reference_queries.rs, qsl-semantics/src/model/population.rs, qsl-semantics/tests/it/model_population.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: reviews
---
# Independent code and Rust review of QSL PR #721

## Summary

Ticket: QSL-674. Reviewed the fetched PR head `288503ee7cbc7c6a1bc98ffe5dff94bdbfd3848e` against fetched `origin/main` `f6adf1fbccca4b2a8db00ba142300537228bebd7`. `git diff --name-only origin/main...HEAD` is exactly the three intended G3 code/test files. Static review found a compile-blocking borrow-after-move in the new `PopulationBinding` literal, an unmetered input-proportional identity-encoding allocation, and one false test-to-criterion binding. No build or tests were run because gates are active; coder-reported passes were not independently verified.

The new test oracle is substantially independent of the production preimage builder: a literal seven-field empty-members vector is hashed directly, and a separate helper constructs other preimages. Behavior assertions cover member changes/permutation, maxima, Post/Pre identity, and evaluator recording/resolution. No additional oracle weakness was found in the reviewed fixtures. The test carrying the false AC-4 tag only verifies foreign-type refusal.

## Verdict

Changes requested: one high compile blocker and two medium findings. `git diff --check origin/main...HEAD` passed. No source edits, builds, or tests were made/run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Population admission moves the member map before borrowing it to mint the identity | qsl-semantics/src/model/population.rs:1243-1254 |
| FND-002 | medium | Identity construction allocates and sorts unbounded member encodings outside admission charges | qsl-semantics/src/model/population.rs:782-787 |
| FND-003 | medium | Foreign-type refusal test is falsely tagged for conflicting-identity AC-4 | qsl-semantics/tests/it/model_population.rs:3287-3307 |

## Finding details

### FND-001

In the `PopulationBinding` struct literal, `members: admitted` moves the `BTreeMap`; the later `population_id` field then passes `&admitted`. The source-order evaluation therefore borrows a moved value and fails Rust's borrow check (E0382) on the successful admission path. Compute the id before the literal or otherwise avoid using the moved map. The claimed workspace check at this SHA conflicts with this static finding and was not rerun while gates were active.

### FND-002

For accepted members with large authored identity strings, this path serializes each member using `Limits::new(u64::MAX)`, retains every resulting byte vector to sort, then includes the members in the full identity preimage. Admission charges member-count/work units, not bytes of these encodings. The work and retained memory therefore scale with input bytes without a corresponding selected admission ceiling; encoding errors also go through `expect` rather than a typed incomplete result.

### FND-003

`invalid_population_member_is_refused_before_identity_is_minted` changes a member to undeclared `foreign.Type` and asserts `ForeignType`, which exercises FR-084-AC-1. It does not exercise FR-084-AC-4's conflicting-identity refusal or duplicate collapse. Existing TC-229 tests in the same test file separately cover AC-4, so this is a false binding, not a claim that the criterion lacks all coverage.

## Scope examined

- FR-089-AC-7: member/max/pre identity distinction and independent fixed preimage oracle.
- FR-089-AC-1: identity stability and role distinctions (context for changed producer).
- FR-084-AC-1 and FR-084-AC-4: foreign-type refusal and conflicting-identity/duplicate semantics.
- NFR-012: bounded admission charges and work.
- Oracle assessment: literal vector, independent expected-ID helper, behavioral assertions, and trace tags in the changed tests.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 458a0ff32a14683bcc4c3b2c9bbb5b78774844bf |
| FND-002 | fixed | 458a0ff32a14683bcc4c3b2c9bbb5b78774844bf |
| FND-003 | fixed | 458a0ff32a14683bcc4c3b2c9bbb5b78774844bf |
| FND-001 | fixed | 458a0ff32a14683bcc4c3b2c9bbb5b78774844bf |
| FND-002 | fixed | 458a0ff32a14683bcc4c3b2c9bbb5b78774844bf |
| FND-003 | fixed | 458a0ff32a14683bcc4c3b2c9bbb5b78774844bf |
| FND-004 | still-open | The linear work estimate omits repeated-prefix byte comparisons from sorting encoded members. |
| FND-005 | still-open | The work-budget-only test still carries the identity-behavior FR-089-AC-7 trace. |

## Disposition pass 2

Reviewed fix commit `458a0ff32a14683bcc4c3b2c9bbb5b78774844bf` at PR #721 head against base `f6adf1fbccca4b2a8db00ba142300537228bebd7`. FND-001 is fixed by computing `population_id` while `admitted` is still borrowed, then moving the map into the binding. FND-002 is fixed by charging input-sized work before encoding, applying derived byte bounds to each member and the full preimage, and returning a typed refusal on encoding failure. FND-003 is fixed by removing FR-084-AC-4 from the foreign-type test; the new charge-bound test is traced to FR-089-AC-7. No new finding was identified in the reviewed fix diff.

No build or tests were run by this reviewer. The PR's visible checks are CLA-only; focused tests and the final gate have not been evidenced here. The review findings are fixed, but this disposition is not a mergeable verdict until those checks pass.

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Population identity work charge underprices worst-case member sort comparisons | qsl-semantics/src/model/population.rs:877-897 |
| FND-005 | medium | Identity-budget test is falsely traced to FR-089-AC-7 | qsl-semantics/tests/it/model_population.rs:3315-3342 |

### FND-004

`population_id_work_bound` charges a linear multiple of input string lengths plus a small member-count term, but `population_id_preimage` sorts all encoded members with a bytewise comparator. With many admitted members sharing long declaration fields and object names that differ near their ends, each sort comparison rescans a long common encoded prefix; sorting performs O(n log n) comparisons. The charge neither includes comparator count nor multiplies encoded length by a logarithmic factor, so the selected `work_units` ceiling can admit substantially more identity-sorting work than it prices. This leaves the NFR-012 work ceiling ineffective for this path's comparison cost.

### FND-005

`population_identity_encoding_is_charged_before_hashing` asserts only that a work-unit limit returns `Incomplete` before identity encoding. It does not assert any FR-089-AC-7 identity distinction or fixed digest. Its `FR-089-AC-7` tag therefore claims coverage of behavior the test does not exercise; it belongs to the NFR-012 admission-work obligation.

## Disposition pass 3

Reviewed exact pushed head `331d2ccab5e90586112c5af09ed3b5380536c186` against fresh `origin/main` base `bdf259f008c6577f4235b06ab1d87c32db83099a`. The prior compile-order defect, input-sized encoding charge/bound defect, and foreign-type test's AC-4 tag remain fixed at `458a0ff32a14683bcc4c3b2c9bbb5b78774844bf`. New FND-004 remains open because the work meter estimates input bytes linearly but does not account for worst-case repeated-prefix byte comparisons in the member sort. New FND-005 remains open because the resource-budget test is still falsely tagged to the identity criterion. The manual gap pass found no uncovered FR-089-AC-7 identity behavior: separate tests cover member/max/pre distinctions and the fixed vector; this trace error is reported as a binding defect, not a missing behavioral test.

No tests or builds were run by this reviewer. The coder reports focused tests green, but that evidence was not independently verified; hosted checks currently visible are CLA-only. The final gate remains pending, and the still-open FND-004/FND-005 make this non-mergeable regardless.
