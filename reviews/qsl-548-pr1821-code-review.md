---
id: SR-1821001
title: "code review of PR #1821 (QSL-548 slices B and C): per-step simulation settlement"
type: SpecReview
analysis: code-review
date: 2026-10-29
scope: "agent-ix/quire-spec-language; git diff origin/terra-qsl-integration...748406e67 (qsl-replay model_check/step_safety.rs, step_settle.rs, certificate/alethe.rs, outcome.rs, proof_result plugin_read; qsl-analyze step_simulation.rs; qsl-route)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1821001: code review of PR #1821 (QSL-548 slices B and C)

## Summary

Head 748406e67. The review covers code review and Rust review, with soundness checked first.

**The alethe.rs string-edge refactor (7bb466853) keeps soundness.** It maps the operator, binder and Boolean spellings once, through `op`, `quantifier` and `boolean_literal`. It is otherwise mechanical. The sort check, the `la_generic` normalisation, `la_mult` and the `=` canonicalisation decide the same cases as before.

The #1788 probes were re-run at this head and reverted afterwards. The results are identical to #1788 round 1:
- The numeral `equiv_neg1`/`equiv_neg2` forgery is rejected `ProofStepInvalid`, at step 1.
- The non-integral strengthening is rejected at step 3. Each `(* 0.5 x)` and `(/ 1 2)` variant is rejected.
- The `ite` Int literal, the Bool `la_tautology` and the `(* 3 (/ x 3))` probes are rejected.
- The forged CasRefinesCounter certificate gives `Err(ProofStepInvalid at ProofStep{Base, 1})`.
- The valid steps `t0b` and `t0c` are accepted.

**The settlement (step_safety.rs, step_settle.rs) follows FR-314 "Outputs" and FR-468:**
- Only `Verified` gives `Certified`.
- `Unverifiable` and `Unencodable` give `Uncertified`.
- `Stopped` gives V-7 with its cause, and `Fault` gives `failed`.
- `Err` rejects only the record of its part, and leaves every other record `Uncertified`.
- A rejection with no record still settles the half `certificate-rejected`.
- A refused `Initial` gives `QueryMismatch` at `Base`.
- The half applies these in rule order: initial counterexample (refuted only through replay), hypotheses not established, step counterexample, rejection, other results, then the proof.
- A vacuous hypothesis proof (`Checks{0}`) counts as not established.
- `HypothesesNotEstablished` is an inconclusive cause, and its wire form and plugin read round-trip.

**Probes (reverted).** `settle_step_simulations` accepts any records list and any hypotheses list:
- With only the `Initial` record and no certificate, the half settles `Proved{Inductive{1}}`, `Uncertified`. The seven step queries have no result at all.
- With only the `Initial` record and a verified certificate, the half is `Certified`.
- A ninth, extra record is accepted.

Gates at 748406e67, with QSPEC at quire-specification origin/main 42428d76 (past ccae09a2):
- package clippy (qsl-replay, qsl-analyze, qsl-route, `--all-features --all-targets -D warnings`): exit 0;
- tests, all exit 0: qsl-replay (lib 919, it 167, and every facade target), qsl-analyze (lib 94, it 1063) and qsl-route (21 and 77).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **`settle_step_simulations` trusts the caller's record count and hypothesis list.** It sets `rows = records.len() - 1` and takes `hypotheses` as given, checking neither against the refinement. If only the `Initial` record is passed, the half settles `proved` (`Uncertified` with no certificate, `Certified` with a valid one), although the seven step obligations have no result. An empty hypotheses list drops FR-468's "hypotheses not established" rule: the rule that keeps a false invariant from making every step query vacuous. **Fix:** derive the step-record count and each record's `hypotheses` from the request's refinement, for example `step_simulations()`. Refuse a records list of any other length, and a hypotheses list whose clauses differ from the records' `hypotheses`. | FR-146 "Routing and settlement"; QSpec FR-468 "Settlement" |
| FND-002 | low | **FR-468's order is ambiguous, and the code takes rule order.** The spec says "the first that applies in the order initial query, hypothesis items, then step queries". FR-468-AC-7 says the half takes the first other result "whatever the other queries returned". The code (and `other_backend_results_settle_the_record_and_the_half`) lets a step counterexample outrank an earlier `unknown`. Both outcomes are non-`proved`, so this is not a soundness issue. The fix is to settle the wording on the spec side (QSpec FR-468). | QSpec FR-468 "Settlement", FR-468-AC-7 |

## Verdict

Changes requested on FND-001. FND-002 is a spec-side note. The proof-checker refactor reopens neither of the #1788 holes.
