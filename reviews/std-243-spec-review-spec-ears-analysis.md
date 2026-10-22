---
id: SR-13971
title: 'Spec review of PRs #1187 and #328: spawn/retire instance member, pop resolution, send/receive owner, predicate reads (STD-243), EARS analysis'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@949dcb17f; git diff origin/main...949dcb17f (PR #1187, branch std-protocol-243): spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md, spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md; agent-ix/quire-specification@2ad2f256 (PR #328): spec/functional/protocol/FR-426-apply-protocol-steps.md, spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md; ticket STD-243'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-208
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-216
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
---

## Summary

Ticket: STD-243. PRs: quire-spec-language#1187 (head 949dcb17f) and quire-specification#328 (head 2ad2f256), both based on main. Method: spec-review with the EARS sub-analysis of every new or edited requirement statement. `make check-spec-validation` reports no EARS warning on a touched file. The findings below are ones the linter does not flag. The QSpec FR-426 and FR-428 changes are a table row and descriptive prose in QSpec's house style, which has no SHALL statements, so the EARS patterns do not apply to them. They were read only for consistency (SR-13970).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-208 spawn/retire identity statement | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:65-68 | examined |
| FR-208 send/receive owner statement | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:69-72 | examined |
| FR-209 Dependencies bullet (identity) | spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md:131-133 | examined |
| FR-216 predicate-read statement | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:100-108 | examined |
| FR-216 activate enabling footprint clause | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:121-127 | examined |
| FR-218 scope region statement | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:359-362 | examined |
| FR-218 pop resolution statement | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:363-368 | examined |
| FR-218 Outputs owner entry | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:89 | examined |
| QSpec FR-428 replicated-roles prose | quire-specification spec/functional/protocol/FR-428-check-replicated-roles-and-per-trigger-activation.md:59-80 | context_only |
| QSpec FR-426 identity table instance row | quire-specification spec/functional/protocol/FR-426-apply-protocol-steps.md:197 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-208's new send/receive bullet joins two requirements in one statement, with two SHALLs and a condition in the middle ("A `send` node SHALL be owned by ... and when that role is replicated the protocol system SHALL give the node once per live instance"). Split it into a ubiquitous ownership statement and a state-driven one ("While the owning role is replicated, the protocol system SHALL give a `send` or `receive` once per live instance ..."). | spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:69-72 |
| FND-002 | low | FR-218's scope-region bullet now adds a sentence with no SHALL inside a SHALL statement: "A `send` is owned by its channel's `from` role and a `receive` by its channel's `to` role, and the region counts those nodes as `by` that role." This is a requirement on the computed region, so state it as one: "S3 SHALL count a `send` as a node `by` its channel's `from` role and a `receive` as one `by` its channel's `to` role." | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:360-362 |
| FND-003 | low | FR-216's predicate-read statement puts its qualifier after the location clause ("for each field read `x.f` in `p`, the location (`o`, `f`) of every object ... whose type declares `f`, for a model-state field read (...)"). At first it reads as if every field read contributes model locations. ADR-027 FT-1's order ("for each model-state field read `x.f` in `p` ...") is clearer. Lead with "for each model-state field read". | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:102-107 |

## Verdict

Mergeable on EARS grounds. All three findings are low wording issues and none changes behaviour. The other new statements are singular, name their subject (the protocol system, S3), and can be tested.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@5c5adee7a` (fix commit `5c5adee7a`). Each outcome was checked against the spec text at that head, not taken from the commit message. `make check-spec-validation` reports no EARS warning on a touched file.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed 5c5adee7a | FR-208 after: "A `send` node SHALL be owned by the `from` role of its channel and a `receive` node by the `to` role of its channel." and, as a separate statement, "While the role that owns a `send` or `receive` node is replicated, the protocol system SHALL give the node once per live instance, naming the acting instance in its transition identity." |
| FND-002 | fixed 5c5adee7a | FR-218 after: "For each replicated role with lifetime `scope`, S3 SHALL record the smallest control holding every node `by` that role, counting a `send` as a node `by` its channel's `from` role and a `receive` as a node `by` its channel's `to` role." The sentence with no SHALL is gone. |
| FND-003 | fixed 5c5adee7a | FR-216 after: "for each model-state field read `x.f` in `p`, the location (`o`, `f`) of every object `o` in the subject's universe whose type declares `f` (a field read of a record binder reads no model location, ...)". The qualifier now leads, as in FT-1. |
