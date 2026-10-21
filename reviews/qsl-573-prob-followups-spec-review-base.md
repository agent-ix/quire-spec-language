---
id: SR-13607
title: "base review of the probabilistic spec follow-ups (PR #1124)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8a1f1ba219d7c8344c06301b2758cfad1a03fa23; spec/decisions/ADR-028-exact-probabilistic-engine.md, spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md, spec/functional/FR-196-build-the-probabilistic-product.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md, spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md, spec/test-cases/TC-627-statistical-results-settle-as-measured.md, spec/test-cases/TC-630-exact-only-forms-and-exact-routing.md, spec/test-cases/TC-631-probabilistic-product.md, spec/test-cases/TC-641-an-undefined-state-of-positive-probability-refutes-exactly.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-621
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-627
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-630
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-631
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-640
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-641
    type: reviews
---
# SR-13607: base review of PR #1124 (QSL-573, QSL-568, QSL-571)

## Summary

Tickets: QSL-573, QSL-568 and QSL-571. This review applies the base
checklist to `git diff origin/main...8a1f1ba21` (12 files, +231/-66):
ADR-028, FR-186, FR-192, FR-195, FR-196, FR-202, FR-203 and TC-621, 627,
630, 631 and 641. The PR changes only spec files, so code-review and
gap-analysis do not apply. Sub-analyses: ears-conformance (SR-13608),
integrity (SR-13609) and failure-domain (SR-13610).

Gates. `make check-spec-validation` exits 0, with no warning on a touched
file. `tools/check-index-completeness.sh` passes. ID formats and AC
numbering are in sequence (FR-195-AC-5, FR-196-AC-10 to AC-12), and the TC
scopes include the new criteria (TC-630 to AC-5, TC-631 to AC-12).

Each statement was checked against origin/main (#1116, #1118) and
origin/qsl-573-en5-exact (#1122, 9a7b85081). Confirmed against the code:
`over-not-object-type` (claim.rs Pass::over, the main test at
probabilistic_declarations.rs:2203); the Reach shape and `not-state-predicate`
spans; the expected-reward threshold table (Pass::threshold
`Measured::Expectation`); the gate order (require_statistical); the
FR-192-AC-1 setting names (`meter`, `max_candidates` in
ModelCheckLimit::setting); `ProbKey`, `ProbBuild`, `ProductRefusal`,
`ProductStop`, `UndefinedPath` and `PathStep` as recorded; the `binding`
key component; the absorbing `Unactivated` phase; stutter steps counted by
`steps`; zero-reward resolution edges; `resolved` on MDP paths; and
FR-196-AC-5's `drop` model (absent-key, position 1, 1/2).

Owner rulings applied and not raised here. FR-196's "SHALL reuse
model_check's product TransitionSystem ..." stays as written, and the
builder's own BFS is remaining work on the ticket. The verdict of an
unactivated comparison event (ADR-024 PF-3) is with the planner.

Criteria the code lane still has to trace (acceptable for a spec PR):
FR-192-AC-1's MaxCandidates `setting` (settle.rs:385 asserts the value
only); FR-192-AC-4 and FR-195-AC-4 (#1122's gate test is tagged `TC-630`
with no AC id); FR-195-AC-5 (no test; main's settle_statistical_refusal has
no arm for EveryScheduler/ExactOnlyForm/MissingConfidence, and #1122 adds
the variants without #1118's settle.rs, so its rebase needs those arms);
FR-195-AC-2's `6/5`, `<= 0`, `rational(-1, 2)` and bare-`3` clauses (no
test); FR-196-AC-10, AC-11 and AC-12 (#1122 has matching tests,
each_over_binding_has_its_own_states,
an_unactivated_measure_ends_decided_and_a_fraction_reads_the_source,
a_measure_continues_on_the_stutter_steps_to_the_end_of_its_window,
a_resolution_advances_the_accumulator_with_the_draws_rewards and
an_mdp_undefined_path_names_the_post_state_taken_at_each_draw, but they are
tagged by TC only). FR-196-AC-5 is traced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-195-AC-2 ends with "§15.5's MeanTime checks with the elapsed quantity and threshold 3 ms", but the PR's own Inputs prose says no subject is timed at S3 and S3 refuses every `expected elapsed` as `unsupported_construct`/`not-yet-implemented`; the #1122 code does exactly that (claim.rs ExpectedQuantity::Elapsed arm) and its TC-630 test asserts `not_yet`. No test can pass that clause until FR-204 lands. The companion clause, `expected elapsed` over an untimed subject "is refused", names no code or cause. Fix: move the MeanTime clause to FR-204's criteria (or state the current refusal), and name `unsupported_construct`/`not-yet-implemented` for the untimed case in AC-2 and TC-630 step 2. | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:66-70; spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:109-110; spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:139 |
| FND-002 | medium | Outside the diff but now contradicted by it. FR-189-AC-6 (and TC-640, and FR-192-AC-5 through it) still uses the `Coin` claim `holds(1 / (1 - c.v) = 1)` with no `over`, which this PR's FR-196-AC-5 states S3 cannot admit ("S3 admits neither `/` in a model unit's profile nor a name that no `over` binds"). Its test (qsl-analyze/src/statistical/tests.rs:493-560) is a synthetic ClaimSampler that never runs S3, so it is green while the AC's fixture cannot be checked. Not a merge blocker (pre-existing, nothing red), but cheapest to fix here with FR-196-AC-5's `drop` model over `(c: M::Coin)` and cause `absent-key`; otherwise open a ticket. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:215 |
| FND-003 | low | FR-196-AC-5 closes with build-log wording about the replaced example: "A division by zero is not the example: S3 admits neither `/` in a model unit's profile nor a name that no `over` binds." That explains the edit, not the system. Fix: delete the sentence (the rule it cites belongs in FR-186, which already states it). | spec/functional/FR-196-build-the-probabilistic-product.md:725 |
| FND-004 | low | FR-196-AC-11 asserts the canonical control body `["measure","0","unactivated"]`, but the build entry this PR records says the builder does not encode `ProductKey` (its in-memory key is `ProbKey` with `MeasurePhase::Unactivated`). A TC-631 test of the builder can observe only the in-memory phase; the canonical body belongs to the qsl-replay encoder, which does not exist yet, and the worked-vector table (AC-6/AC-7) has no unactivated row. Fix: state the in-memory outcome (`Measure{remaining: 0, phase: Unactivated}`, `activated` false, amount 0) in AC-11 and add the canonical body as a worked-vector row under AC-7. | spec/functional/FR-196-build-the-probabilistic-product.md:731 |

## Verdict

Not mergeable as it stands. The blockers are SR-13610 FND-001 (high)
and the medium contradictions in SR-13609 (FND-001 to FND-003). They are
wording fixes in this PR, and none needs a code ruling except SR-13610
FND-002, which needs an owner choice. Here, FND-001 (MeanTime) should be
fixed in this PR. FND-002 (FR-189-AC-6's `1/(1-c.v)`) does not have to be:
it is outside the diff and nothing is red. Fixing it here is one AC and
one TC step, so do that or file a ticket. The rest of the diff is clean.
The FR-186/FR-195 threshold, gate and shape rules, the FR-192-AC-1 setting
names, and FR-196's binding key, unactivated phase, stutter counting and
resolution rewards each agree with the code. There is no changelog, status
or owner wording other than FND-003.
