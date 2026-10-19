---
id: SR-12811
title: "Code review of state-graph claims at S1-S3 (QSL-535, PR #968)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f414b7f16304cb757661adc907c524a534418d24; PR #968; qsl-cst, qsl-forms, qsl-semantics/src/check, qsl-replay/src/spine/refinement.rs, src/complete/editor.rs, spec/functional/FR-165"
review_set: subset
evaluated_revision: "f414b7f16304cb757661adc907c524a534418d24"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: references
---

## Summary

Ticket: QSL-535 (E11fe), re-scoped by the planner to S1 grammar, S2
`StateGraphClaimForm`, S3 `StateGraph` family with FR-104 clause checking,
span-located refusals, and the (temporal-satisfaction, Unbounded) record with
form beside it. Per-object instances, the deadlock item (AC-3) and the
FR-361-AC-4 gate test are out of scope (STD-162, QSL-863) and not raised.
This file folds the `rust-review` lane into `code-review`.

Focused checks run with a private target dir: `cargo test -p qsl-semantics
-p qsl-forms -p qsl-cst -p qsl-replay` (all green except the known macOS
`model_identity_full_size::ten_thousand_edge_chain`), `cargo clippy
--all-targets -D warnings` on qsl-forms/qsl-semantics/qsl-cst (clean),
`cargo fmt --check` (clean). Behaviour probes ran on a scratch copy only.

## Verdict

NOT MERGEABLE until FND-001 is resolved. The grammar, form builder, the
syntactic refusals (fairness, leading/joining temporal operators, `self`,
`result`, `pre`) and the FR-104 clause-checker reuse are sound; refusals are
located at the right spans (probed: type mismatch, non-Boolean `from`,
unknown name, `pre` in `unique path` `to`). No panics, unsafe or unchecked
conversions in production code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A checked state-graph claim is retained on CheckedGraph and `check()` returns Ok, but no S4 emission writes it, so a compiled package silently lacks every claim the author wrote. The temporal-clause precedent refuses a checked-but-unemitted clause (`temporal_clause::not_emitted`); this family should do the same until emission lands. | qsl-semantics/src/check/mod.rs:1560-1615 |
| FND-002 | medium | A temporal operator not in leading or joining position (`c.n = 1 and eventually c.n = 2`, `(eventually c.n = 2)`, `not eventually c.n = 2`) fails at S1 `invalid_syntax`, not S3 `unsupported_construct`/`expression-form`; FR-165 says any contained operator refuses at S3, and the PR's spec edit says a predicate keeps every operator written in it. | qsl-cst/src/grammar.rs:1143-1167 |
| FND-003 | medium | `over (x: T)` is mandatory in all three productions, but FR-165 Behavior and ADR-022 §1 admit a claim with no `over` parameter (one instance) and Outputs carry `over: Option<OverBinding>`. Owner question: amend the spec or make `over` optional. | qsl-cst/src/grammar.rs:1094; qsl-cst/src/grammar.rs:1104; qsl-cst/src/grammar.rs:1120 |
| FND-004 | low | Two claims with the same name both check; `CheckedGraph::state_graph_claim(name)` returns the first. No duplicate-name refusal exists while claims have no node key. | qsl-semantics/src/check/mod.rs:2499 |
| FND-005 | low | `DomainKind::InfiniteTrace` is documented as a temporal formula under the infinite-trace profile, which a state-graph claim does not have; the wire label `infinite-trace` mislabels the domain. Behaviour (`finite_kind` None) is right; whether a distinct kind is wanted is an owner question. | qsl-semantics/src/check/state_graph_claim.rs:159-168 |
| FND-006 | low | A read of an operation parameter cannot be told from an unknown name; it refuses `missing_declaration`/`missing-name`, not the `unsupported_construct`/`expression-form` FR-165 names. A claim has no operation in scope, so the spec's "a parameter, a result" clause wants owner clarification. | qsl-semantics/src/check/state_graph_claim.rs:262-280 |
| FND-007 | low | Each predicate is checked against the package base node total, so the node limit applies per predicate rather than to the claim's cumulative total. | qsl-semantics/src/check/state_graph_claim.rs:383-391 |
| FND-008 | low | A `Predicate` cause whose spans do not reach the inner location yields `refusal_region` None: the refusal carries no source range. | qsl-semantics/src/check/region.rs:270-272 |

## Coverage

Examined every hunk of `git diff origin/main...HEAD`. Author decisions judged:
(1) a refusal at check is feasible now and has direct precedent, so this PR
should refuse, and the emission ticket owns writing the node (FND-001);
(2) InfiniteTrace is acceptable behaviourally, label question to owner
(FND-005); (3) acceptable, spec clarification to owner (FND-006); (4) conflicts
with spec, owner decision (FND-003); (5) see SR-12812.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The merge of origin/main (3fcce8cf5) replaced the removed `UnsupportedDeclarationForm` with `UnresolvedTypeName { name: format!("{:?}", over.head) }`. `over (c: Integer)` now refuses as `missing_declaration`/`missing-name` naming the Debug text `Builtin(Integer)`, although the type exists and simply is not an object type. | qsl-semantics/src/check/assemble.rs:3344-3352 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | A temporal operator inside a quantifier body (`exists(d in c.items: eventually d.n = 2)`), an `if` branch (`if c.b then eventually c.n = 2 else true endif`) or a comparison operand (`c.b = eventually c.n`) is still refused at S1 as `invalid_syntax`. The merged FR-165 says S1 SHALL NOT refuse a predicate for holding a temporal operator, and that `graph-predicate` admits one at any operand position. `StateGraphTerm` reaches only `not`, a unary operator, `Comparison` and parentheses, so the S3 `expression-form` refusal never runs for these shapes. The three AC-4 shapes and `c.b implies eventually ...` do parse. | qsl-cst/src/grammar.rs:1187-1200 |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | S3 runs the syntactic refusals before it resolves any name, so `{ eventually c.versionNumber = newName }` refuses `expression-form` at `eventually`. STD-176 (PR #986, in review) says S3 SHALL resolve names first and refuse `missing-name` at `newName` (its new AC-6 last sentence). Merged FR-165 does not decide this. If #986 lands as written, #968 fails it. The owner should choose: resolve names before the operator refusal, or change STD-176. | qsl-semantics/src/check/state_graph_claim.rs:409 |
| FND-012 | low | S2 does not keep operators and operands in source order. Within a node it emits tokens and operands immediately and defers nested nodes, so `eventually c.a until c.b` builds `[until, eventually]`. In `(c.a until c.b) until c.c`, element 0 is the outer (second) `until`. This contradicts `StateGraphPredicateForm`'s "Every temporal operator written, in source order". S3 is unaffected, because `syntactic_refusal` takes the minimum span start. The SG-R18 test asserts only `ops[0].0 == Until`, so it cannot detect the order. | qsl-forms/src/state_graph.rs:129-165; qsl-forms/src/syntax.rs:3141 |

## New findings (disposition pass 5)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | low | Names-first misses an unknown name that sits beside a `let`-bound name. `unresolved_name` checks each operand alone. When the clause checker stops at a `let`-bound name (`MissingName("n")`), the function skips that operand, so a later unknown name in the same operand is never seen: `let n = 1 in eventually n = newName` refuses `expression-form` at `eventually`, not `missing-name` at `newName`. `bindings` also ignores scope: in `if c.versionNumber = 1 then let n = 1 in eventually n = 2 else eventually n = 3`, the unbound `n` in the else branch is excused and the claim refuses `expression-form`. This contradicts FR-165 Predicates ("S3 SHALL resolve names first"). The claim is still refused with no checked claim, so the only defect is the wrong code. Binding the `let` names into the operand's view would fix both cases. | qsl-semantics/src/check/state_graph_claim.rs:361-401; qsl-forms/src/state_graph.rs:141-146 |


## Dispositions

Round 1, at 5456c3b03f01941ac09a6aeca5bc3f7344fa4801 (fix commit) on top of merge 3fcce8cf5. Every other merge resolution, in assemble.rs, region.rs and refinement.rs, keeps both #960's terminal code and the claim code correctly.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | mod.rs refuses every checked claim with `StateGraphCause::NotEmitted` (`unsupported_construct`/`not-yet-implemented`, located at the claim) unless the test-support `with_unwritten_state_graph_claims` is set; covered by `a_checked_claim_is_refused_as_not_emitted_by_default`. |
| FND-002 | fixed | `StateGraphFormula`/`StateGraphTerm` parse an operator inside a connective, a negation or parentheses; S2 keeps it at its span and S3 refuses `expression-form` at `eventually` (forms test and semantics test, all three shapes). |
| FND-003 | deferred | Planner ruling: `over` stays mandatory for now; STD-162 makes it optional. |
| FND-004 | fixed | `DuplicateStateGraphClaim` (`conflicting_declaration`) at assembly, located at the second name; tested. |
| FND-005 | accepted-no-change | Planner ruling: the `InfiniteTrace` domain kind is accepted for the Unbounded record. |
| FND-006 | accepted-no-change | Planner ruling: a parameter-like name keeps refusing as `missing-name`. |
| FND-007 | fixed | `check_predicate` now starts each predicate from the claim's running total; `the_node_limit_applies_to_the_claims_cumulative_total` shows a limit one predicate fits under refuses the two-predicate claim. |
| FND-008 | fixed | `Predicate.span` is now `Span`, falling back to the operand root and then the claim span. |

Round 2, at f797d36addb7be8a8ed3044eeb09debfbce1be3b (fix commit), on top of merge 9e9dbc6d4. That merge brings in specs and reviews only and touches no claim code. The fix was reviewed statically: the focused build could not run because the shared disk was full (ENOSPC, 950 MiB free).

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | fixed | A new `AssemblyCause::OverNotObjectType { claim }` (`ill_typed`/`type-mismatch`, located at the type) replaces the Debug-formatted `UnresolvedTypeName`. The replay message reads "the `over` type of state-graph claim `P` is not an object type". Every `AssemblyCause` match site was updated. The new test is `an_over_type_that_is_not_an_object_type_refuses_at_the_type`. |

Round 3, at b82b29bc8bee5d29ca40a5df3bb6e80eb2d4760b (merge of origin/main b5c704456, which also carries the optional-`over` change). The conflicts in refusal.rs and region.rs are resolved correctly: both `CheckCause::StateGraph` and `ProtocolObligation` keep their `code`, `cause` and region arms. The four other `ClauseView` callers (state clause, operation clause, terminal, type invariant) all pass `Some(context)` and `Some(("self", ..))`, which is the previous behaviour; the postcondition parent-projection branch now also requires `Some(context)`, which those callers always supply. Focused tests (qsl-semantics with test-support, qsl-forms, qsl-cst, qsl-replay) are green apart from the known macOS `ten_thousand_edge_chain`; `clippy --workspace --all-targets -D warnings` and `fmt --all --check` are clean.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | `over` is optional in all three productions (`opt(s(vec![x("over"), bound_parameter()]))`), S2 builds `over: Option<ParameterForm>`, and S3 records `over()` `None` with no slot bound, as merged FR-165-AC-4 requires. Covered by `each_form_parses_without_over`, `a_claim_without_over_checks_and_records_no_binding` and `a_claim_without_over_binds_no_object_name`. |
| FND-010 | deferred | STD-176 (planner): FR-165 Parsing, AC-4 and ADR-022 SG-4 are to be aligned with what QSpec's merged graph-predicate recognises. QSpec makes a temporal operator inside an `if` condition, a `let` binding or a primary's operand (call argument, quantifier body) a syntax error, so FR-165's "any operand position" wording is too broad. The parser alignment commit needs its own pass. |

Round 4, at 8fe62d1e35b3456c07f81bed858c67ea8e67c311 (S1 aligned to QSpec `graph-predicate`, after a clean merge of origin/main), checked on a clean `git archive` export of that commit.

- **Grammar.** The grammar matches quire-specification origin/main 4f65ed2 shared-grammar.md lines 574-601, production by production:
  - optional `over` in all three clauses;
  - `always possible` in the order over, then from, then body;
  - `unique path` ending in `;`;
  - `graph-body` takes fairness only at its start;
  - the let/if/implication alternatives;
  - right-recursive `implies`;
  - at most one binary temporal operator per relation.
- **Expr-first rule.** `graph-unary` tries `Comparison` before `not`, temporal and `(`. Together with the `Expression` + `Peek` alternative, this realises QSpec's "derives as an `expr`" rule: `not c.n = 2` reads as one expression. No phrase without an operator reaches S3 with more than one operand (probed: `(c.a) and (c.b)`, `let`/`if` bodies, chained `implies`).
- **Syntax-error locations.** Each matches QSpec SG-R13/R15/R18 and STD-176's AC-6 (probed):
  - `;` for fairness after a body predicate or inside `from`;
  - `fair` after a `unique path` clause;
  - `eventually` in an `if` condition, a `let` binding, `exists(d in c.items: ...)`, a call argument and a comparison operand;
  - the second `until` in a flat chain.
- **Peek terminal.** It is used only in `StateGraphPredicate`, is zero-width, and records an expected-set entry only on mismatch.
- **Reserved words.** `possible`, `unique` and `path` are complete-facet reserved words, as QSpec shared-grammar.md line 702 states. Member names (`MemberName`) still accept them. No in-repo source broke.
- **Tests.** qsl-cst, qsl-forms, qsl-semantics (test-support), qsl-replay, qsl-eval, qsl-package, qsl-route, qsl-analyze and the root crate all pass, apart from the known macOS failures `ten_thousand_edge_chain` and `cli_preserves_os_paths_and_refuses_non_utf8_labels_before_io`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-010 | fixed | The predicate is now QSpec's `graph-predicate`. A temporal operator in a quantifier body, a call argument, a comparison operand, an `if` condition or a `let` binding is an S1 syntax error at the operator, which is what QSpec and STD-176 require. One in an `if` branch, a `let` body, a group, a connective or a `from`/`to` operand parses and is refused at S3. |

Round 5, at f43e510325272eae2c120e9cc663324a2d0eecb9 (after a merge of origin/main a14b0b3e1), checked on a clean `git archive` export of that commit.

- **Merge resolution.** assemble.rs keeps both `state_graph_claims(...)` and main's three-argument `operation_bodies`. layer.rs imports both `OperationBodyForm` and `StateGraphClaimForm`. The claim path computes `model_gate::selects_model(profiles)` and passes it to `LayerAdmission::new`, the same way the three other callers do.
- **Names-first.** `over`-bound names resolve through the receiver slot, and a name bound by a quantifier inside an operand resolves within that operand (probed). The one gap is `let`-bound names (FND-013).
- **Tests and lint.** Focused tests are green on qsl-cst, qsl-forms, qsl-semantics (test-support) and qsl-replay, apart from the known macOS `ten_thousand_edge_chain`. `clippy --workspace --all-targets -D warnings` and `fmt --all --check` are clean.

| FND | outcome | reason |
| --- | --- | --- |
| FND-011 | fixed | When a syntactic refusal is found, `check_claim` now first takes the earliest `missing-name` among the predicate's operands. `{ eventually c.versionNumber = newName }` refuses `missing-name` at `newName`, as merged FR-165-AC-5 requires (test `an_unknown_name_beside_an_operator_refuses_as_a_missing_name`). Residual edge: FND-013. |
| FND-012 | fixed | `temporal_operators.sort_by_key(\|operator\| operator.span.start)`. Two tests now assert exact offsets and kinds: `[eventually, until]` and the inner `until` before the outer one. |

Round 6, at 75fd07be96bb0818d8389c5af43c262c3f47d9a7 (after a merge of origin/main 8711b7802), checked on a clean `git archive` export of that commit. The merge resolution is correct: token.rs keeps both the claim words and main's `Arrow`, and lib.rs keeps main's comment. Focused tests (qsl-cst, qsl-forms, qsl-semantics test-support, qsl-replay) are green apart from the known macOS `ten_thousand_edge_chain`; `clippy --workspace --all-targets -D warnings` and `fmt --all --check` are clean.

The new tests would have failed before the fix: at f43e510 both vectors refused `expression-form` (round-5 probes), and they now refuse `missing-name`. Extra probes, all correct:

| Probe | Result |
| --- | --- |
| nested `let a = 1 in let b = a in eventually b = zz` | `missing-name` at `zz` |
| the same with `b = 2` | `expression-form` |
| a binding value with an unknown name, `let b = qq` | `missing-name` at `qq`, span mapped into the value |
| shadowing `let n = 1 in let n = qq in ...` | `missing-name` at `qq` |
| a `let` closed inside a group, read outside it: `(let k = ... in k = 1) and eventually k = 2` | `missing-name` at `k` |
| `let k = 1 in (k = 1 and eventually k = ww)` | `missing-name` at `ww` |
| an else-branch `let m = n` with `n` bound only in the then branch | `missing-name` at `n` |

| FND | outcome | reason |
| --- | --- | --- |
| FND-013 | fixed | S2 records, index-aligned with the operands, the `let` scope each operand sits under, outermost first, with each name's value form. A binding value sits outside its own `let`. S3 rebuilds each operand wrapped in its real `let`s, checks it, and maps refusal paths back to the operand or binding-value spans. Tested by `let_names_are_scoped_when_names_resolve_first` and the S2 `let_bindings_are_kept`. |
