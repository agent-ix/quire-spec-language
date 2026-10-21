---
id: SR-12284
title: "Gap analysis of quire-spec-language PR #1167 (QSL-887 remainder) against FR-147-AC-8 and TC-552"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@fdf2208cf; PR #1167 diff origin/main...HEAD; FR-147-AC-8 and the FR-147 fixture paragraph (examined), TC-552 step 8 and Fixtures line (examined), FR-147 class table rows 145-154 (context_only); resolver qsl-semantics/src/check/refinement/internal_list.rs (context_only). Includes a quick spec-review of the touched FR-147 text."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-552
    type: reviews
---
# Gap analysis of quire-spec-language PR #1167

## Summary

Ticket: QSL-887. Ticket, PR and comment text were read as data. Plan
completion: not assessed (planless). The quick spec-review of the touched
FR-147 text is folded into this file.

What was checked:

- **Each class resolves.** `the_timeout_cattempt_cend_and_activate_classes_resolve`
  resolves `Spec::Rich::Wait` (timeout), `activate Spec::Rich`,
  `Spec::Rich::Undo` (cattempt) and `cend Spec::Rich::Undo` against a
  literal expected list. Together with
  `each_class_spells_and_resolves_over_a_protocol_that_has_it` (the 13
  classes of `Full`), every class AC-8 names now has a positive vector.
  The positive oracle is strong: dropping `Template` from the `Node` arm or
  from the `Cend` arm fails it.
- **"Exactly its own class".** The `Full` test has negatives for `fork`/`join`
  (bare `par`), `spawn`/`retire` (static role), `memory` and `activate`
  (`on origin`). The `Rich` test has the `activate` negative only. A
  measured mutant, `AbstractStepKind::Cend => self.nodes.contains_key(member)`
  (so `cend` is accepted on any node), passes all 66 refinement tests. That
  is FND-001.
- **Tag honesty.** With both tests, AC-8 is fully covered, so a full
  `FR-147-AC-8` tag is now honest for the criterion. It sits on the smaller
  test only. That is FND-003.
- **Spec text against the fixtures.** The `Full` sentence (attempt, event,
  send, receive and finish nodes, a fence, a `parallel`, a channel, a static
  and a replicated role, `tso` flush) matches `full()`. The `Rich` sentence
  (an `on each` trigger, an `await` node, template `Undo` over `Spec::Try`)
  matches `rich()` and `counter_spec()`. The AC-8 row's "over `Full` and
  `Rich`" is accurate. TC-552 was not updated (FND-002), and the refinement
  name in the paragraph does not match the tests (FND-004).
- **Spec-review of the touched text.** The AC-8 row is still one testable
  criterion. Its class list is the same as before, and it matches the class
  table at FR-147 lines 145-154. There is no EARS or atomicity regression in
  the edit.
- **Ran at the reviewed sha:** `cargo test -p qsl-semantics --all-features
  --test it` gave 1160 passed. The mutant above was run in the review
  worktree and reverted.

## Verdict

Mergeable. There is no major finding. Two medium findings: there is no
negative vector for `cend`, and TC-552's fixture list and step 8 are stale.
Two low findings: the tag placement, and the refinement name in the fixture
paragraph. Fix them in this PR. None blocks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No vector shows `cend` is restricted to a compensation template. AC-8 says each entry makes "exactly its own class internal". The mutant `AbstractStepKind::Cend => self.nodes.contains_key(member)` (so `cend` is accepted on any node) passes all 66 refinement tests. Fix: in the `Rich` test, assert `cend Spec::Rich::Wait` and `cend Spec::Rich::Tried` each refuse `MISSING`, like the `Full` test's negative loop. | qsl-semantics/tests/it/refinement_protocol.rs:906-934; qsl-semantics/src/check/refinement/internal_list.rs:554 |
| FND-002 | medium | TC-552 is stale against the edited AC-8. Its Fixtures line lists only `Twice`, `CasTwice` and `Full`. Step 8 says "Resolve a value naming one entry of every class over `Full`". But `Full` has no timeout, cattempt, cend or activate class, and AC-8 now says "over `Full` and `Rich`". Fix: add `Rich` to the Fixtures line and change step 8 to "over `Full` and `Rich`". | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:27; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:44 |
| FND-003 | low | The `FR-147-AC-8` tag is only on the `Rich` test, which covers 4 of the criterion's 17 classes. The `Full` test, which covers the other 13 and the cross-class byte order, stays TC-552-only with a "Part of" doc. The matrix then binds AC-8 to the smaller half. Fix: tag both tests `#[trace("TC-552", "FR-147-AC-8")]`, and reword the `Full` doc so it is not "Part of". | qsl-semantics/tests/it/refinement_protocol.rs:827-831; qsl-semantics/tests/it/refinement_protocol.rs:902-906 |
| FND-004 | low | The FR-147 fixture paragraph says "a refinement `CasFull` selects it", and names no refinement for `Rich`. The tests use the refinement `CasTwice` (from `refinement()`, keyed by `internal()`) over both `Full` and `Rich`. Fix: say that `CasTwice`, with its abstract side set to `Full` or `Rich`, selects each, or rename the test refinement to match. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:264-271; qsl-semantics/tests/it/refinement_protocol.rs:48-50 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed | 10ee126e9 |
| FND-002 | fixed | 10ee126e9 |
| FND-003 | fixed | 10ee126e9 |
| FND-004 | fixed | 10ee126e9 |
