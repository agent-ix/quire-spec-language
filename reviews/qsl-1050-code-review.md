---
id: SR-17013
title: "Code review of quire-spec-language PR #1450: EN-5 exact-build stops name exact_prob.* settings"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@41c394c47c44b430fc13a96ab7de364e25582770; PR #1450 (QSL-1050) diff origin/main...HEAD: qsl-replay/src/model_check/limits.rs, qsl-analyze/src/exact_probabilistic/build.rs, qsl-analyze/tests/it/exact_probabilistic.rs; read for context: qsl-analyze/src/exact_probabilistic.rs, qsl-replay/src/outcome.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/src/statistical/settle.rs, qsl-replay/src/certificate/check.rs, qsl-replay/src/refinement/step.rs, spec FR-255, FR-203, FR-127, TC-1161, QSpec FR-331"
review_set: subset
---
# Code review of quire-spec-language PR #1450

## Summary

Ticket: QSL-1050 (engine half). The change is code-review with the
rust-review lane. It adds `ModelCheckLimit::ExactProbMaxStates`,
`ExactProbMaxTransitions` and `ExactProbMaxAutomatonStates` to
`qsl-replay`. Their `setting()` returns the FR-255 `exact_prob.*` rows, and
their `as_str()` returns the EN-1 spelling. EN-5's three product-build stop
sites in `qsl-analyze/src/exact_probabilistic/build.rs` now use the new
variants. The setting names are correct and are tested. The limit spelling
the variants inherit does not match FR-255's limit names (FND-001). That
spelling predates this PR, but the PR extends it to EN-5 and its new unit
test pins it.

## Verdict

**CONDITIONAL**: no high findings. One medium finding and two low findings.

What holds:

- **Every EN-5 stop is covered.** The `exact_probabilistic` module (`build.rs`, `plan.rs`, `exact_probabilistic.rs`) builds
  `ProductStop::Limit` at exactly three sites: the automaton `StateLimit`,
  `max_states` and `max_transitions`. All three now use the new variants. No
  solver, iteration, precision, policy or witness-path stop exists in the
  tree yet, so FR-255's other `exact_prob.*` rows (`max_iterations`,
  `precision_bits`, `max_precision_bits`, `max_rational_bits`,
  `max_policy_iterations`, `max_witness_paths`) have no emitter to rename.
  The PR body says the same.
- **No match needs updating.** Every `match` over `ModelCheckLimit` is in
  `limits.rs` (`as_str`, `setting`), and both handle the new variants. Other
  consumers construct particular variants (`settle.rs`, `statistical/settle.rs`,
  `certificate/check.rs`, `refinement/step.rs`) or format a variant through
  `as_str`/`setting` (`outcome.rs:599-601`). No `_` arm absorbs the new
  variants. `ProductStop` and `ProductRefusal` have no consumer outside
  `qsl-analyze`, and the root crate, CG and the driver do not name them. No
  EN-5 settlement map exists, so the new variants reach no wire path today.
  When one lands, `outcome.rs:599-601` writes `limit = as_str()` and
  `setting = setting()` (see FND-001).
- **The stale note is gone.** The old `setting()` note ("An EN-5 exact-build
  stop still names the `model_check.*` row ... QSL-1050") is replaced by an
  accurate list. No other reference to QSL-1050 or QSL-1043 is left outside
  `spec/` and `reviews/`.
- **The tests can fail.** The integration test asserts the variant, the
  value and `setting()` for each of the three stops. Reviewer re-runs of the
  coder's two mutants, plus one more, are recorded under Gates.
- **Hygiene is clean.** No AC ids appear in the added doc comments or tests.
  rustfmt passes, and there is no new `allow`, `unwrap` on a production path,
  or `as` cast.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new EN-5 variants' `as_str()` returns `max-states`, `max-transitions` and `max-automaton-states`. FR-255's engine table gives these rows the limit names `max_states`, `max_transitions` and `max_automaton_states`. FR-255 requires each limit name to match `^[a-z][a-z0-9_]*$` (AC-12), and FR-203's use case says the result "names `max_states`". QSpec FR-331's `limit-reached{limit, ...}` uses the member name (`max_clocks`; FR-331-AC-29 `max_solver_steps`). The doc "The limit's FR-331 spelling" is therefore inaccurate. The kebab spelling predates this PR for the EN-1 members (and `solver-steps` and the others). The PR extends it to three new EN-5 variants, and the new unit test asserts `exact.as_str() == model.as_str()`, which pins it. Scenario: once an EN-5 settlement map exists, `outcome.rs:599` writes `limit: "max-states"` with `setting: "exact_prob.max_states"`, and FR-255-AC-12's pair check (limit name, setting name) = (`max_states`, `exact_prob.max_states`) fails. Fix in this PR (snake-case member names for the EN-5 variants, with the test asserting them), or defer to a follow-up ticket that fixes every `ModelCheckLimit` spelling together. | qsl-replay/src/model_check/limits.rs:128-134, qsl-replay/src/model_check/limits.rs:235, qsl-replay/src/outcome.rs:599 |
| FND-002 | low | Two doc comments are stale after the change. `ProductStop::Limit` still reads "A FR-126 budget: `max_states`, `max_transitions` or `max_automaton_states`", but its variants are now FR-203's EN-5 product bounds. The `ModelCheckLimit` enum doc still reads "The limit a stopped EN-1 run reached ... a member of [`ModelCheckLimits`], or of the subject's FR-120 limits", which does not cover the new EN-5 members. The new variant docs name `ExactProbLimits.model.limits.max_states`, a type that does not exist in the tree: `ProductRequest` still carries `ModelCheckLimits` "until FR-203's `ExactProbLimits` exists". That is acceptable as a spec reference, but "(FR-203)" should mark it as the spec name. No wrong behaviour. | qsl-analyze/src/exact_probabilistic.rs:413, qsl-replay/src/model_check/limits.rs:88, qsl-replay/src/model_check/limits.rs:98-105 |
| FND-003 | low | The new unit test `exact_prob_product_bounds_name_exact_prob_settings_and_share_spellings` carries no `#[trace]`, while the other tests in the module are tagged. TC-1161 step 2 describes exactly this check ("map every variant ... through its limit-name and setting-name functions"). A bare `#[trace("TC-1161")]` binds no AC, so it would not overclaim the criteria the PR body says are only partly asserted. The integration test keeps its `TC-631` tag. | qsl-replay/src/model_check/limits.rs:213 |

## Gates

Run in `/home/peter/dev/worktrees/review-1450` at 41c394c47, with its own
target dir:

- `cargo test --locked -p qsl-replay --lib model_check::limits`: 2 passed, 0 failed.
- `cargo test --locked -p qsl-analyze --test it exact_probabilistic`: 22 passed, 0 failed.
- rustfmt `--check` on the changed source files: clean.
- Mutants, each with its own target dir, all killed:
  - m1 (coder's): `build.rs` state stop reverted to `MaxStates`. Killed by
    `a_budget_or_a_cancellation_stops_the_build` (:1131) and
    `a_budget_equal_to_the_product_size_completes`.
  - m2 (coder's): `ExactProbMaxTransitions.setting()` set to
    `model_check.max_transitions`. Killed by the new unit test (limits.rs:233)
    and by the integration test's new setting assertion (:1181).
  - m3 (reviewer's): `build.rs` automaton stop reverted to
    `MaxAutomatonStates`. Killed by the integration test (:1157).
- `cargo clippy --locked -p qsl-replay -p qsl-analyze --all-targets -- -D warnings`: clean.
- `cargo run -p xtask -- string-edge`: "no unmarked, unlisted string comparison or string match found".
- `cargo deny check`: advisories, bans and sources ok; licenses FAILED with 251
  `rejected` errors from the local cargo-deny. The PR changes neither
  `deny.toml` nor `Cargo.lock`, so this is local environment noise, not
  this PR.
- CI: guard, cla/gate, language-boundary and publish-status pass.
  `cla / cla-check` fails. The PR is BEHIND main, and main changed only
  FR-203's use-case line, which does not conflict.
