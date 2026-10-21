---
id: SR-13610
title: "failure-domain review of the probabilistic spec follow-ups (PR #1124)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@8a1f1ba219d7c8344c06301b2758cfad1a03fa23; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md, spec/functional/FR-196-build-the-probabilistic-product.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: reviews
---
# SR-13610: failure-domain review of PR #1124 (QSL-573, QSL-568, QSL-571)

## Summary

Ticket: QSL-573 (also QSL-568). This review looks for unstated failure
modes and identity confusion in the new FR-186 `over` rule and in FR-196's
recorded build API: undefined-path evidence, binding identity, refusal
mapping and empty binding sets. Each was traced through the #1122 builder
(qsl-analyze/src/exact_probabilistic/build.rs, plan.rs) and
ModelSystem::instances.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new `UndefinedPath`, which FR-196 calls "the refutation's evidence", has no initial-state index and no `over` binding, and records the scheduler's choice at a several-post-state draw only as model state-key bytes (`resolved`). ADR-028 WS-2 requires `{initial_state, binding, ...}` and WS-1 requires the complete PostState ProductKey choice. FR-196's own canonical-key section (lines 193-198) requires the complete product key in undefined-evaluation evidence and says a model-state digest SHALL NOT replace it, yet the path carries `state_key: DigestRecord`. For a claim `over` two objects (FR-196-AC-10's world; the #1122 Ghost test, where the second binding is undefined at position 0), the evidence cannot say which binding is undefined, so FR-202 cannot re-evaluate the read. The #1122 Builder::undefined drops the node's binding in the same way. Fix: add `initial: u32` and `binding: Option<ModelInstance>` to `UndefinedPath`, and record each step's `ProductAction` and target key (or state that EN-5 derives them from the product and give the rule), so that FR-202's `WitnessPath` can be formed. | spec/functional/FR-196-build-the-probabilistic-product.md:123-146 |
| FND-002 | medium | FR-186 does not tie the `over` type to the subject's model, and the code checks only that it is a reference type, which is a spec/code mismatch. FR-186 requires only "an object type". Its own rationale is FR-125's binding to the objects of the subject's universes, and the #1116 doc comment on ProbabilisticCause::OverNotObjectType says "not an object type of the subject's model". The code (claim.rs Pass::over) checks only `ValueType::Reference(_)`. A reference type that no population of the subject admits passes S3, then ModelSystem::instances returns no instance, build_product gets zero bindings and returns `Built` with no initial state. No statement gives that claim a verdict, so it holds or fails vacuously depending on the solver. Fix (spec): require an object type that some population of the subject's model admits and refuse anything else `over-not-object-type`; or state the verdict for a claim with no binding. Code then follows the ruling. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:126-131 |
| FND-003 | medium | The recorded build entry returns `ProductRefusal` with nine members, but FR-196 maps only `Admission` and `NotMarkov` onto `check_exact_probabilistic`'s `ExactProbRefusal`. Nothing says what `NotSimulated`, `Expansion` (with or without a limit), `UndecidedSuccessor`, `PredicateRefused`, `Translation`, `Stopped(Limit)`, `Stopped(Cancelled)` and `Internal` become in `ExactOutcome` / `ExactProbRefusal` or in FR-203's settlement. `ExactOutcome::Stopped` takes an `ExactProbLimit` and a setting, and has no member for a cancellation or an expansion stop. Two implementers would settle a cancelled build differently: `incomplete`, `cancelled` as FR-192's map does, or `failed`. Fix: add a table in FR-196 or FR-203 from each `ProductRefusal` member to its outcome or refusal and FR-331 settlement, following FR-192's rows for expansion stops and cancellation. | spec/functional/FR-196-build-the-probabilistic-product.md:66-89 |

## Verdict

Not mergeable. FND-001 is high: the recorded `UndefinedPath` cannot carry
the binding and the initial state that FR-202's replay needs. The #1122
builder has the same gap, so the code lane should fix both together.
FND-002 is the answer to the "object type" question. It is a real
spec/code mismatch and needs a ruling: either refuse at S3, or specify the
empty-binding verdict. FND-003 needs a mapping table.
