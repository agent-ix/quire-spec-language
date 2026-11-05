---
id: SR-2341001
title: 'Code review of PR #2341: quantity integer power e ^ n (QSL-1656/QSL-1700), code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@026b62234; git diff origin/main...026b62234 (PR #2341, branch peter/qsl-1656-quantity-power): qsl-cst/src/grammar.rs; qsl-cst/src/parser.rs; qsl-cst/src/lexer.rs; qsl-forms/src/syntax.rs; qsl-forms/src/value.rs; qsl-semantics/src/check/{check.rs,check/typing.rs,facts.rs,family.rs,hyper_clause.rs,layer.rs,assemble.rs,checked_dispatch.rs}; spec/functional/FR-255 row; ticket QSL-1656'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1656 (with QSL-1700). PR: quire-spec-language#2341, head 026b62234, merge base 26bc56773. Method: code-review with the rust-review lane. One pass, scoped to the PR diff. QSpec was read by ID from a detached worktree at quire-specification origin/main e85daed5.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Unary rule `postfix [ '^' signed-int ]` | qsl-cst/src/grammar.rs:796-806 | examined |
| hyper_expression_rules Unary copy | qsl-cst/src/grammar.rs:1212-1243 | examined |
| DEFAULT_WORK_UNITS 256 -> 288 and steps table | qsl-cst/src/parser.rs:1250-1292 | examined |
| measured_work_leaves_headroom_under_the_budget | qsl-cst/src/parser.rs:2019-2058 | examined |
| ExprNode::Power and its S2 mapping | qsl-forms/src/syntax.rs:640-648; qsl-forms/src/value.rs:1637-1645 | examined |
| Typer::power (zero-budget probe) | qsl-semantics/src/check/check.rs:3798-3843 | examined |
| Power i128 bound and Unary frame | qsl-semantics/src/check/check/typing.rs:1268-1282 | examined |
| QuantityPower nonzero obligation | qsl-semantics/src/check/facts.rs:1170-1188 | examined |
| family encoding, hyper/layer/assemble arms | qsl-semantics/src/check/family.rs:551-555 and others | examined |
| FR-255 s1.work_units row | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:94 | examined |
| QSpec shared grammar power / indexed-read | quire-specification proposals/quire-v1/shared-grammar.md (power, indexed-read rules) | context_only |
| quire-semantic-value evaluate_quantity_unit | quantity.rs:446-453, 634-639 (locked rev 30c24931) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Inside a hyper `holds(...)`, `v.l @ a ^ 2` is refused `invalid_syntax` (measured), but QSpec's shared grammar admits `indexed-read` wherever it admits `postfix` and binds `@` tighter than every operator, so it must parse as `(v.l @ a) ^ 2`. `hyper_expression_rules` inserts `IndexedRead` as a sibling alternative of the power sequence, not in its postfix slot. Fix: in the `P::Unary` arm put `c([r(IndexedRead), r(HyperPostfix)])` in the power sequence's base position, drop the separate alternative, and add CST cases for `v.l @ a ^ 2` (admitted) and `v.l ^ 2 @ a` (refused). | qsl-cst/src/grammar.rs:1228-1232 |

## Verdict

Conditional. FND-001 must be fixed in this PR, or deferred to a ticket that names it. The rest of the change is sound.

Grammar (measured). A temporary dump test compared CST, diagnostics, recoveries and node identities on main and on the branch for 49 expressions and one dimension declaration. Every expression without `^` is byte-identical: unary/prefix chains, `*`, `div/rem/mod`, field/call/index, quantity literals, `-1 m`, `if`/`let`/`forall`/`set`/constructors, the malformed inputs, and `dimension A = L ^ 2 * L ^ -1`. `a ^ 2`, `-a ^ 2` (Negate over Power), `a * b ^ 2`, `a ^ 2 * b`, `a[b] ^ 2` and `a ^ - 2` are admitted. `a ^ 2 ^ 3`, `a ^ (2)`, `a ^ b`, `a ^ 2.5`, `a ^ 2 m` and `a.f ^ 2 .g` are refused at the offending token. `2 m ^ 2` is unambiguous because `QuantityLiteral` is `ExactValue QualifiedName` with no unit exponent.

Typing. The probe calls `evaluate_quantity_unit`, whose unit comes from the pure `type_check`. For Power that is `canonical_compound().power(n)` after the affine check, and it runs before any metering. The probe's meter is a local `NO_CHARGE` meter, never the caller's, so it cannot charge or leak anything. A magnitude-1 probe cannot divide by zero, and the first charge is denied before any arithmetic, so `2^127` exponents cost nothing. With n = 0 the result is the dimensionless compound unit, and `d ^ 1` over `ms` gives `s^1`. Both match QSpec's QP-02 and QP-03. The only `IllTyped` cause is affine, so mapping it to operator-ineligible is exact.

Facts. A non-literal or negative exponent is treated as negative, which is conservative. Only a nonzero quantity literal base discharges the obligation.

Judgement call, budget 256 -> 288: this is an honest recalibration, not a gamed gate. Steps per (token+1) were measured on main and on the branch.

| Input | main | branch |
| --- | --- | --- |
| 63 nested parens | 83.98 | 85.66 |
| `x = x and` chain | 82.91 | 84.88 |
| `x + x` chain | 78.22 | 80.20 |
| `x * x` chain | 72.79 | 74.77 |

The temporal, `Option<>` and `not` inputs are unchanged. The delta is exactly 4 steps per Unary operand: the outer Seq, Optional, inner Seq and the `^` Terminal. That is +2.0 to 2.5% on expression-heavy input.

Main already sat at 98.4% of the guard (83.98 against 85.33) because its doc table was stale by up to 15 steps per token. It showed 69.4 for parens, which really measured 84.0. Any rule at Unary would have tripped the guard.

No restructuring keeps 256:

- The cheapest encoding the combinators allow costs at least 3 steps per operand.
- Folding `^` into Postfix's suffix star costs 1 step, but it admits chaining and mixing (`a ^ 2 ^ 3`, `a ^ 2 .f`). The refusal would then move to S2, against AC-58's "before S2".

The guard's intent is preserved: the same inequality, steps <= budget/3, runs over the same inputs. The budget is a named, raisable S1 resource ceiling (FR-255 `s1.work_units`). Its worst-case memory grows 12.5%, from 25.6M steps (about 0.95 GiB) to 28.8M steps (about 1.07 GiB), which the doc still correctly calls "about 1 GiB". The FR-255 row and the lexer doc match the constant.

Pre-existing note, not this PR: the per-token cost rises with nesting depth. 200 nested parens measure 92.68 on main and 94.56 on the branch, so the guard's fixed depth-63 sample does not bound deeper nests.

Gates run on 026b62234:

- qsl-cst: all tests pass (lib 15, it 100).
- qsl-forms value_forms: 34 tests pass.
- qsl-eval `quantity_`: 18 tests pass.
- qsl-semantics lib with `--features test-support`: 812 pass. The 2 failures are the known main-red scan tests. Their messages point to `lower_node` and `qsl-forms/src/lib.rs:70`, neither of which this PR touches, so it adds no new scan failure.
- Root `it` deep_sources/formatter/work_units: 105 tests pass.
- Clippy, default and `--all-features`, on the four touched crates with `--all-targets -D warnings`: clean. `cargo fmt --check`: clean.
- Trial merge with origin/main e5ac77201, then `cargo check --locked --workspace --all-targets`: clean.
- Cargo.lock is unchanged. There is no compat layer, and no test was weakened. `quantity_literals` items were only widened to `pub(super)`.

## Dispositions

| Finding | Disposition |
| --- | --- |
| FND-001 (medium, `v.l @ a ^ 2` refused in a hyper condition) | Fixed. The hyper `Unary` power sequence takes `c([IndexedRead, HyperPostfix])` as its base and the separate alternative is gone. `qsl-cst/tests/it/hyper_clause.rs` admits `v.l @ a ^ 2` as `(v.l @ a) ^ 2` and refuses `v.l ^ 2 @ a` at the `@`. The work-budget guard passes at 288 unchanged. |
