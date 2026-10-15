---
id: SR-11084
title: "QSL-501 quire-spec-language PR 852 code review: union and expression case frontend"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@da5e8f44f9ff7687813e37e545d76603f2ab6f8d; qsl-cst/src/{cst,grammar,token}.rs, qsl-forms/src/{dispatch,lib,sum_case,syntax,value}.rs, qsl-semantics/src/check/{assemble,check/typing,checked_dispatch,family}.rs, qsl-replay/src/spine.rs, qsl-semantics/tests/it/variant_fields.rs, qsl-forms/tests/it/sum_case_forms.rs, tests/it/parser_differential.rs, tests/fixtures/parser-differential/baseline.txt"
review_set: subset
---

## Summary

Ticket: QSL-501 (slice S2, FR-313). Code review with the rust-review lane,
scoped to `git diff origin/main...HEAD` (2 commits: 8e9673864 cherry-pick of
the FR-313 parser forms, da5e8f44f adaptation to main's checker). Read in
place; no build was run (brief: no builds). The coder's `make ci` log was
not re-run.

Checked:

- Refusals. `AssemblyCause::UnsupportedUnion` maps to
  `unknown_required_feature/unsupported-feature`, the same catalogued pair
  as `FloatingType` and `UnsupportedModelMember`. `ExprNode::Case` in
  `Typer::infer_form` refuses with the existing `CheckCause::UnsupportedFeature`.
  Both are typed refusals with catalogued codes. Neither drops the form or
  falls back to another meaning.
- Reserved words. `case` was already a shared keyword (`token.rs:159`), so
  no expression starting with `case` was admitted on main. `union` is newly
  reserved in the `complete:` section only. `composed_base` unreserves
  complete-only words, so composed sources are unchanged. FR-313 requires
  the keyword ("S1 SHALL treat `union` as a keyword"). No tracked fixture or
  corpus uses `union` as an identifier.
- Protocol `choice`/`case`. `P::Case` and `P::Choice` are unchanged. No
  production tries an `Expression` at a position where a protocol `case`
  token comes next. `Block` closes with `}` and `Visibility` closes with `)`.
- Differential baseline. Checked by reasoning, not by re-running. The
  generator's only `case` is the protocol choice at
  `parser_differential.rs:242`. It never generates `union`, and the
  mutations insert only `( ) in then , not + [ } x`. The new alternatives
  start with `case` or `union`. So no admitted input can become refused,
  and no refused input can complete as a `CaseExpression`. Buckets 8 and 10
  are unchanged, which fits buckets with no refusal.
- `family.rs`: one new `ExprNode::Case` arm. Every existing arm's encoding
  is unchanged, so no existing unit's fingerprint changes.
- Panics: the `expect` and `unreachable!` calls in `grammar.rs`
  `scrutinee_rules` run only while the grammar is built, and they follow
  the existing convention at `grammar.rs:70`. No new `unwrap` appears in
  non-test code.
- Cherry-pick leftovers: none. Every file in the diff belongs to FR-313 or
  to the adaptation.

## Verdict

PASS for this method: no blocking finding. One medium is recorded and does
not block. The FR-360-AC-5 contradiction is reported once, in gap-analysis
SR-11085 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Recorded (latent until FR-318). `PreserveBind` keeps an arm binder that collides with a rename target, so `case s { A(r): x + r; }` with `x -> r` captures as `r + r`. `let` binders are alpha-renamed in that case. No effect today, because typing refuses every `case`. | qsl-semantics/src/check/checked_dispatch.rs:824-831 |
