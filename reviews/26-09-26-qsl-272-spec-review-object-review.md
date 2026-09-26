---
id: SR-644
title: "QSL-272 object review of the simulation engine's new types"
type: SpecReview
scope: "agent-ix/quire-spec-language@84a691bf8beb9df40aa945c48e1e05d7f5fb070b; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; qsl-eval/src/simulation/{explore,frontier,trace,sample,mod}.rs; qsl-foundation/src/digest.rs; qsl-foundation/src/selection.rs; qsl-semantics/src/family/requirements.rs; qsl-foundation/src/diagnostic.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
---

## Summary

Ticket: QSL-272. This review checks the domain objects FR-101 names, their
owners, and whether layer 5 (`qsl-eval`) can hold them under the ADR-011
X-8 dependency set.

These resolve to existing owners reachable from layer 5:

- `DigestDomain::SimulationStateKeyV1` (qsl-foundation/src/digest.rs:158)
- `DefinitionRef` (qsl-foundation/src/selection.rs:41)
- `classify_extent`, `UnboundedDomains` and `DomainKind`
  (qsl-semantics/src/family)
- `Limit`, `Limits`, `Stats`, `StopReason` and `ReplayError`'s three
  variants (qsl-eval)

Three items are named without an owner or a signature.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-101 names three items without saying who owns them or what their signature is. (1) The "simulation entry" that returns `RequiresBound` has no function name or signature. `classify_extent` also needs a `TypeEnvironment` and a `position_limit` (requirements.rs:215-218), which are not in FR-101's Inputs, and TC-455 step 4 says only "Submit a simulation request". (2) `RequiresBound` has no stated shape; it could be a type or an enum arm, and it may or may not wrap `UnboundedDomains`. (3) The `cause` of `Outcome::Cancelled` has no type. The existing `cancelled`/`caller-cancelled` carriers are `qsl_replay::proof_result::IncompleteCause::Cancelled` (layer 6, above `qsl-eval`) and `qsl_cst::diagnostic::…::CallerCancelled` (not a normal dependency of `qsl-eval`). Only F's `CatalogCode` can be reached under X-8. The failure: an implementer adds `qsl-replay` or `qsl-cst` to `qsl-eval`'s `[dependencies]` to reuse the cause, which breaks TC-390's exact set and ADR-011 layering, or uses a free `String`. Fix: name the entry and its inputs (including the type environment and limit), `RequiresBound`'s shape, and the cause type (for example F `CatalogCode::new("cancelled", "caller-cancelled")`). | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:48-66,108-120; spec/test-cases/TC-455-stopped-explorations-stay-incomplete-and-unbounded-requests-require-a-bound.md:21-23; qsl-semantics/src/family/requirements.rs:215-218; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:352 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@be1851894b7dbe9f47a606186f181900e5e6eb3b` (fix commit `be185189`, "QSL-272 spec: fix PR #457 review findings (SR-641 to SR-645)"). I re-checked each outcome against the spec at that head. I did not take any outcome from the commit message. Vectors re-run: the mixed-n vector (n=1 at steps 0 and 1, n=5 at step 2, seed 424242, trace 0) selects 4, and a counter that skips no-draw steps selects 0. The NaN digests for 7ff8000000000000 and 7ff8000000000001 are a3d5ecff68c7cfb60a687aa72b743a04e1dc8513e348b9c3f64393dd96f4bdec and 62c344cca9a4942b80644ca8527bc7ccced905f01bf67262a9bd5e824356a955, and they match TC-453 step 6.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed be185189 | FR-101 now gives the signatures of `explore_request` and `sample_request` (domains, `TypeEnvironment`, `position_limit`), plus `enum NotSimulated{RequiresBound, Extent(ClassifyFailure), GeneratorMismatch, EmptyInitial}` and `struct RequiresBound{domains: UnboundedDomains}`. The cause is `CatalogCode::new("cancelled", "caller-cancelled")`. Every type named can be reached under the X-8 set, and FR-101 states that `qsl-eval` gains no `qsl-replay` or `qsl-cst` dependency. |
