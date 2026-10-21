---
id: SR-13606
title: "Gap analysis of QSL-573 E18a PR #1122 against FR-195 (AC-1..AC-4) and FR-196 (AC-1..AC-5)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@14268eab1; PR #1122 (git diff origin/main...14268eab1): qsl-analyze/src/exact_probabilistic.rs, qsl-analyze/src/exact_probabilistic/{build,plan}.rs, qsl-analyze/src/statistical.rs, qsl-analyze/src/statistical/claim.rs, qsl-eval/src/automaton/translate.rs, qsl-eval/src/simulation/model.rs, qsl-eval/src/simulation/model/{probabilistic,state_predicate}.rs, qsl-eval/src/value/expression/mod.rs, qsl-replay/src/{spine,statistical}.rs, qsl-semantics/src/check/{mod,probabilistic,state_graph_claim}.rs, qsl-semantics/src/check/probabilistic/claim.rs, qsl-analyze/tests/it/{exact_probabilistic,prob_world}.rs, qsl-semantics/tests/it/probabilistic_declarations.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
---
# SR-13606: Gap analysis of QSL-573 E18a PR #1122 against FR-195 (AC-1..AC-4) and FR-196 (AC-1..AC-5)

## Summary

Ticket: QSL-573 (E18a). Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

| AC | Status | Bound by | Honest? |
| --- | --- | --- | --- |
| FR-195-AC-1 | tagged | probabilistic_declarations.rs:1875, :1955 | yes |
| FR-195-AC-2 | tagged | probabilistic_declarations.rs:2014 | `ExpectedReward` and the untimed refusal; `MeanTime` deferred (QSL-577); `6/5` written `rational(6, 5)` (FND-002) |
| FR-195-AC-3 | tagged | probabilistic_declarations.rs:2093 | yes |
| FR-195-AC-4 | tagged | exact_probabilistic.rs:1195 | the three refusals only; manifest and `unsupported` settlement not built (FND-004) |
| FR-196-AC-1 | tagged | exact_probabilistic.rs:116 | yes |
| FR-196-AC-2 | tagged | exact_probabilistic.rs:196, :306 | yes (integer `duration`; `6 ms` is the above-bound tag) |
| FR-196-AC-3 | tagged | exact_probabilistic.rs:377 | yes |
| FR-196-AC-4 | tagged | exact_probabilistic.rs:509 | yes |
| FR-196-AC-5 | tagged | exact_probabilistic.rs:599, :639 | a substitute model and cause; the AC cannot be written (FND-001) |
| FR-196-AC-6..9 | untagged | - | deferred by the scope ruling (QSL-577) |

Every in-scope AC's primary test asserts what it binds and can fail: each is killed by at least one mutant in code review SR-13605 (AC-1: M19, M02; AC-2: M03, M04, M08; AC-3: M11, M20; AC-4: M14, M15, M16; AC-5: M17; FR-195-AC-1..4: M24 to M29, M32). The builder rules those ACs do not reach are code SR-13605 FND-008.

Declared not delivered, and confirmed not built: `MeanTime`/`Elapsed`, the FR-195-AC-4 manifest evidence kind, FR-192's 'settles unsupported' map, FR-196-AC-6 to AC-9. The oracle and mutation results are in SR-13605.

## Verdict

**Mergeable for coverage once FND-004 is either built or split into a follow-up ticket; FND-001, FND-002, FND-003 and FND-005 are spec fixes to route to /specify, not code changes.** The rest are low.

- **FND-001 (medium).** FR-196-AC-5 cannot be written in today's S3; the substitute test is faithful to the mechanism only.
- **FND-002 (medium).** FR-195-AC-2's `6/5` does not check and the threshold type is stated two ways.
- **FND-003 (medium).** FR-186's bullet and AC-3 contradict FR-195-AC-1.
- **FND-004 (medium).** FR-195-AC-4's manifest and `unsupported` settlement are not built.
- **FND-005 (medium).** FR-196's canonical key has no `over` binding.
- **FND-006 to FND-008 (low).** An unregistered cause, measure-semantics gaps, and unrecorded API drift.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-196-AC-5 cannot be built as written. Probed in my clone: the claim is refused `MissingName("c")` (no `over` binds `c`); with `over (c: M::Coin)` it is refused `ProfileProhibitedOperator(Divide)`, and `div` is refused `ProfileProhibitedOperator(IntegerDiv)`. The two tests tagged AC-5 (exact_probabilistic.rs:599, :639) check a substitute: a `drop(b)` that deletes the coin when `b = 0`, cause `absent-key`. That substitute checks the mechanism (first undefined state in breadth-first order, path, probability 1/2, position 1) faithfully, because the builder carries any `Undefined` cause; it does not check the cause, the operation or `UndefinedEvaluation{where, cause}`. Spec fix (/specify): rewrite AC-5 with `over (c: M::Coin)` and an undefined read S3 admits, for example the deleting `drop(b)` and `holds(c.v = 0)` with cause `absent-key`, or a profile that admits `/`; then update FR-202-AC-5 and FR-203-AC-5, which cite `division-by-zero` at `where: 1`. | spec/functional/FR-196-build-the-probabilistic-product.md:AC-5 |
| FND-002 | medium | FR-195-AC-2's `<= 6/5` does not check: probed, `/` in the threshold is refused `ProfileProhibitedOperator(Divide)`, so the test writes `rational(6, 5)`. And the behaviour bullet says '`c` of `R`'s type', while AC-2 needs a rational threshold for an integer reward; the code follows AC-2 (`Measured::Expectation`). FR-195 gives no rule for the sign or range of `c`. The `MeanTime`/`Elapsed` half of AC-2 is deferred (QSL-577) and the test asserts it is refused not-yet-implemented. Spec fix: write AC-2's threshold as `rational(6, 5)`; state that `c` is a rational for an integer reward and a quantity of `R`'s unit for a quantity reward, and whether a negative `c` is admitted. | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:AC-2 |
| FND-003 | medium | FR-186 is stale against FR-195. Its 'Events and measures' bullet refuses an event whose profile 'is not a bounded profile' as `unbounded-event`, and AC-3 says 'an event under an unbounded profile is refused `unbounded-event`'. FR-195-AC-1 now admits `[eventually holds(p.done)]` under the infinite-trace profile as `Reach`, and the code does. Spec fix: FR-186's bullet and AC-3 should except the three `Reach` shapes FR-195 admits (one interval-free `eventually`, `until` or `always` over `holds` operands under the infinite-trace profile), so the two ACs cannot both be read literally against one claim. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:AC-3 |
| FND-004 | medium | FR-195-AC-4 is in the ruled scope but only partly built. The test tagged AC-4 (exact_probabilistic.rs:1195) checks `require_statistical`'s three refusals. EN-5's manifest listing exactly (`probabilistic-satisfaction`, `exact`) is not built (`BackendDescriptor` has no evidence kinds), and 'settles `unsupported` with its cause' (FR-192's map) is not built or tested. The author declares both. Either build the two halves here, or open a follow-up ticket for them and narrow the AC-4 tag or the planner's scope ruling to the half delivered, so the matrix does not show AC-4 as fully bound. | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:AC-4 |
| FND-005 | medium | FR-196's canonical product key `[model, monitor, accumulator, pending]` has no `over` binding, but `initial` is per binding and the monitor's letters are read for one binding. Two bindings reaching one model state with one monitor state have different futures, so the spec's key would merge them. The code adds `binding` to `ProbKey` (the author's judgement call), which is correct; the spec is not. Spec fix: add the binding to the canonical key, or say the product is built per binding and the key is local to it. | spec/functional/FR-196-build-the-probabilistic-product.md:Canonical probability product key |
| FND-006 | low | The new refusal `invalid_probabilistic_model`/`not-state-predicate` (code comment: 'QSL-proposed, STD-188') is in no spec. FR-195 says only that a temporal operator is refused, with no cause; the code also refuses a constant (code SR-13605 FND-006). Spec fix: name the cause in FR-195 and register it through STD-188, stating whether a constant operand is refused. | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:S3 |
| FND-007 | low | FR-196 defines censored ('greater than every finite threshold') but not what an unactivated measure means for `<= c`, `>= c` or a quantile, and it does not say whether a terminal stutter step counts as a step of a `steps` measure. The code keeps the unactivated phase and counts the stutter, which is consistent with 'continues on the specified zero-reward stutter steps through its horizon'. Spec fix (or a pointer to QSpec FR-407): state the verdict for an unactivated measure and that a stutter step counts for `steps`, before EN-5's solver reads these phases. | spec/functional/FR-196-build-the-probabilistic-product.md:measure semantics |
| FND-008 | low | The delivered API differs from FR-195/FR-196's Outputs without a spec note: `reward: Identifier` for `RewardRef` (no `Elapsed`); `ProbKey`/`ProbStateId` in qsl-analyze for `ProductKey`/`ProductStateId` in qsl-replay; `build_product(ProductRequest)` over `ModelCheckLimits` for `check_exact_probabilistic(ExactProbRequest)` over `ExactProbLimits`; `UndefinedPath` for `UndefinedEvaluation{where, cause}`. Accepted under the untimed scope. Record each in the deferral ticket (QSL-577 or the certificate ticket) or as a spec note, so FR-201/FR-202's coder does not read the spec's types as built. | spec/functional/FR-196-build-the-probabilistic-product.md:Inputs/Outputs |

## Dispositions

Round 1 at ad8df47db (fix commit ad8df47db on top of a merge of origin/main). Checked against the fix commit in a fresh clone, not the PR text.

- **Gates:** all pass: fmt, clippy -D warnings, the four crates' tests, root `it` (975 passed), string-edge and the three arch-lints. 4,928 tests passed, 0 failed, 7 ignored.
- **Clean tree:** the committed tree carries no mutant. I checked every mutant of round 0 against the source: each original line is present and no mutated line is (M07's `Unactivated` arm is intact).
- **Oracle, re-run:** 40 random models, 756 products, 0 mismatches. 96 undefined paths agree, 0 mismatches. Two new checks pass. First, each of the 26 MDP paths through a draw with several post-states names the post-state it took (`resolved`), and replays to exactly one model state. Second, every resolution edge carries every reward at 0 while its draw edge carries the draw's rewards.
- **Mutants, re-run against the new tests:** 19 run, 19 killed. These are the 8 round-0 survivors (M07, M09, M18, M21, M30, M31, M35, M36), M02 and M15, and 9 new ones: N01 the shared closure inverted, N02 `resolved` dropped, N03 `resolved` naming the source, N04 and N05 resolution rewards empty or the draw's, G01 gate cause order, G02 the gate's `Ok` shortcut, E01 and E02 `exact_only` missing the form or `ExpectedReward`.
- **reviews/:** the committed `reviews/qsl-573-*.md` equal the round-0 files byte for byte. They need re-copying with this round's `## Dispositions`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Spec fix routed by the coordinator to /specify (PR #1122 'Spec follow-ups and deviations'); no ticket id is named yet. It is not a code change in this PR. AC-5's predicate and cause, and FR-202-AC-5 and FR-203-AC-5. |
| FND-002 | deferred | Spec fix routed by the coordinator to /specify (PR #1122 'Spec follow-ups and deviations'); no ticket id is named yet. It is not a code change in this PR. AC-2's threshold rules. |
| FND-003 | deferred | Spec fix routed by the coordinator to /specify (PR #1122 'Spec follow-ups and deviations'); no ticket id is named yet. It is not a code change in this PR. FR-186's stale bullet and AC-3. |
| FND-004 | still-open | The `FR-195-AC-4` tag was removed from `#[trace]` (now TC-630 only), but `quire matrix` (0.36.2, exit 0) at ad8df47db still reports FR-195-AC-4 tagged, bound by exact_probabilistic.rs:1197. The doc comment still opens with 'FR-195-AC-4, refusal half only', which appears to be what binds it. So the matrix still shows AC-4 as covered while the manifest and the FR-192 settlement are unbuilt. To fix: reword the doc comment so it no longer names the criterion id, and re-run the matrix until AC-4 reads untagged; or open a follow-up ticket for the unbuilt half and name it here. |
| FND-005 | deferred | Spec fix routed by the coordinator to /specify (PR #1122 'Spec follow-ups and deviations'); no ticket id is named yet. It is not a code change in this PR. The `over` binding in the canonical key. The code keeps the binding and now tests it (each_over_binding_has_its_own_states). |
| FND-006 | deferred | Spec fix routed by the coordinator to /specify (PR #1122 'Spec follow-ups and deviations'); no ticket id is named yet. It is not a code change in this PR. The `not-state-predicate` cause and the constant rule (STD-188). |
| FND-007 | deferred | Spec fix routed by the coordinator to /specify (PR #1122 'Spec follow-ups and deviations'); no ticket id is named yet. It is not a code change in this PR. The unactivated-measure verdict and `steps` over the stutter. |
| FND-008 | deferred | Spec fix routed by the coordinator to /specify (PR #1122 'Spec follow-ups and deviations'); no ticket id is named yet. It is not a code change in this PR. The RewardRef/API drift. |
