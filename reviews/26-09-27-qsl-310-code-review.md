---
id: SR-768
title: "QSL-310 code review (with rust-review lane) of PR 507"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6200370a44f5521e3b4618739a95535980cee603; qsl-replay/src/spine/clause/tests.rs (new: config_version_invocation_snapshot, config_version_invocation_bytes, config_version_invocation_selection, run_config_version_invocation, config_version_step2_unit_and_packages, config_version_step2_request, tc466_step1_version_unchanged_over_an_invocation, tc466_step2_pre_version_number_is_two, tc466_step2_post_version_number_is_three, tc466_step2_pre_parent_implies_version_one; read unchanged: config_version_unit_and_packages, config_version_request, config_version_snapshot, run_config_version_current, boolean_disposition, s4_emits_exactly_the_fr_105_state_nodes); spec/functional/FR-107-evaluate-state-clauses-at-s6a.md (unchanged); spec/functional/FR-106-admit-snapshots-and-invocations.md (frame check, unchanged)"
review_set: subset
---
## Summary

Ticket: QSL-310. PR: quire-spec-language#507 at 6200370a, base f17c2d4f.
Methods: code-review with the rust-review lane folded in. The diff is
test-only: +301/-0 in `qsl-replay/src/spine/clause/tests.rs`.

What I checked, and what I found:

1. **Step 1 `VersionUnchanged` discriminates.** The clause is
   `self.versionNumber = pre(self.versionNumber)` (tests.rs:1615-1616). The
   test runs it over a real `attemptUpdate` invocation with `child` at 2->2
   (asserts `true`) and 2->3 (asserts `false`). The two assertions are not
   identical: an evaluator that ignored `pre` would read 3 = 3 and fail the
   second one. `boolean_disposition` (tests.rs:1750-1757) panics on anything
   other than `Evaluate(Completed(Boolean(_)))`, so a refusal or an
   `Incomplete` cannot count as the expected `false`.
2. **Step 2: separate clauses, but only two of the three terms
   discriminate.** Each term is its own `post` clause, selected by name and
   run in its own test, so the three are not one bundled check.
   `PreVersionIsTwo` (`pre(self.versionNumber) = 2`) and
   `PostVersionIsThree` (`self.versionNumber = 3`) each fail if the pre/post
   read is swapped or ignored, because pre is 2 and post is 3.
   `PreParentVersionIsOne` does not discriminate: see FND-001.
3. **The separate step-2 unit is justified.** TC-462's
   `s4_emits_exactly_the_fr_105_state_nodes` asserts
   `clause_kinds == ["invariant", "invariant", "postcondition"]` over
   `config_version_compiled()`, which is built from the base unit
   (tests.rs:3004). Adding the three step-2 posts to the base unit would
   make that list six entries. The PR has zero deletions, so neither TC-462
   nor the base unit changed. TC-462 passes in my own gate run (below).
4. **`run_config_version_invocation` is new plumbing, not a renamed copy.**
   It builds three documents (pre snapshot, post snapshot, and an invocation
   document that names both by identity plus digest). It inserts into both
   `request.snapshots` and `request.invocations`, selects through
   `ClauseSelectionInput::Invocation`, and takes the request builder as a
   `fn` pointer, so one runner serves both the base and the step-2 unit.
   `run_config_version_current` builds one `current` snapshot with an anchor
   and a self object. It is a sibling for the other input kind, not a
   generalization that replaces the current-snapshot runner. The copying
   is in the fixture builders instead (FND-002, FND-003).
5. **Trace tags are correct.** See SR-769: FR-107-AC-1 is the row that
   names `VersionUnchanged`, and FR-107-AC-2 is the row that names the
   three step-2 terms.

Rust-review lane: test-only diff. The `expect`/`unwrap`/`panic!` calls are
confined to test code, there are no casts, and no production code changed.
`i64::to_string()` feeds the snapshot's canonical integer text directly.

Gates, run fresh by me (not taken from the PR's `ci-qsl-310.log`): at
6200370a, `make ci` exited 0 with the worktree's own `CARGO_TARGET_DIR`. The
log has 93 `test result: ok` lines and 0 FAILED, and all five new tests plus
TC-462's `s4_emits_exactly_the_fr_105_state_nodes` ran and passed. A first
run against the shared `~/.cargo-target` exited 2: two arch-lint live-tree
tests (`tc_452_spine_surface_check_passes_over_qsl_replay`,
`tc_arch_lint_api_surface_024_...`) panicked `NotFound` on `canonicalize()`.
That was environmental: stale binaries in the shared target dir, with
another checkout's `CARGO_MANIFEST_DIR` baked in. Both tests pass in the
own-target run, and the PR does not touch arch-lint. On a scratch merge of
origin/main (760ef144) with the PR, the TC-466 and TC-462 tests pass too
(6 passed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `tc466_step2_pre_parent_implies_version_one` cannot tell whether `pre` is honored for the deref, and it can also pass vacuously. (a) `run_config_version_invocation` writes the same `root_version` into both the pre and the post snapshot, so `deref(value(self.parent)).versionNumber` is 1 in both observations. An evaluator that reads the deref'd `root` in post, contrary to FR-107's "A reference keeps the observation it was read in, so a deref of it reads that observation", still passes. (b) The body is an `implies`, so if `present(self.parent)` wrongly gave `false` the clause would still be `Completed(true)`, and no control case expects `false`. The fix is to give `root` a different version in post than in pre (for example pre 1, post 5; FR-106 check 11.3 admits it because `versionNumber` is in the frame's `modifies`), and to add a control whose pre `root` is not 1 and which asserts `false`. I measured this on a scratch main+PR merge with a temporary env override of the pre/post `root` versions: pre 1 / post 5 admits and gives `Completed(true)`, and pre 5 / post 1 gives `Completed(false)`, which fails at the assertion (tests.rs:2736), not in `boolean_disposition`. So the evaluator is correct today and the stronger test is buildable; the PR's test just does not pin that behavior. | qsl-replay/src/spine/clause/tests.rs:2555-2617; qsl-replay/src/spine/clause/tests.rs:2727-2745 |
| FND-002 | low | `config_version_step2_request` copies the whole 18-line `ClauseRunRequest` literal from `config_version_request`; only the unit text and the `path` differ. A single `config_version_request_for(unit, packages, path)` used by both would keep the request shape in one place. | qsl-replay/src/spine/clause/tests.rs:2670-2689; qsl-replay/src/spine/clause/tests.rs:1633-1652 |
| FND-003 | low | `config_version_invocation_snapshot` re-spells the `root`/`child` object JSON and the snapshot `identity`/`model` blocks from `config_version_snapshot`. `config_version_invocation_bytes` spells the identity JSON a third time, through its own `identity_json` closure. Shared `config_version_objects(root, child)` and `document_identity_json(label)` helpers would give each shape one source. | qsl-replay/src/spine/clause/tests.rs:2459-2495; qsl-replay/src/spine/clause/tests.rs:1667-1703; qsl-replay/src/spine/clause/tests.rs:2509-2514 |
| FND-004 | low | `run_config_version_invocation` takes six positional parameters, three of them bare `i64`s, so two versions are easy to transpose. `root_version` is `1` and `self_key` is `"child"` at all five call sites, so that generality is unused. Its pre, post and invocation stages also repeat the same build / `document_digest` / relabel-with-digest block three times. A `with_digest(label, bytes)` helper, plus dropping or fixing the unused parameters (or taking a small struct for the pre and post versions, which FND-001's fix needs anyway), would shorten it and make the call sites self-describing. | qsl-replay/src/spine/clause/tests.rs:2555-2617; qsl-replay/src/spine/clause/tests.rs:2625-2745 |

## Verdict

Approve with findings. There are no high findings, and the gate is green.
Step 1 discriminates, two of the three step-2 terms discriminate, the
separate step-2 unit is justified by TC-462's exact clause-kind assertion,
and the trace tags match FR-107's AC table. FND-001 (medium) should be fixed
in this PR: `PreParentVersionIsOne` needs a pre/post-distinct `root` version
and a `false` control so that it actually tests the `pre`-scoped deref.
FND-002 to FND-004 are duplication and readability cleanups for the same
fix round.
