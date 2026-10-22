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
