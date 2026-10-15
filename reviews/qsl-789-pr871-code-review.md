---
id: SR-11268
title: "Code review — QSL-789 relationship-end navigation typing and lowering (PR 871)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f80c8544db29f10047636e047510c61179c765e7; qsl-semantics/src/check/{navigation,state_model,check,assemble,refusal,ir,lowering,observation,claims,mod}.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/model/{domain_package,intake,systems}.rs, qsl-eval/src/value/expression/{causes,evaluate}.rs, qsl-semantics/tests/it/relationship_navigation.rs, qsl-semantics/tests/it/tc235_record_event.rs, qsl-package/src/emit/tests/navigation.rs"
review_set: subset
---

## Summary

Ticket: QSL-789. Code review with the rust-review lane over the 27-file diff of PR 871 (`git diff origin/main...HEAD`). It checks the implementation against FR-363 AC-1 to AC-6, TC-1031 N01 to N12, TC-1032, FR-300, FR-093 and FR-094, and the quire-contract-ir v2 `check_relationship_member` and `navigation_type` on IR main. No build was run: this is a read-only review. The coder's logs show clippy `-D warnings` and the focused all-features runs exiting 0 on this head.

## Verdict

**PASS. No blocking findings.** Resolution follows the near-end rule: `Relationships::navigable` indexes each end by its own role and keeps an end only when the receiver conforms to that end's type. The destination is the other end. The checks run in the stated order: the receiver must be a Reference (refused at the operand), then the name (none = type-mismatch at the form; more than one = ambiguous-name naming every candidate), then direction (operator-ineligible), then the bound (Undeclared, Unbounded or Ordered = expression-form). `deref(r).name` leaves ends out and refuses at the form. The result-type table matches FR-363 and IR's `navigation_type`. Lowering emits one `query` application of `quire.op.model.navigate` with a `relationship_end` member and one reference argument. IR's checks agree with it: role lookup, receiver conformance, direction and canonical result.

The direction match is written out in full, with no wildcard. Every NodeKind match touched names `Navigate`: children, claims, observation, lowering and the qsl-eval `apply` (`deny(wildcard_enum_match_arm)`). The remaining `_` arms (facts stable path, `reads_witnessed`, `dereferences`) correctly leave a navigation out. The new CheckCause variants are named in `code()` and `cause()`. No unwrap or panic was added outside tests.

Test strength: N01 to N07, N09, N10 and N11 each fail under a plausible wrong implementation: typing removed, far-end naming, direction skipped, bound skipped, direction after bound, or the first of several candidates picked. N08 passing with typing removed is legitimate. The unknown-name refusal is meant to be unchanged, so N08 guards against regression rather than detecting a mutant. N12 part 1 fails if `deref` admits ends. For N12 part 2, see FND-003. The TC-235 M21 run 2 assertion fails without the typing. The TC-1032 test checks only the form of each result_type, but IR's reader, which must return Verified, checks the canonical type. No existing test was weakened: the edits to existing tests only wrap `Multiplicity` in `Some` and add an `upper` to the M21 fixture. Trace tags do not overclaim. The N tests sit behind `test-support`, and both the CI and `make ci` all-features lanes run them.

Recorded and not blocking: FND-001 (medium) and FND-002 and FND-003 (low).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `Frame::Field` guard changed from "state clause and Reference" to "not Composite". Every non-record `x.f` in every context (Int, Option of a record, Tuple, Set) is now refused by the StateModel hook: the refusal is attributed to StateModel and sits at the operand, where before it was a Value-family type-mismatch at the form. A bare `r.f` over a Reference in a function body is now StateModel-wrapped, with the same code, cause and location. The PR says such reads "stay refused as before", which is not true of their attribution. No test pins either behaviour, and FR-363 states the operand rule only for model receivers | qsl-semantics/src/check/check/typing.rs:1547, qsl-semantics/src/check/state_model.rs:220-236 |
| FND-002 | low | `check_connection` now skips the narrowing check for an end with no multiplicity. Intake never reaches that case (`multiplicity_of` refuses an absent multiplicity as malformed), and no test covers it. The change from `Multiplicity` to `Option` stays inside this repo: there is no serde, no schema and no consumer outside it | qsl-semantics/src/model/systems.rs:660-663 |
| FND-003 | low | N12 part 2 (`self.order.order`) also passes with typing removed, because the inner unknown name refuses type-mismatch at the same path [0,0]. That is acceptable, because N01 pins the typing and the path check catches a form-located or Option-lifting implementation. A fixture such as `self.order.lines` would make the vector prove the rule on its own | qsl-semantics/tests/it/relationship_navigation.rs:716-724 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | `is_model_receiver` sends an Option, collection or population of references to the hook only inside a state clause. In a function body, `o.order` over `o: Option<Reference<M::Fulfilment>>` therefore keeps the Value-family type-mismatch at the form. FR-363's non-Reference receiver rule (StateModel, at the operand) states no context limit, but every TC-1031 vector is an invariant. Recorded, not blocking | qsl-semantics/src/check/check.rs:2331-2346 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b6338e48c12633c2d44a8d82a897b338aecc650b |
| FND-002 | accepted-no-change | A recorded low under the review cap. The skip cannot be reached from intake, and the Option change stays inside this repo |
| FND-003 | fixed | b6338e48c12633c2d44a8d82a897b338aecc650b |
