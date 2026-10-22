---
id: SR-15550
title: "spec review of STD-311 HP-2 self-composition SMT queries (QSL PR #1228)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@9f8444aff47efe3f52f3f8075abafa00f8c97392; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md, spec/test-cases/TC-1120-the-hyper-self-composition-smt-queries-are-canonical.md, spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: reviews
---
# SR-15550: spec review of STD-311 HP-2 self-composition SMT queries (QSL PR #1228)

## Summary

Ticket: STD-311. The diff is `origin/main...9f8444aff` (main 3fc22a165, which is an ancestor of the head). It touches 3 files and is spec only. It adds FR-315's "Hyper self-composition queries" section, FR-315-AC-7 to AC-11, TC-1120 and one FR-176 Dependencies line. The reviewer checked it against ADR-023 (HS-5, HM-3, HM-7, HC-1, HC-4, HC-6, HC-7, HV-1, HV-4, HV-8 and §8), ADR-018 (EN-3, V-3, V-6 and UE-1), FR-176, FR-182 (the V-3 and V-6 rows and AC-7), FR-172 (`CheckedHyperClause`), FR-107 (the evaluator signature) and FR-106 (structural equality on `Option`). All are on QSL main.

These claims hold:

- AC-11 is sound for the secure vault. Each matched joint step sets `l_a = l_b = i`, and both initial states have `l = 0`, so the base query at depth 1 is unsatisfiable. In the step query, every matched successor has equal `l`, so the step query is unsatisfiable even with no hypothesis. For the vault whose post clause is `self.l = self.h`, the initial pair `((0,0),(1,0))` with joint step `(step(0), step(0))` gives `l_a = 0` and `l_b = 1` at position 1, so the base query is satisfiable.
- HC-4 soundness holds. If the base and step queries at depth `k` are both unsatisfiable, simple-path `k`-induction shows that no reachable matched joint state violates the body or fails definedness. Every fair matched tuple is a matched path, so ignoring fairness is sound. The cost is completeness: a bad prefix that cannot be extended to a fair tuple leaves the base satisfiable.
- The Uncertified V-3 settlement agrees with FR-182's V-3 row, FR-182-AC-7 and ADR-023 HV-1. An undefined `μ_U` counted as a definedness failure agrees with HV-8 and FR-176. Refusing `Unrolling` matches FR-182, which has no V-2 row for hyper items.
- The declaration order in AC-7 matches the stated order rule (position, then variable, then object, then field). `h` and `l` are `Int[0, 1]`, so `Int` with range assertions is the right sort.
- The ids FR-315-AC-7 to AC-11 and TC-1120 appear on no other open PR head and not on main. The diff has no status or changelog wording and adds no compatibility layer.

Lead defaults:

- Defaults 1, 2, 4, 5 and 7 are sound.
- Default 3 (refuse when `μ_U` is refused or incomplete on any tuple) is acceptable, because a refusal is not a verdict. It needs a meter source (FND-003).
- Default 6 (refusal names as strings) is acceptable, because base FR-315 also names constructs.

The open vacuity question cannot stay open. The PR's "Reading the results" bullet makes an unsatisfiable base and step settle V-3 `proved`. That would let a vacuous clause settle `proved`, which ADR-023 HM-7 forbids (FND-001).

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: The PR gives a normative path to `proved` for a vacuous clause. The "Reading the results" bullet maps an unsatisfiable base and step to `Inductive{depth: k}`, which FR-182 settles V-3 `proved`. The vacuity bullet then admits that the same pair is unsatisfiable when no fair tuple matches `μ_U`. ADR-023 HM-7 requires a vacuous non-empty `μ_U` to settle V-6 `VacuousMatch`, "never `proved`". FR-176 (Vacuity) and FR-182's use case say the same. A related gap: a subject with no initial state makes the base trivially unsatisfiable, where ADR-018 settles `NoInitialState`. The PR has to close this path; stating the gap is not enough. <br>Fix, owner decision: <br>(a) Recommended: V-3 settles only with a replayed non-vacuity witness, a fair `μ_U`-matched lasso tuple found by an EN-2-style lasso query with no body. Replay checks it as HX-2 does, without evaluating the body. Without the witness, the item stays inconclusive. This amends FR-182's V-3 row and FR-183. <br>(b) Restrict EN-3 to an empty `μ_U` and refuse otherwise. This excludes `NonInterference`. <br>(c) Run EN-1's vacuity check before V-3. That needs the product EN-3 exists to avoid. <br>(d) Amend HM-7. That weakens an ADR guarantee and is not recommended. <br>If the PR does not adopt one of these, at least make the reading say that `Inductive` from these queries is not sufficient for V-3 while `μ_U` is non-empty, and file the settlement ticket. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:234-253 |
| FND-002 | high | major: AC-7's joint-disjunct counts leave out the terminal stutter. Base FR-315 encodes "the terminal stutter as one more disjunct under infinite-trace", and every `behaviours` clause is infinite-trace (HS-7). The hyper section's own component list includes "the terminal stutter under infinite-trace". So each vault component has 3 disjuncts: `step(0)`, `step(1)` and `stutter`. `NonInterference`'s `μ_U` holds on `(stutter, stutter)`, because the ops are equal and both `Option` arguments are absent, which FR-106 structural equality treats as equal. That gives 3 joint disjuncts, not "exactly two". Without `match` there are 9 joint disjuncts, not 4. TC-1120 step 1 repeats the same counts. The position of `stutter` in each component's disjunct order is also not pinned, and the lexicographic order depends on it. <br>Fix: count and order the stutter tuples, with `stutter` last per component. Alternatively, specify a static pruning of the stutter disjunct when a subject has no terminal state, and say so in the base encoding too. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:204-212, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:280 |
| FND-003 | medium | minor: `μ_U` is evaluated at encode time through FR-107, which takes a `Meter`. `HyperQueryRequest` carries no meter or budget. The `match-conjunct` refusal for an "incomplete" conjunct, and AC-10's "function whose evaluation exhausts its meter" case, therefore have no defined budget. Two implementations can disagree on whether a script is written. <br>Fix: add the meter, or a named budget with a published default, to `HyperQueryRequest`, and include it in "equal requests give byte-equal scripts". | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:172-176, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:263-264 |
| FND-004 | medium | minor: `ParameterBinding` (`instance: &[ParameterBinding]`) is defined nowhere on main. The prose also says the request carries "one instance of FR-176's HP-2 `ModelCheckRequest`", but FR-126's `ModelCheckRequest` has no instance member, because instances come from FR-125's canonical instance order. <br>Fix: define the binding, for example as `CheckedHyperClause.parameters` paired with universe keys in parameter order. Alternatively, reference the type that FR-125 or FR-126 already uses for one instance. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:170-184 |
| FND-005 | low | minor: A satisfiable base is "no outcome of these queries", and FR-182's inputs list no mapping for it. Every item must settle exactly one terminal record, so the spec should name the outcome CG reports when the base is satisfiable and EN-2 finds no replayable lasso, for example V-5 `BoundReached` or V-6 `InductionNotClosed`. Separately, a satisfiable base that ends at an undefined `μ_U` or letter is HV-8 refuting evidence that needs no lasso, so "need not extend to a fair matched tuple" does not apply to the undefined case. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:243-249 |
| FND-006 | low | minor: Wording. "read as a temporal item's is above" is ungrammatical. "joint states at positions `0..depth` SHALL be pairwise distinct" means `0..=depth`, as AC-8 shows, while the same sentence uses `0..depth` exclusively for the property. AC-11 calls the `self.l = self.h` base model "not a counterexample", although that clause is refuted. The intended meaning is "not a `HyperCounterexample` (no lasso)". TC-1120 step 5 passes silently when no solver is available, so AC-11 is in effect Analysis only. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:216-228, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:284 |

## New findings (disposition pass 1)

Head a43e3cad1 (main with STD-297 merged in; `origin/main` is an ancestor).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | major: FR-182 now settles an SMT `Inductive` outcome with a non-empty `μ_U` and no `Accepted` `MatchWitnessCheck` as `VacuousMatch`. FR-182's own Outputs comment defines `VacuousMatch` as "non-empty μ_U that no fair universal tuple satisfies", and ADR-023 HV-4 defines it as "the HM-7 case", which is a positive vacuity claim. FR-182-AC-10 settles the secure `NonInterference`, which is not vacuous (FR-183-AC-8's matched witness exists), as `VacuousMatch`. That reports a false cause on the wire and tells the author their `match` is wrong. The verdict, `inconclusive`, is right; the cause is wrong. <br>Fix, choosing one: <br>(a) Redefine `VacuousMatch` in FR-182 (Outputs comment and use case) as "non-empty μ_U with no matched fair tuple established: EN-1 found none, or an SMT proof came with no accepted witness", and amend ADR-023 HV-4 to match. <br>(b) Use a distinct cause for "witness missing", which needs QSpec FR-399 and ADR-023 QS-5 (an owner or QSpec ticket). | spec/functional/FR-182-settle-a-hyper-verdict.md:71, spec/functional/FR-182-settle-a-hyper-verdict.md:123-131, spec/functional/FR-182-settle-a-hyper-verdict.md:148 |
| FND-008 | medium | minor: An `Inductive` outcome with a non-empty `μ_U`, no witness and a subject with no initial state meets both new bullets: "settle V-6 `VacuousMatch`" and "settle V-6 `NoInitialState`". FR-182-AC-10 expects `NoInitialState` "with or without a witness", but the Behavior text states no precedence. <br>Fix: say that the initial-state check comes first. | spec/functional/FR-182-settle-a-hyper-verdict.md:123-135 |
| FND-009 | medium | minor: `MatchWitness.instance` is not tied to the outcome it vouches for. FR-315 reads results "for one instance", but FR-182's V-3 condition accepts any `Accepted` check, so a witness for one instance could license `proved` for another. `check_match_witness` also does not say that `μ_U` is evaluated under the witness's instance. <br>Fix: require the witness's instance to equal the `Inductive` outcome's instance, or state per instance how vacuity is read, and say that the check evaluates `μ_U` under `witness.instance`. | spec/functional/FR-182-settle-a-hyper-verdict.md:61-63, spec/functional/FR-183-replay-a-hyper-counterexample.md:100-111 |
| FND-010 | low | minor: `pub fn check_match_witness(/* FR-098 request */, witness: &MatchWitness)` is not a signature: the request parameter has no name or type. FR-315's "charges it for each tuple's evaluation in the joint-step order" also leaves open whether `μ_U` is evaluated once per tuple or once per step `i`, and that changes when the meter runs out. | spec/functional/FR-183-replay-a-hyper-counterexample.md:111, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:187-190 |

## New findings (disposition pass 2)

Head 1644e112b (`origin/main` is an ancestor).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | minor: The QSL meaning of `VacuousMatch` now differs from the QSpec wire meaning. QSL widens `VacuousMatch` to "no matched fair universal tuple established" in FR-182, ADR-023 HV-4 and HM-7, and FR-290 still maps it to the wire cause `vacuous-match`. QSpec FR-399 defines that cause as "A vacuous clause" (row at line 82, AC-5), and QSpec TC-340 VS-01 reads it the same way. ADR-023 QS-5 gives QSpec ownership of the inconclusive causes. The PR body says "the QSpec wire is unchanged". The token is unchanged, but a QSpec consumer will read an EN-3 `vacuous-match` as a proven-vacuous clause. <br>Fix: file an STD ticket to amend QSpec FR-399's `vacuous-match` row (and AC-5's wording) to the widened meaning, and cite it in the PR body beside the FND-007 lead default. No change is needed in this PR. | spec/functional/FR-182-settle-a-hyper-verdict.md:72-75, spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:290, spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:465 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-182 V-3 now requires every subject to have an initial state and an empty `μ_U` or an `Accepted` `MatchWitnessCheck` (FR-183 `check_match_witness`), otherwise V-6. The empty-`μ_U` exemption is sound under ADR-018 §4 and HM-7. The new cause label is wrong: see FND-007. |
| FND-002 | fixed | The component disjuncts end with `stutter`; AC-7 now gives 3 joint disjuncts with the match and 9 without, `stutter` last and the first variable varying slowest; TC-1120 step 1 matches. |
| FND-003 | fixed | `HyperQueryRequest.meter: Meter`; `match-conjunct` is tied to meter exhaustion; the byte-equality claim covers equal budgets. |
| FND-004 | fixed | `instance: &[(Name, Key)]`, the `HyperCounterexample.instance` binding in `clause.parameters` order, drawn from FR-125's instancing. |
| FND-005 | fixed | A satisfiable base now maps an undefined failure to an `Undefined` `HyperCounterexample` (HV-8, replayed) and a body violation to EN-2's `Lockstep` or `BoundReached{k}` (HV-3). |
| FND-006 | fixed | The Property sentence is reworded; distinctness is over `0..=depth`; AC-11 says "not a `HyperCounterexample`"; TC-1120 step 5 reports a skip and never a pass. |
| FND-007 | fixed | `VacuousMatch` is redefined once in FR-182's Outputs as "no matched fair universal tuple established". ADR-023 HV-4, HM-7 (amended) and HC-7, FR-176, FR-183, FR-315 and TC-607 are aligned, and every QSL reference was checked. The QSpec wire meaning remains: see FND-011. |
| FND-008 | fixed | FR-182 states the order: `NoInitialState` first, whatever witness comes with the outcome, then `VacuousMatch`. |
| FND-009 | fixed | The witness's instance must equal the outcome's instance. `μ_U` is evaluated under `witness.instance`. An instance that is not one of the clause's is `Refused` before re-execution. FR-182-AC-10 (the `{v, w}` vault) and FR-183-AC-8 carry vectors for both. |
| FND-010 | fixed | `check_match_witness(wire: ReplayRequestWire, subjects: &[ModelTraceSubject], witness: &MatchWitness, replay_limits: ReplayLimits)` uses existing `qsl-replay` types. `μ_U` is evaluated and charged once per tuple, independent of `depth`, which is sound because HS-5 labels read no state. |
