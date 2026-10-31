---
id: SR-1935001
title: "code review of PR #1935 (QSL-1306): refinement-query normal form Term::normal"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1935 git diff origin/main...13402c26a (qsl-eval/src/smt/normal.rs, qsl-eval/src/smt/refinement.rs, qsl-eval/src/smt/mod.rs, qsl-replay/tests/it/refinement_smt.rs, tests/support/refinement_smt_proofs.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-315
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-314
  type: references
---
# SR-1935001: code review of PR #1935 (QSL-1306)

## Summary

Ticket: QSL-1306. Code review with the rust-review lane folded in, scoped to the PR diff at head 13402c26a3761569a9dfee393465dbfa9e31dac9. Major bugs first, soundness first. Focused runs: `cargo test -p qsl-eval --lib smt::normal` 5/5, `cargo test -p qsl-replay --test it refinement` 90/90, `cargo test -p qsl-analyze --test it refinement` 140/140. With the four `.normal()` calls in `encode_refinement_query` reverted, 7 `refinement_smt` tests fail, `refinement_queries_are_in_normal_form` among them.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Term::normal and helpers | qsl-eval/src/smt/normal.rs:31-239 | examined |
| encode_refinement_query | qsl-eval/src/smt/refinement.rs:139-270 | examined |
| Term constructors and printer | qsl-eval/src/smt/script.rs:65-260 | context_only |
| FR-314 query comparison | qsl-replay/src/certificate/smt.rs:232-270 | examined |
| expression encoder (mod, times) | qsl-eval/src/smt/expression.rs:332-375 | context_only |
| Alethe refutations | tests/support/refinement_smt_proofs.rs | examined |
| refinement_smt tests | qsl-replay/tests/it/refinement_smt.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `mod` over a literal dividend is left unfolded, so a constant-foldable `(mod k n)` survives normalisation when an argument bound as a literal is the dividend | qsl-eval/src/smt/normal.rs:54 |

## Verdict

**Approve: no high findings. Mergeable once the medium finding is fixed or explicitly deferred.**

Soundness, rule by rule (all hold over SMT-LIB Int/Bool, where every function is total):

1. `negation` and `(=> l r)` to `(or (not l) r)`: equivalences.
2. `junction`: flattening, dropping the neutral constant, dropping structurally equal repeats, and returning the absorbing constant are all valid for `and`/`or`. `kept.remove(0)` runs only when `len == 1`.
3. `equality`: `(= t t)` is true for any total term. `mod` only exists with a nonzero literal divisor (expression.rs:362-371), and there is no `div`, so no partial term exists in the SMT layer. `(= t c)` for a Boolean constant is `t`/`(not t)`. Two literals compare exactly.
4. `comparison`: `(< t t)`/`(> t t)` false, `(<= t t)`/`(>= t t)` true. Correct.
5. `shifted`: `(+ t k) ~ d` iff `t ~ d-k`, and `d ~ (+ t k)` iff `d-k ~ t`, for `<`, `<=`, `>`, `>=` and `=`. `Integer` is exact, so negative literals and large values cannot overflow. A normal sum has at least two summands with at most one literal at the end, so the remainder is never a literal and the relation cannot become literal against literal.
6. `choice`: the four Boolean-constant-branch rewrites are correct, including both-constant cases (`(ite p true false)` gives `p`, `(ite p false true)` gives `(not p)`). Equal branches fold, which is fine because the condition is total.
7. `sum`/`difference`/`product`: reordering literals to the end, `(- t t)` to 0, `(* 0 t)` to 0, `(* n (* m t))` to `(* nm t)`. Correct.

Removing the text-keyed `HashSet` dedup of disjuncts is safe: rule 3 now drops structurally equal disjuncts after normalisation, which also matches FR-315-AC-54's "conjunctions print equal". Each assertion is replaced by an equivalent term, so no verdict changes and FR-314 still proves the same obligation.

FR-314 consistency: the checker (qsl-replay/src/certificate/smt.rs:244) and the `qsl_replay::encode_refinement_query` facade (qsl-replay/src/refinement_query.rs:221) both call the one `smt::encode_refinement_query`, so producer and checker both see the normalised bytes. No `.smt2` golden or stored certificate in the repo carries the old bytes. The rewritten Alethe refutations check `Verified`, and the `hole` test still replaces command 3.

Tests are not tautological. They fail on the pre-PR encoder, and the `unnormal` walker checks itself against a probe term that has one of each shape.

Rust lane: no `unsafe`, no new panics, no new integer casts. Recursion depth matches the existing printer. `#[must_use]` is on `normal`.
