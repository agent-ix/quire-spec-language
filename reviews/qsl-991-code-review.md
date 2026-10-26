---
id: SR-1587001
title: 'Code review of PR #1587: HP-2 k-induction settlement (FR-182), code-review with rust-review lane'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@2d47d5bce; git diff origin/main...2d47d5bce (PR #1587, branch qsl-991-kinduction-settle): qsl-replay/src/lib.rs, qsl-replay/src/model_check.rs, qsl-replay/src/model_check/hyper_settle.rs, qsl-replay/src/model_check/hyper_smt_outcome.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/tests/it/hyper_smt_settle.rs, qsl-replay/tests/it/main.rs; ticket QSL-991'
review_set: subset
date: '2026-10-26'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-182
  type: reviews
---

## Summary

Ticket: QSL-991. PR: quire-spec-language#1587, head 2d47d5bce. Method: code-review, with the rust-review lane folded in. The PR adds `HyperSmtOutcome`, `HyperSmtResult::{Inductive, InductionNotClosed}`, `settle_hyper_smt`, `ProductMember` and two `HyperSettleRefusal` arms. The settlement logic that is built follows FR-182's identity-first order and V-3 guard order, never settles `proved` for a non-empty `μ_U`, and has no stub or tautology. The public types diverge from FR-182 Outputs in two ways: `product` is not a `Result`, and the `settle_hyper_smt` signature has no witness or replay-limit parameters. CG (IR-974, in progress) constructs these types, so the shape matters now.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-182 Outputs (`HyperSmtOutcome`, `HyperSmtResult`, `settle_hyper_smt`) | spec/functional/FR-182-settle-a-hyper-verdict.md | examined |
| FR-182 Behavior, identity check | spec/functional/FR-182-settle-a-hyper-verdict.md | examined |
| FR-182 Behavior, V-3 guards | spec/functional/FR-182-settle-a-hyper-verdict.md | examined |
| FR-315 Outcome handoff | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md | examined |
| CG FR-045 handoff table | quire-contract-codegen origin/main spec/routed/functional/FR-045-hyper-smt-induction-map.md | context_only |
| settle_hyper_smt, inductive_row, ProductMember | qsl-replay/src/model_check/hyper_settle.rs | examined |
| HyperSmtOutcome, HyperSmtResult | qsl-replay/src/model_check/hyper_smt_outcome.rs | examined |
| hyper_smt_settle tests | qsl-replay/tests/it/hyper_smt_settle.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `HyperSmtOutcome.product` is `HyperProductIdentity`, but FR-182 Outputs and FR-315 Outcome handoff define it as `Result<HyperProductIdentity, InternalFault>`, so the spec'd rule "item identity Ok and `outcome.product` Err refuses `ProductMismatch` naming `clause`" cannot be written, and CG, which constructs this public field, will take a breaking change later. Small: fix in this PR. | qsl-replay/src/model_check/hyper_smt_outcome.rs:35 |
| FND-002 | medium | `settle_hyper_smt` has no `witness: Option<&MatchWitness>` or `replay_limits: ReplayLimits` parameter, `HyperSmtResult` has 2 of the 10 spec'd variants, and `HyperSettleRefusal` lacks `LimitNotApplicable` and `QueryKind`. The missing parts need FR-183's unbuilt `MatchWitness` check and replay, which is too big for this PR; no ticket tracks them (QSL-1103 covers only tagging). Needs a linked subticket before merge. | qsl-replay/src/model_check/hyper_settle.rs:300-305 |
| FND-003 | low | Doc comment of `settle_hyper_smt` has double-encoded UTF-8: `Î¼_U` for `μ_U`. | qsl-replay/src/model_check/hyper_settle.rs:291 |
| FND-004 | low | `HyperRecord.method` doc still says it is `EXPLICIT_STATE`; this PR makes it `K_INDUCTION` for SMT records. | qsl-replay/src/model_check/hyper_settle.rs:73 |
| FND-005 | low | The Fault arm is detected by `refusal.code().is_none()` instead of matching `SmtEncodingRefusal::Fault(_)`; and a subject whose `ModelSystem::new` fails settles `failed`, which the `# Errors` doc does not state. | qsl-replay/src/model_check/hyper_settle.rs:307,345 |

## Verdict

Not mergeable as is: fix FND-001 in this PR and file the FND-002 subticket. The built rows are sound. Dropping `Copy` from `HyperSettleRefusal` is required, because `SmtEncodingRefusal` holds `String`s, and it breaks no workspace use: `qsl-analyze` tests compile. CG depends on `qsl-replay` and `qsl-replay` does not depend on CG, so there is no cycle. CG can construct `HyperSmtOutcome` from `HyperQuery.product`.
