---
id: SR-2238001
title: "spec-review of FR-285 reads the shared ten-category catalog (quire-spec-language PR 2238)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md; spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/test-cases/TC-769-the-exit-function-maps-every-category-and-multi-item-outcome-with.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
---

# Spec review of FR-285 catalog-owned label and exit (quire-spec-language PR 2238)

## Summary

Ticket: QSL-1574. One spec-review pass (with EARS, criterion-strength and integrity lenses applied by hand) over `git diff origin/main...HEAD` at head 92629b508d56d5fab48d1273b55313a8c00def0d (FR-285, FR-330, FR-501, ADR-012, TC-769).

Verified exactly right against quire-semantic-value ab81a09 `src/catalog.rs`: ten variants in `Category::ALL` order success, violation, undefined, refusal, unsupported, incomplete, pending, indeterminate, inconclusive, internal-failure; `exit_code` 0, 10, 10, 20, 21, 22, 0, 11, 22, 30; `trace_exit_code` is `exit_code` for all ten; `as_str` spells `internal-failure`; Pending is label `pending`, exit 0. The FR-285 table, the new "catalog owns" section, AC-1/9/11/12 values and TC-769 steps 1 and 10 agree with it and with ADR-013 O-16's ten-row category table. Code on main already emits `internal-failure` (qsl-replay/src/outcome.rs:2448, 2851, 2956; qsl-foundation/src/diagnostic.rs:1153 test) and no test, fixture or doc quotes `internal failure` as a machine label; README's outcome table is a human display table (also `refused` vs label `refusal`), so it is not a conflict. ADR-029's exit table is left unaligned: agreed, it defers to FR-285 for 11 in its own text and FR-285 is the owner. "eight categories" in spec/reviews/ and reviews/ are dated review records: agreed to leave. No other spec prose says eight or has no pending/indeterminate. FR-330, FR-501, FR-283 and FR-527 text do not contradict. No new ids; SR-2238001 is unused on main and all open PR heads. `quire validate` exit 0 on all five files (module duplicate-archetype warnings only).

Spec is ahead of the code: main pins quire-semantic-value d5e2826 (eight values, `Inconclusive.trace_exit_code` = 0); PR 2231 bumps the pin to ab81a09 and updates qsl-foundation/src/diagnostic.rs tests, but leaves qsl-replay/src/spine/clause.rs `TemporalCategory`, `shared_category` and `label_and_exit`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-285-AC-11's second half, "a catalog whose `Pending` exit differs changes the reported exit", cannot be executed: `Category` is a concrete enum with const fns in an external crate, QSL has no seam to substitute a catalog, and TC-769 step 10's "search QSL for a local outcome-to-label or outcome-to-exit mapping" names no pattern or pass/fail rule. The first half (equality with `Category::Pending.as_str()` and `exit_code()`) can fail; the "no local function" half can only pass vacuously or by reviewer judgement. State a checkable form (for example a source check that qsl-replay and qsl-analyze contain no literal `("pending", 0)` or `("indeterminate", 11)` pair and no `match` over a temporal outcome yielding a label or exit) or drop the substitution clause. | spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:156; spec/test-cases/TC-769-the-exit-function-maps-every-category-and-multi-item-outcome-with.md:29 | wrong-requirement |
| FND-002 | medium | The spec now says QSL keeps no local mapping and AC-9 requires `Category::Pending` not `Category::Inconclusive`, but qsl-replay/src/spine/clause.rs on main (and on PR 2231's head) still maps Pending to `Category::Inconclusive`, Indeterminate to `Category::Undefined` in `shared_category` and holds `label_and_exit` with `("pending", 0)` and `("indeterminate", 11)`; main's pinned catalog has no `Pending`. AC-9, AC-11 and TC-769 step 10 will be red until a code ticket deletes `TemporalCategory` and reads `Category`; the spec names no such ticket. Name or file the code ticket (and order it after the pin bump in PR 2231). | qsl-replay/src/spine/clause.rs:547-585; Cargo.lock | missing-requirement |
| FND-003 | low | ADR-012's Q210-3 row drops "eight" but still writes the category as a backticked literal `internal failure` (twice: "tool failure -> `internal failure`" and "no family maps to `internal failure`"), while this PR spells the label `internal-failure` in FR-285 and TC-769. Backticks mark a literal; use `internal-failure` there or drop the backticks. Other spec prose that writes "internal failure" unquoted is the category name, not the label, and is fine. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1043 | wrong-requirement |

## Verdict

Mergeable once FND-001 is fixed in the PR (one sentence in AC-11 and step 10). FND-002 is a non-blocking spec/code ordering gap that needs a named code ticket; FND-003 is wording. AC-9 can fail (on main it reports Inconclusive for a pending clause); AC-11's first half can fail, its second half cannot (FND-001). Table, labels, exits and the internal-failure spelling are exactly right.
