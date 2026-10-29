---
id: SR-1788001
title: "PR #1788 (QSL-1219) code review: FR-315 encode_refinement_query and FR-314 check_refinement_smt_proof with the Alethe checker"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/terra-qsl-integration...0e8981915 (qsl-replay certificate/alethe.rs, certificate/smt.rs; qsl-eval smt/refinement.rs, relation.rs); first read at cd02b49f3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: reviews
---

# PR #1788 code review (Rust, soundness first)

## Summary

Ticket: QSL-1219 (QSL-548). First read at cd02b49f3. Re-reviewed at 0e8981915, which folds in QSL-1231: anchor contexts, `bind`, binders and `let`, `define-fun` with parameters, exact or searched resolution pivots, the `and`/`not_or` index, and a refused `Initial` settling `QueryMismatch` at `Base`.

**What holds.**
- **`check_refinement_smt_proof` pipeline.**
  - It returns `Verified` only after every check below has passed:
    - the shape;
    - byte equality of every encodable part against FR-315's printing;
    - a read of every proof;
    - the empty-proof check;
    - `ProofStepInvalid` over every command;
    - the last command concluding `(cl)`;
    - no command `Undecided` in any part.
  - A refused row gives `Unencodable`, never `Verified`.
  - Proofs are checked against the expected query, not the carried one.
- **Top-level `assume`.** It must be an assertion of the query, compared after expansion.
- **Proxies.** `:named`, `define-fun` (with or without parameters) and `let` are expanded consistently. A proof that redefines a query symbol only changes what its own text abbreviates: every rule judges the expanded terms, so no shadowing forges a link to the query.
- **Anchor contexts.**
  - Inside a non-empty context only `refl`, `trans` and `cong` are decided, under `subst(Γ)` with capture-avoiding substitution and alpha-equality.
  - Local assumptions cannot feed them: an `assume` has no oriented equality.
  - `bind` checks that the anchor fixes and maps the variables, freshness, and the last oriented step.
  - A context subproof closed by any other rule is `Undecided`.
- **Resolution.** Named pivots and the bounded search both demand the conclusion as an exact literal set.
- **Shape locus.** `ShapeMismatch` is located at `Step { row: min(present, rows) }`.

**Soundness: two ways to verify a false refutation, both confirmed at 0e8981915** (probe_1788_0e89.diff, reverted).

- **No sort check: Int-sorted literals are reasoned about as Booleans (FND-001).** Nothing checks that a clause literal is Boolean. The equivalence rules read any `=` as `↔`.
  - Three steps of `la_tautology` derive `(cl (not (= 0 1)))`, `(cl (not (= 0 2)))` and `(cl (not (= 1 2)))`.
  - `equiv_neg1` and `equiv_neg2` over `(= a b)` give `(cl (= a b) (not a) (not b))` and `(cl (= a b) a b)`.
  - Resolution leaves the numerals `0`, `1` and `2` pairwise "different Booleans", and four more resolutions reach `(cl)`.
  - The proof assumes nothing, so it "refutes" every query.
  - Every part of CasRefinesCounter's certificate given this proof makes `check_refinement_smt_proof` return `Ok(Verified)` (at cd02b49f3). The Alethe reader returns `Refutes` for it at 0e8981915.
  - The same gap opens the `ite*`, `not_ite*`, `equiv*`, `not_equiv*`, `equiv_pos*`/`equiv_neg*`, `not_not`, `and`/`or` and `eq_congruent_pred` shapes to Int and Real terms.
- **Integer strengthening with non-integral coefficients (FND-002).** `la_generic` strengthens `s > d` to `s ≥ ⌊d⌋ + 1` whenever every atom of `s` is an `Int` constant, whatever its coefficient.
  - It therefore accepts `(cl (not (> (* (/ 1 2) x) 0)) (not (< x 2)))` with `:args (2 1)`. That clause is false at `x = 1`.
  - Together with the valid `(cl (not (> x 0)) (> (* (/ 1 2) x) 0))`, it refutes the satisfiable query `x > 0 ∧ x < 2`.
  - The reader returns `Refutes` at 0e8981915.
  - The two-literal `la_tautology` path calls the same check.

**Deviations listed in the PR body.**
1. **A refused `Initial` query.** It now rejects `QueryMismatch` at `Base`, not `Unencodable { part: Base }`. Either way it can never be `Verified`. FR-314 does not state this case, so record it on the spec side (FND-004).
2. **`and` without its index.** It is resolved at 0e8981915: `and`, `not_or`, `and_pos` and `or_neg` now require the index.
3. **`H`'s tie-break by package order instead of declaration identity text.** This fails closed: another producer's spec-conformant query would be rejected `QueryMismatch`, so it is not a soundness issue. It is still a conformance deviation, and the fix is to carry the clause's `DeclarationKey` (FND-003).
4. **The fixture's `add` postcondition is guarded by `pre(self.value) < 2`.** This is a fixture deviation from FR-315-AC-6's text, forced by S3's range check. It is acceptable in a test, but the AC text and the fixture now disagree, so record it on the spec side (FND-004).

**Gates** at 0e8981915 (`--all-features`, debug info off, QSPEC_DIR at qspec-63ff21f1), all exit 0: qsl-eval (lib 140, it 823) and qsl-replay (lib 915, it 160); clippy over qsl-eval, qsl-replay, qsl-semantics and qsl-analyze with `-D warnings`; fmt. They pass without exercising either hole.

## Verdict

**Not mergeable.** FND-001 and FND-002 each let `check_refinement_smt_proof` return `Verified` for a refutation that refutes nothing, so a refinement that does not hold can settle `proved`, `Certified`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | **No sort check on clause literals.** `equiv_neg1` and `equiv_neg2` over Int `=` (with `la_tautology`'s `(not (= 0 1))`) give a zero-assumption refutation, and every part carrying it is `Verified` (`Ok(Verified)` on CasRefinesCounter). **Fix:** type every literal, and every operand that a propositional rule reads as a formula, against the query's sorts. Refuse non-Boolean ones. | FR-314 "Checked Alethe rules" ("checks each command against the rule's definition"), FR-314-AC-4, qsl-replay/src/certificate/alethe.rs `tautology`, `follows` |
| FND-002 | high | **`la_generic` strengthens over non-integral coefficients.** `(cl (not (> (* (/ 1 2) x) 0)) (not (< x 2)))` with `:args (2 1)` is accepted, and with it a refutation of the satisfiable `x > 0 ∧ x < 2` is accepted. **Fix:** strengthen only when every coefficient of `s` is an integer, after scaling `s` to integral coefficients as Alethe's step 4 assumes. | FR-314 `la_generic` ("checked by exact rational arithmetic"), alethe.rs `la_generic`, `integer_sorted` |
| FND-003 | low | **The hypothesis tie-break uses package order, not declaration identity text.** This applies when two invariant clauses share a name on different contexts (PR deviation 3). It fails closed as `QueryMismatch` against a conformant producer. **Fix:** carry the declaration key. | FR-315 "Refinement queries" (STD-556 order) |
| FND-004 | low | **Two spec gaps to record.** A refused `Initial` query has no stated FR-314 outcome; the code rejects it `QueryMismatch` at `Base`. FR-315-AC-6's `add` postcondition differs from the fixture's guarded one. Neither changes a verdict. | FR-314 Behavior, FR-315-AC-6 |
