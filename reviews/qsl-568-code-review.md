---
id: SR-12163
title: "Code review of quire-spec-language PR #1000: random parameters, workloads, rewards and statistical claim forms"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@af798d76e2f850955074e0bf48bcd75cd5ee7899; PR #1000 diff against origin/main: qsl-cst/src/{cst,grammar,lexer,parser,token}.rs; qsl-cst/tests/it/{complete_grammar,probabilistic}.rs; qsl-forms/src/{dispatch,lib,probabilistic,syntax,temporal_clause,value}.rs; qsl-forms/tests/it/{probabilistic_forms,protocol_clause_forms,value_forms}.rs; qsl-foundation/src/diagnostic.rs; docs/native-error-codes.md; qsl-semantics/src/check/{probabilistic,assemble,check,check/typing,checked_dispatch,family,layer,mod,refusal,region,temporal_clause}.rs; qsl-semantics/tests/it/probabilistic_declarations.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/refinement.rs; src/complete/editor.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1000

## Summary

Ticket: QSL-568. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (36 files, +3415/-30). Read-only: nothing
was built or run, at the dispatcher's instruction. The coder reports `cargo
check`, `clippy --all-features -D warnings`, `fmt --check` and the qsl-cst,
qsl-forms, qsl-semantics and qsl-replay tests green; those claims were not
re-run here. QSpec read at its current main, cited by id.

Checked and clean:

- Grammar matches QSpec's shared-grammar "Probabilistic, timed and
  real-time forms" EBNF production for production: `exact-value`,
  `quantity-literal` (as a `primary`), `random-decl`, `weighted-value`,
  `reward-decl`, `workload-decl`, `workload-weight`, `probabilistic-clause`,
  `scheduler-clause`, `fairness-item`, `confidence`, `confidence-item`,
  `probabilistic-form`, `bound-op`, `event`, `measure`, `fraction`,
  `weighting`, `horizon`. Ordered-choice placement (`QuantityLiteral` before
  `uint` in `Primary`, `Measure` before `TemporalExpression` in `Event`) is
  what a PEG needs; `delay-decl` is out of scope (FR-204).
- No new reserved keyword. Every leading and inner word is a `literal(...)`
  terminal, which `complete_reserved_words` does not collect. Every `x(...)`
  word the family uses (`on`, `using`, `over`, `holds`, `until`, `fair`,
  `weak`, `strong`, `each`, `of`, `from`, `within`, `by`) was already an `x`
  terminal on main, so the reserved set is unchanged. The PR adds a test that
  all 23 contextual words stay record, field and type names. The #961
  `terminal` contextual literal is untouched and `#![recursion_limit = "256"]`
  is unchanged.
- `QuantityLiteral` as a primary cannot change the meaning of previously
  parsing source: it needs an `ExactValue` followed by a `QualifiedName`,
  whose first token is a non-reserved identifier. On main every follower of
  `Expression` (and of the `Primary`/`Postfix` chain) is a reserved word or
  punctuation (`in`, `then`, `else`, `implies`, `;`, `)`, `]`, `}`, `,`,
  `:`, operators), and case arms end in `;`. No old source has an integer
  directly followed by a non-reserved identifier.
- The `~` token: on main `~` was a lex error (`unexpected source
  character`), so no previously parsing source contains it. The historical
  recognizer keeps refusing it (`Kind::Tilde` joins the complete-only list in
  `lexer.rs`).
- `DecimalLiteral` admits a `Fractional` token with no exponent; `1.5e3`
  refuses, tested. `decimal(c, s)` takes a `uint` scale (QSpec), so no
  negative scale reaches `magnitude`. A scale beyond `u64` is correctly
  below one; `ratio` expands at most `10^4096`.
- `invalid_probabilistic_model` is registered consistently: `Code` variant,
  `as_str`, `Code::all`, `CATALOG_CATEGORIES` (array length 49 to 50,
  `Refusal`), `Code::category` (falls to `Refusal`) and the
  `docs/native-error-codes.md` row, whose cause list matches
  `ProbabilisticCause::tag` exactly. QSpec FR-272 and native-diagnostics
  catalog the code; `invalid-confidence` is not in their closed cause set
  (declared conflict 6 in the PR, not re-raised here).
- Soundness: `probabilistic::check` runs for every unit and gives at least
  one refusal for every probabilistic declaration (`NotYetImplemented` when
  no form check fires), so none compiles. `DeclarationForm` has exactly one
  consumer (`assemble::Unit::new`), matched exhaustively. A quantity literal
  in any typed expression refuses in `Typer` (`ExprNode::Quantity`), and
  every other `ExprNode` match the PR touches is exhaustive. The grammar
  admits the four declarations at unit level only, so none can sit inside a
  protocol or other family.
- Form builders read contextual words by token position and refuse
  `unexpected` on any other shape; no `unwrap`, `expect`, indexing panic,
  `unsafe` or lossy integer cast in production code. `horizon_form` parses a
  `u64` that S1 already bounded.

## Verdict

PASS. No high finding. FND-001 (medium) is a wrong refusal cause for a form
QSL FR-195 admits; nothing compiles because of it. Three lows, recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `claim` refuses every temporal operator with no closed interval in a `probability` event as `unsupported_construct`/`expression-form` ("needs a finite window"), whatever the claim's profile. QSL FR-195 and QSpec FR-407's forms table admit `probability >= θ [eventually holds(P)]`, `[holds(A) until holds(B)]` and `[always holds(P)]` under the infinite-trace profile. S3 resolves no profile yet, so a spec-admitted exact-only claim gets a cause and message that say the form is invalid. Until the profile resolves, an unbounded operator should fall through to `not-yet-implemented` (or the check should run only when the selected profile is known to be bounded). | qsl-semantics/src/check/probabilistic.rs:561-576; qsl-semantics/tests/it/probabilistic_declarations.rs:327 |
| FND-002 | low | `dispatch_call` repeats the syn walk of `dispatch_entry_is_a_single_thin_call` (find `fn dispatch`, find its `match`). Extract one helper that returns the match and use it from both tests. | qsl-forms/src/dispatch.rs:654-703 |
| FND-003 | low | `Horizon` is `QuantityLiteral \| bound()`. A count above `u64` refuses `invalid_syntax` at the token after the number (`)`), because the `QuantityLiteral` alternative reads further than `bound` and wins the failure merge. The test pins the `)` locus. Point the refusal at the number, or say in the test why `)` is the locus. QSpec's `horizon` is `uint`; FR-392's `Spanned<u64>` narrowing is QSL's choice. | qsl-cst/src/grammar.rs:2182; qsl-cst/tests/it/probabilistic.rs:216 |
| FND-004 | low | The repeat checks are quadratic and charge no meter: `form.support[..index].iter().any(same_literal)`, the workload `weights[..index]` scan and `distributions.contains`. The default 50 000-node ceiling keeps this small, but a caller-raised ceiling does not. A set keyed on the literal (or the spelled name) makes each check linear. | qsl-semantics/src/check/probabilistic.rs:289; qsl-semantics/src/check/probabilistic.rs:437-439; qsl-semantics/src/check/probabilistic.rs:481-484 |

## New findings (disposition pass 1)

Reviewed at `ed2402498e5fe74036bc2c378d6ad62c54fac195` (fix commits
c29cae523, 8e699a819, ed2402498; merges of main incl. #1001 and #1007). New
behaviour checked and clean: `duplicate-reward` (keyed on operation and
reward name as written, at the second declaration); `quantile-over-fraction`
(at the fraction's span); a repeated confidence parameter refuses
`partial-confidence` naming it, before any range check; `rational(-1, 1)` as
a weight refuses `invalid-weight` through `magnitude`'s sign test;
`EventForm::Temporal` holds `TemporalFormulaForm`, as FR-392 now states. The
`docs/native-error-codes.md` row lists exactly the causes `tag()` emits under
`invalid_probabilistic_model` and marks `duplicate-reward`,
`quantile-over-fraction` and `invalid-confidence` QSL-proposed pending
STD-188. No `s3.rational_bits` setting, `CheckingLimits` field, limit cause
or test remains in code, docs or tests; the only mentions are two comments
saying the ceiling waits on QSL-883, and the PR body says the same. Exact
values: `magnitude` reads sign, `decimal_digits` (a radix-10 rendering of a
source-written integer) and one comparison; nothing is reduced; no `pow`,
`gcd` or unbounded shift; no panic path. `ratio` is the one expansion, below.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The module doc says a written exact value is never reduced, and `huge_written_exact_values_are_read_without_expansion` says "never reduced or expanded". `ratio` does expand `10^s` for a decimal scale up to `MAX_EXPANDED_SCALE` (4096, about 13,600 bits) and multiplies written integers for the θ ± ι comparison. That is bounded and correct, but the statement is not true of every exact-value path; say "expanded only up to scale 4096 for the confidence range". The test asserts only the refusal, so it cannot fail on an expansion either. | qsl-semantics/src/check/probabilistic.rs:24-27; qsl-semantics/src/check/probabilistic.rs:418-436; qsl-semantics/tests/it/probabilistic_declarations.rs:228-247 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c29cae523 |
| FND-002 | deferred | low; not fixed in this PR, recorded under the lane's majors-only fix policy |
| FND-003 | deferred | low; not fixed in this PR, recorded under the lane's majors-only fix policy |
| FND-004 | deferred | low; not fixed, and the new `rewards.contains` check has the same shape; recorded under the lane's majors-only fix policy |
| FND-005 | deferred | low; new this round, recorded under the lane's majors-only fix policy |
