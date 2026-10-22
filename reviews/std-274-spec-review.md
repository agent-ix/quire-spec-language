---
id: SR-16010
title: "Spec review of quire-spec-language PR #1261: STD-274, a role on a model alias"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ccdcecc7e8c6005ce3673f3594d651a1b935351a; PR #1261 (git diff origin/main...ccdcecc7e): spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md, spec/test-cases/TC-1061-s4-emits-each-protocol-construct-as-its-v2-protocol-node.md, spec/test-cases/TC-1065-a-channel-s-loss-selector-and-the-admission-rules-emission-needs.md; read for context: FR-205-AC-1, TC-650, FR-218-AC-28, FR-114, FR-383-AC-20, ADR-027 PS-1 and §7, agent-ix/quire-specification main 5ee0071fc FR-425, FR-465 (Role), FR-322 node forms, TC-376"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1061
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1065
    type: reviews
---
# SR-16010: Spec review of quire-spec-language PR #1261: STD-274

## Summary

Ticket: STD-274. Sub-analyses folded in: integrity, EARS, failure-domain, object review. The head ccdcecc7e is behind main 423ac6aa0 but merges cleanly. `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` on the merge result exits 0 with no diagnostic on the five touched files. FR-383-AC-22 appears on no other open PR head and not on main. #1261 and #1258 both edit FR-218 and merge with each other cleanly. No status wording and no compat layer.

The S3 admission half matches the ticket's lead default. Two parts conflict with requirements this PR does not change. The emission half names a node that the IR does not have and that QSpec's reader refuses (FND-001). The binding half contradicts FR-205-AC-1 and TC-650 (FND-002). The stated QSpec follow-up, an FR-425 sentence, is also incomplete: QSpec FR-465 and FR-322 need changing first.

## Verdict

**NOT MERGEABLE.** There are two major findings, and FND-001 needs a QSpec decision before QSL can specify the emission. That decision is the owner's or planner's. The S3 half (FR-218 and the TC-1065 row 8 refusals) could land on its own once FND-004 is fixed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [major] AC-22 and the FR-383 `role` row emit "that alias's `model` node" as the role's model, but no such node exists. QSpec FR-322's `model` forms are `object_type`, `record_value_type`, `event_type`, `variant_type`, `state_machine`, `process`, `persistence_interface` and `systems_interface`, and none of them stands for a model alias. QSpec FR-465 (Role, reader step 1) requires the role's `model` target to be a `model` node of form `object_type`, `process`, `state_machine`, `systems_interface` or `persistence_interface`, and otherwise refuses `invalid_model_binding`/`malformed-declaration` at the target. FR-383-AC-20's "a model of the wrong kind" mutation makes QSL's I2 reader enforce that rule. So "the IR v2 reader admits the package" cannot hold. QSpec first has to choose a representation for the role's model target, such as a new model-alias node form or a role layout whose `model` is absent or names the subject, and widen FR-465 step 1 to admit it. The PR's "Ungrounded (QSpec)" note names only FR-425, so it understates this. Planner: this needs a QSpec spec cycle, and the QSL emission rule should wait for it. | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md:101,294; spec/test-cases/TC-1061-s4-emits-each-protocol-construct-as-its-v2-protocol-node.md:55 |
| FND-002 | high | [major] The new FR-205 rule contradicts FR-205-AC-1, which this PR leaves unchanged. FR-205-AC-1 builds ADR-027 §7's `Fill`, whose role is `role w on M` (ADR-027 lines 322 and 418), with "`w` bound to the model", and says "leaving `w` unbound refuses `invalid_runtime_input`/`wrong-role-mapping`". TC-650 step 3 tests that ("an unbound `w`"). Under this PR, `w` is a static role on a model alias, so it "needs no request binding", and `wrong-role-mapping` applies only to roles on other models, so an unbound `w` now builds. Update FR-205-AC-1 and TC-650 so the unbound-role refusal uses a role on an object type. Also state what happens when a request does supply a binding for a model-alias role: is it ignored, required to name the subject model, or refused? | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:81-85,119; spec/test-cases/TC-650 step 3 |
| FND-003 | medium | [minor] FR-205 says such a role "SHALL be bound to the model subject itself (QSpec FR-425's static role binding)". QSpec FR-425 states only "a binding of each static role" (Inputs) and "the static role bindings" (Protocol subject). It does not say that a role on a model alias binds to the model subject, as the PR body itself notes. The parenthesis reads as grounding a rule that QSpec does not have. Drop the citation or reword it as the QSL rule that QSpec FR-425 is to mirror. The FR-205 change also has no AC of its own: neither the subject built with no binding for `w` nor the case of a supplied binding is tested. | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:81-82 |
| FND-004 | medium | [minor] The replicated-role case is not a well-formed fixture. A replicated role is written `role r each T from pop max n lifetime L` (FR-218-AC-28, QSpec FR-465), not `on`, so "a replicated role `on M`" has to be `role w each M from <pop> ...`. Its `from` population is either a population of some type of `M` or nothing. FR-218-AC-28 already refuses a population whose type is not the role's model type (`ill_typed`/`type-mismatch` at the population). The AC needs to say which population the fixture uses and that the model-name refusal comes before the population check. Otherwise two refusals are possible, at `M` or at the population. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:164-170,442; spec/test-cases/TC-1065-a-channel-s-loss-selector-and-the-admission-rules-emission-needs.md:41 |
| FND-005 | medium | [minor] The rule does not say which model alias a role may name or which model subject it acts on. A protocol subject has one model subject (FR-205, ADR-027 PS-1). With two aliases `M` and `N` and `over (k: M::Cell)`, what does `role w on N` act on? It cannot be "the model subject itself" if the subject is `M`'s. The AC refuses an attempt by `w` on `N::Device::tick` only because `w` is on `M`. State that the alias must be the subject's model, or the alias of the `over` type, and add the refusal for `role w on N`. Planner note: FR-114 puts no matching limit on an object-type role, so a role `on M::Cell` attempting `N::Device::tick` has no refusal. The alias-only refusal is therefore asymmetric, which is acceptable only if it is intended. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:164-170; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:81-82 |
