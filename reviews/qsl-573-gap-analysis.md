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
