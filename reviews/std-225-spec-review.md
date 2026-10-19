---
id: SR-13350
title: "base review of STD-225 parseable random-parameter, Boolean and threshold spellings (PR #1064)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2f1262b61292223fe35c3510835dd1127c1a79db; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md, spec/decisions/ADR-028-exact-probabilistic-engine.md, spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md, spec/functional/FR-196-build-the-probabilistic-product.md, spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md, spec/test-cases/TC-633-unbounded-reachability-and-expected-rewards.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-633
    type: reviews
---
# SR-13350: base review of STD-225 (PR #1064)

## Summary

Ticket: STD-225. Base checklist over
`git diff refs/remotes/origin/main...2f1262b61` (6 files, +20/-16). The diff
rewrites example spellings in ADR-024 §7.1 and §7.3, ADR-028 §15.4 to §15.6,
FR-187-AC-2, FR-196-AC-5, FR-198-AC-5 and TC-633 step 5. Spec-only diff, so
code-review and gap-analysis do not apply. No requirement statement changed,
so no EARS sub-analysis ran.

Checks made by hand, with my own probes (scratch, uncommitted):

- S1. A `qsl_cst::parse` probe parses every rewritten source fragment. These
  are the `random`, `reward`, `workload` and `probabilistic` declarations of
  ADR-024 §7.1 to §7.3 and ADR-028 §15.4 to §15.6, the FR-187-AC-2 random
  declarations, the FR-196-AC-5 random declaration and claim, and the
  FR-198-AC-5/TC-633 claim. All are admissible, with no diagnostic and no
  recovery. The text on main refuses `invalid_syntax`: the inline
  `operation Msg::send_a(random lost: Bool ~ …)`, `probability >= 1/2` and
  `probability <= 1/4`.
- S3. The same declarations, run over TC-458's `ConfigVersion` fixture
  (`admit_and_assemble_with_body`, then `check`), refuse only
  `unsupported_construct`/`not-yet-implemented`, one per declaration. So
  every S3 form check passes. Two controls (weight 0 and a repeated value)
  refuse `invalid-weight` and `repeated-support-value`, so the checks are
  live.
- Behaviour. Every support, weight and threshold is semantically identical.
  send_a `{true: 1, false: 9}`, send_b `{true: 1, false: 4}`, send
  `{true: 1, false: 9}`, flip `{true: 1, false: 1}` and Coin
  `{0: 1, 1: 1}` are unchanged. `1/2` becomes `0.5` and `1/4` becomes
  `0.25`; QSpec reads a decimal literal in an exact-value position as the
  exact rational. No alpha, beta or indifference value changed. TC-641's
  fixture refers to FR-196-AC-5 and still agrees with it.
- Wording. Every changed line describes the model, with no changelog
  phrasing.
- Gates. `make check-spec-validation` exits 0, with no warning on a touched
  file (17 warnings elsewhere, all from before this diff).
  `tools/check-index-completeness.sh` passes. Tools: quire 0.36.2
  (engine 0.50.2), quoin 0.28.3.
- Sweep of `spec/` for the same defect class (inline `random p: T ~`,
  `p ~ {…}`, rational thresholds in probability and confidence clauses, and
  `Bool` in QSL blocks). Findings below.
- The PR's claim about the remaining S1 refusals. Operation `pre`/`post`
  lines are model content: the shared grammar has no `operation`
  production, and the source form of a condition is
  `pre <label> using <alias> on T::op { … }`. The `terminal when …;` and
  `time dense unit ms;` / `time invariant when …` lines are a different
  case. They correspond to QSpec source declarations (`terminal-decl`,
  `time-decl`, `time-invariant`), and S1 admits the grammar's spelling
  (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-193-AC-3 still has a rational threshold in a probability clause, `probability <= 1/100 [eventually[0,1000] holds(not n.healthy)]`. As a claim with its `alpha 0.01, beta 0.01, indifference 0.005`, S1 refuses it `invalid_syntax` at `/`, and the same claim with `0.01` is admissible. This is the defect class the PR fixes in FR-196-AC-5 and FR-198-AC-5. It falls under the ticket's aim that no spec example fails to parse. Spell it `probability <= 0.01`. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:126 |
| FND-002 | medium | FR-199-AC-2 still writes its random parameters inline, as `repair_fast(ok ~ {true: 9, false: 1})` and `repair_slow(ok ~ {true: 1, false: 1})`. FR-185 does not define that form, and S1 refuses it. The ticket names this defect class. FR-200-AC-3 and TC-635 reuse the `Avail2` fixture. Respell it as `repair_fast(ok: Boolean)` with `random Server::repair_fast(ok) ~ { true: 9, false: 1 };`, and likewise for `repair_slow`. | spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md:74 |
| FND-003 | low | `quantile 1/2 of M >= 3` is used as a concrete claim in FR-197-AC-4, FR-189-AC-4, FR-201-AC-5, FR-202-AC-6 and TC-624/TC-632/TC-636, which also use `quantile 9/10` and `quantile 1/2 of M <= 1`. S1 refuses it twice over. `q` is an `exact-value`, so `1/2` is refused, and `quantile-form` requires `( measure )`. The decimal, parenthesized form is admissible. `M` is an abstract measure here, so this reads as notation, but it is the same rational-in-a-claim spelling the PR removes elsewhere. ADR-024 PF-4's `quantile q of M` definition is mathematical notation and fine as is. | spec/functional/FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md:95; spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:170; spec/functional/FR-201-check-a-probability-certificate.md:167; spec/functional/FR-202-replay-a-probabilistic-witness.md:183 |
| FND-004 | low | The PR leaves `terminal when …;` (ADR-028 §15.4 to §15.6) and `time dense unit ms;` / `time invariant when … { … }` (§15.5) unchanged, calling them model-document content outside the source grammar. That holds for `operation … pre/post`, but not for these lines. QSpec's shared grammar has source productions `terminal-decl` (`terminal <name> using <alias> on <alias> when { … };`), `time-decl` (`time <name> dense unit <unit>;`) and `time-invariant` (`time invariant <name> when { … } { … }`). S1 admits all three in those spellings, and refuses the ADR's spellings. ADR-026 CK-1/CK-6 still describe `time` as a state-model member, and both ADRs say the spellings are illustrative, so this is low. But the ticket's aim that no example fails to parse is not met for these lines, and ADR-026 §11 has the same spellings. Respell them, or record a follow-up ticket. | spec/decisions/ADR-028-exact-probabilistic-engine.md:494; spec/decisions/ADR-028-exact-probabilistic-engine.md:526; spec/decisions/ADR-028-exact-probabilistic-engine.md:532-533; spec/decisions/ADR-028-exact-probabilistic-engine.md:581; spec/decisions/ADR-026-dense-time.md:252-269 |
| FND-005 | low | `Bool` remains where the PR's own rule (`Bool` in model comments and parameters is `Boolean`) applies. These are the ADR-026 §11 model comment (`inflight: Bool, late: Bool`), FR-126-AC-4 (`healthy: Bool`) and ADR-022's `Job` fixture ("all `Bool`"). QSpec spells the type `Boolean`. Rust `Value::Bool` and the SMT `Bool` sort (FR-315) are correct and out of scope. | spec/decisions/ADR-026-dense-time.md:251; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:426; spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:373 |

## Verdict

Not mergeable as is: two medium findings and three low ones, all
omissions. The rewrite itself is correct. Every fragment the PR rewrote
parses at S1 and passes S3's form checks, the old text refuses, and no
support, weight or threshold changed meaning. The gates pass.

FND-001 and FND-002 are the ticket's own defect class in two acceptance
criteria the PR did not touch. Each is a one-line respelling. FND-003 to
FND-005 can be fixed in this PR or deferred with a follow-up ticket. One of
the PR's claims is wrong: the time and terminal lines are not outside the
source grammar (FND-004).

A note on the PR description, not the spec: it says the old FR-196-AC-5
claim refused at `alpha 1/100`, but that criterion on main has no confidence
clause; only `probability >= 1/2` refused.

Examined and clean: ADR-024 §7.1 and §7.3, ADR-028 §15.4 to §15.6,
FR-187-AC-2, FR-196-AC-5, FR-198-AC-5, TC-633 step 5. Also clean: the PR's
reading that FR-189-AC-2's `α = β = 1/100` is mathematical notation.
