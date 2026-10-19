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
