---
id: SR-11744
title: "Code review of quire-spec-language PR #905: reachability over a relationship end"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@90b36116b595fc690050926cbadfce42dd839a30; PR #905 diff origin/main...HEAD: qsl-semantics/src/check/{state_model,state_clause,ir,lowering,navigation,observation,claims}.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-semantics/tests/it/relationship_reachability.rs, qsl-package/src/emit/tests/reachability.rs, qsl-eval/tests/it/relationship_reachability.rs, qsl-forms/tests/it/value_forms.rs, plus the spec/, reviews/ and test files the diff also touches"
review_set: subset
---
# Code review of quire-spec-language PR #905

## Summary

Ticket: QSL-804. One pass of code-review with the rust-review lane folded in,
scoped to `git diff origin/main...HEAD` (one commit, 90b36116b, on a merge
base equal to the current `origin/main` tip b2c925a03). Spec oracle: FR-365,
TC-1046, TC-1047, TC-1048 as merged in #896, with FR-104, FR-105, FR-363 and
FR-364 as context. Only major defects are findings; mediums and lows are
recorded in the Verdict and not routed for a fix.

The single commit also rewrites files that three merged PRs changed. Every
file #898, #900 and #901 touched is back at its state before that PR, apart
from the lines this PR adds. No build was run (read-only review).

## Verdict

Changes requested: three high findings.

Examined clean: the `ReachesEnd` check order (operand conformance to the near
end, then direction, destination multiplicity, destination type). It matches
FR-365 "Checking" 2 to 5 and the C12/C16 vectors. Name resolution over the
field and the navigable ends is clean. The walked population is keyed on the
near-end type `E`. The lowering emits one `quire.op.model.reaches`
application: `Operator::Reaches`, a `relationship_end` member, no `deref`,
plain laws/leaves/mode, Boolean result. The TC-1047 tests assert every part of
that, the dependencies and the v2 reader verdict.

`evaluate_reaches_end` follows FR-365 "Evaluation" steps 1 to 6:
- the start is resolved with no charge;
- `graph.expand` is sized `value_occurrences = k`;
- an extent that is not complete is incomplete after the start's expand;
- each neighbour is resolved before its `graph.edge`, and the target is
  matched before the discovered check, so self-loops and cycles return `true`;
- `graph.result-retain` charges one work unit and one result unit.

The Warshall closure oracle in row 23 is independent of the production
traversal: it is a Boolean matrix closure, and the inverse role uses its
transpose. Production code adds no `unwrap`, `expect` or panic. Every new
`NodeKind` match is exhaustive. `via` is a grammar token (`qsl-cst`), so the
tests' field `trail` is correct, and the code does not special-case it.

Recorded, not findings:
- (medium) TC-1048 row 21 runs only one of its two orders. The expected
  outcome is computed from the production `ObjectReference` `Ord` (`if s1 < d2`).
  With one universe the type decides the order, so the undecidable-first
  arrangement is never exercised.
- (medium) The field-edge form still charges `graph.expand` and
  `graph.result-retain` as one work unit only, with no `value_occurrences = k`
  and no `result_units`. The PR leaves this unchanged.
- (low) A non-`Reference` operand refuses `operator-ineligible` before name
  resolution. So `reaches(self, i, nothing)` with an `Int` operand never
  reports `missing-name`. No vector covers this.
- (low) `population_of` does not filter out populations that have a maximum.
  This helper predates the PR and is now reused for `E`.
- (low) With no extents section at all, the start is not resolved before the
  incomplete outcome.
- (low) The author's C03 rewrite (`pre` clauses on `Rush::hop`/`hopRush`)
  and the `via`/`trail` substitution are spec-text defects, not code defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The commit reverts three merged PRs. **#901 (spec):** `spec/` FR-103/104/105/114/154 and TC-459/460/462/513 are back at their state before #901, and `reviews/qsl-808-pr901-spec-review.md` (SR-11706) is deleted. **#898 (code):** `NodeKind::Navigate.unique` is removed and `single_target` always retains `CollectionKind::Set`. A non-unique single-valued end's cardinality refusal therefore now names `Set`, which contradicts FR-364-AC-6 ("`Bag` for a `unique: false` one"); #898's eval tests in `relationship_navigation.rs` are reverted with it. **#900 (tests):** the Text-argument replay tests in `native_scalar.rs` are removed. Fix: rebase onto `origin/main` and keep only the FR-365 changes, so the diff touches no `spec/`, `reviews/`, `native_scalar.rs` or `Navigate.unique` code. | spec/functional/FR-104-check-state-clauses.md; reviews/qsl-808-pr901-spec-review.md; qsl-semantics/src/check/ir.rs:820-830; qsl-semantics/src/check/state_model.rs:156-161; qsl-eval/src/value/expression/evaluate.rs:288-305; qsl-replay/src/execute/tests/native_scalar.rs |
| FND-002 | high | FR-365 "Resolution of the third argument" and TC-1046 C06/C07 place `missing_declaration`/`missing-name` and `ambiguous_declaration`/`ambiguous-name` "at the third argument". The code refuses at the form's `location`, and the C06/C07 tests assert path `[]` (the form), so the tests lock in the wrong locus. Fix: carry the third argument's location in `Form::Reaches` and refuse there (and assert that path), or amend FR-365/TC-1046 to say "at the form" in the owning ticket. Either way, code, test and spec must agree. | qsl-semantics/src/check/state_model.rs:243, :339, :355-363; qsl-semantics/tests/it/relationship_reachability.rs:392-445 |
| FND-003 | high | TC-1046 C15's two-population refusal is a bare `CheckCause::AmbiguousName` from `ambiguous_population`, not a `StateModel` refusal. TC-1046 Expected Results says every refusal has the `StateModel` family "and the catalog code carrying the `state-model` prefix", so the emitted catalog code differs from the spec's. The C15 test checks code, cause, path and name, but not family or prefix (C01-C16 elsewhere use `assert_refusal`, which checks both). Fix: wrap the end form's population refusal in the StateModel family and assert the prefix, or amend TC-1046 so it exempts the population refusal. | qsl-semantics/src/check/state_clause.rs:399-417, :561; qsl-semantics/tests/it/relationship_reachability.rs:557-571 |

## Dispositions

Round 1, reviewed at `82857a61f54fab4491d36e7ce19b78273391e9fc` (a descendant
of `origin/main` 17fa7e46b).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 82857a61f54fab4491d36e7ce19b78273391e9fc |
| FND-002 | fixed | 82857a61f54fab4491d36e7ce19b78273391e9fc |
| FND-003 | fixed | 82857a61f54fab4491d36e7ce19b78273391e9fc |

FND-001: the diff (22 files) touches only the FR-365 code and tests, FR-365,
TC-1046/1047/1048 and the two `reviews/qsl-804-*` files. It deletes nothing
and leaves untouched qsl-replay, `Navigate.unique` / `single_target` (#898),
FR-103/104/105/114/154, TC-459/460/462/513 and SR-11706.

FND-002: the spec now matches the code. FR-365 "Resolution of the third
argument" and TC-1046 C06/C07 say "located at the form". No other FR-365 or
TC text, and no FR-104, FR-105, FR-363 or FR-364 text, still places these
refusals at the third argument.

FND-003: when `ReachesEnd` hits the two-population ambiguity, the refusal is
wrapped by `state_model_refusal`. The C15 test goes through `assert_refusal`,
which checks the StateModel family and the `state-model` catalog prefix, then
checks the inner `AmbiguousName` text. The field form's refusal is unchanged.

The spec fixes folded in (`via` to `trail`, C03 as `pre` clauses with the
fixture's declared `hop`/`hopRush`, and TC-1048 row 21 naming the key order
and expecting `dangling_reference` retaining `d2`) contradict no merged text.
The row 21 test now pins `first == "d2"`, so the recorded medium on row 21 is
resolved.
