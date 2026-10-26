---
id: SR-60038
title: "QSL-1102 slice 2 part (i) review of the named state-machine subject refusal (quire-spec-language PR 1613)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@69ff7db8748b354a2e1617be382f066f42c7c3df; git diff origin/main (7 files, +257/-19): qsl-semantics/src/check/{mod.rs,refinement.rs,lowering/model.rs}, qsl-replay/src/spine/refinement.rs, qsl-semantics/tests/it/refinement_declarations.rs, spec/functional/FR-136-check-a-refinement-s-step-rows.md, spec/test-cases/TC-541-s3-checks-a-refinement-s-step-rows.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---

## Summary

Ticket QSL-1102, slice 2 part (i). A refinement whose side package declares
no `state_machine` record refuses `unsupported_construct`/`declaration-form`
naming the side and alias; several records refuse
`ambiguous_declaration`/`ambiguous-name` with the count; exactly one per side
leaves the generic `RefinementNotEmitted`. Verdict PASS, with low findings.

## Verified

- Placement. The refusal is built in `PackageDeclarations::check`
  (check/mod.rs ~4753) inside `if !self.emit_unwritten_refinements`, where
  `RefinementNotEmitted` was raised. `emit_unwritten_refinements` is set only
  by the `#[cfg(any(test, feature = "test-support"))]` builder
  `with_unwritten_refinements`, so production always takes the refusing
  branch and every refinement is refused (named when the subject is wrong,
  generic when it is one machine per side). With the flag on, nothing new is
  produced: the qsl-analyze/qsl-replay object-type fixtures (counter, CAS,
  ring buffer) keep their `CheckedRefinement`. Evidence: qsl-replay lib 878
  and integration targets pass except the three inherited tp4 tests below;
  `refinement_declarations` 35/35 pass. No path emits or accepts an
  object-type refinement silently: the branch is the same gate as before, and
  the subject lookup runs only after every earlier check succeeded (a failed
  refinement check returns before this point, so the gate never runs after a
  failure).
- AC wording. FR-136-AC-12 says "emitted, refuses", and its section says S4
  selects the subject; that matches a refusal at the emission boundary that
  the S3-only flag suppresses. It differs from the planner ruling's "refused
  at S3" (FND-001).
- Mutation (each arm, one at a time, in a scratch run of
  `cargo test -p qsl-semantics --features test-support --test it --
  refinement_declarations`; all reverted, tree clean): single machine
  treated as none FAILS 2; zero treated as ambiguous FAILS 2; Abstract
  labeled Concrete FAILS 2; Concrete labeled Abstract FAILS 1; count + 1
  FAILS 1; side order reversed FAILS 1; ambiguous code changed FAILS 1;
  ambiguous cause changed FAILS 1; `state_machine_count` counting every
  record FAILS 3.
- Replay spine: the two arms in `check_limit` return `None`. A replay run
  checks with the flag on, so the causes cannot reach it; unreachable by
  design, but with no comment or test saying so (FND-003). No wildcard arm,
  no `#[allow]` in the diff. clippy `-D warnings` workspace default and
  `--all-features` clean; fmt clean; the string-edge lane reports only the
  inherited terminal.rs 235/236/255 and nothing in this diff.
- Spec. `quire validate` on FR-136 and TC-541 reports no error for them (the
  tool's output holds only repo-wide DuplicateArchetype and similar notices).
  The AC, TC step 12 and the Scope line agree with the tests. ADR-020 object
  type examples are still accepted at S3 (flag on) for analysis.
- Tags: both new tests carry `#[trace("TC-541", "FR-136-AC-12")]`; the
  retitled TC-540 tests keep their tag and now assert the named refusal.

## Gate

`make ci` at 69ff7db87 stops at `ci-default-features`: three tests
`vault_world::tp4::{tp4_closure_accepts_the_independently_replayed_keys,
tp4_closure_refuses_a_foreign_context_key, tp4_closure_refuses_each_key_tamper}`
in qsl-replay panic `the request admits: LivenessNeedsComponents`
(vault_world.rs:739). This is not the expected string-edge red. It is
inherited: the same three fail identically on origin/main 176bd20ce in a
clean worktree (`cargo test -p qsl-replay --test it tp4_closure`); the PR's
only qsl-replay change is a `check_limit` match arm. Most likely cause is
#1611 (TP-4 closure refusal) merging with tests that predate it. The graph
therefore never reached string-edge, so the remaining lanes were run by hand:
fmt, string-edge (only terminal.rs 235/236/255, inherited from terra #1595,
fix #1608), route-lint, checked-input, `test-suites default-qsl-semantics`,
`make ci-all-features` (same three tp4 failures, nothing else), ci-docs,
build-no-std, check-spec-validation, refinement-versioning, cargo-deny-bans, arch-lint and seam-probe exit 0 (ci-clean-build and conformance input lanes not run; the diff touches neither).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The planner ruling says refused at S3; the refusal is at the emission gate, which is what FR-136 says (S4), but the `NoStateMachineSubject` doc comment says S3 has no state-machine record to select. Reword the comment to the emission gate and record in QSL-1102 that S3 still yields a `CheckedRefinement` for object-type models by design. | qsl-semantics/src/check/refinement.rs:258-266; qsl-semantics/src/check/mod.rs:4753 | wrong-requirement |
| FND-002 | low | Coverage gaps: `AmbiguousStateMachineSubject` is tested for the Abstract side only (no Concrete side, no count above 2, no both-sides-bad case); no test asserts that the flag-on state (`with_unwritten_refinements`) yields no subject refusal. It is covered only indirectly by the existing refinement and analyze/replay fixtures. | qsl-semantics/tests/it/refinement_declarations.rs:1851-1905 | correct-requirement-no-evidence |
| FND-003 | low | The two new replay spine arms return `None` and are unreachable by design (replay checks with the flag on), but nothing says so. Add a one-line reason. | qsl-replay/src/spine/refinement.rs:300-301 | correct-requirement-no-evidence |
| FND-004 | low | check/mod.rs falls back to the generic `RefinementNotEmitted` when no `CheckedRefinement` matches a declaration. That cannot happen after a successful check, so the fallback would hide a defect; prefer an internal-fault refusal. | qsl-semantics/src/check/mod.rs:4753-4773 | implementation-bug-despite-evidence |
| FND-005 | low | `refinement_declarations` compiles only with `test-support`, so `cargo test -p qsl-semantics --test it refinement` alone runs zero tests; reviewers and the coder note must pass `--features test-support`. | qsl-semantics/tests/it/main.rs:70 | correct-requirement-no-evidence |

## Verdict

PASS. No blocking finding; every new arm fails under mutation, the refusal is
suppressed exactly as the old `RefinementNotEmitted` was, and the aggregate
gate's red is inherited (three tp4 tests on main, see Gate), not caused here.
