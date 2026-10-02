---
id: SR-1150
title: "QSL-367 spec review of ADR-020, FR-135 to FR-148, TC-540 to TC-554 and US-018"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7b60ebd292368889998c7dda51afb4e697287e1e; git diff 404a5a88...HEAD; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md; spec/usecase/US-018-prove-that-a-detailed-model-implements-an-abstract-one.md; spec/functional/FR-135..FR-148; spec/test-cases/TC-540..TC-554; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
---

## Summary

Ticket: QSL-367, PR quire-spec-language#564, head 7b60ebd2, reviewed as
`git diff 404a5a88...HEAD` (the PR is stacked on the #562 base 404a5a88).
One file holds the base checklist plus the EARS, integrity and
scope-boundary checks, and the comparison with the QSpec counterpart
FR-375 to FR-380 on QSpec branch `spec/wave-b-q2-refine-reduce`.

`quire validate --scope .` over the 36 changed files exits 0 with no error
in any changed file. `tools/check-index-completeness.sh` exits 0. Every
FR-135 to FR-148 AC has an index row in spec/tests.md naming a TC, and each
TC procedure exercises behaviour (verdicts, refusals, computed values), not
structure.

**Rulings checked.** (1) Undefined to refuted: ADR-020 RM-7, RM-8, AX-3,
RE-5, RC-1, RC-2 and RU-6, FR-138, FR-140 Behavior, FR-141, FR-142, FR-144
and FR-145 settle an undefined mapping row, argument or history update
`refuted` with `UndefinedEvaluation{where, cause}`, and TC-554 covers it.
One leftover contradicts it (FND-001). (2) Refused or incomplete to
`MappingUndetermined`: stated in RM-7, AX-3, RE-4, FR-138, FR-140 and
FR-141. FR-140 also adds out-of-type field rows (FND-007), and the
refused/incomplete path has no AC (FND-009). (3) Abstraction stays
separate from refinement: §7 and MC-2 keep the code-side abstraction
relation (QSpec FR-353) a separate mechanism with separate keys. The
ADR-017 AR-1 amendment adds a second `Relation` declaration with its own
`CheckedRefinement`. This is correct.

**Other checks, clean.** No fixed caps: every bound is an FR-126
`ModelCheckLimits` or FR-120 `max_candidates` member with a published
default, and a stopped run names the limit and value. `max_depth` is
FR-126's search horizon and settles a `BoundReached` verdict, not a
`Stopped` limit kind. `Inductive{depth: 1}` is an induction depth, not a
limit. No pins, version checks, ledgers or provenance were added. The
protocol post-state digest in RC-1 is the successor selector. There are no
compat paths. Ticket ids appear only in References. Every FR traces to
US-018. QSpec semantics are cited rather than restated (FR-139 and FR-141
say the rules are QSpec's). The §8 worked examples check out by hand: the
lost-update prefix, the `any` variant failing at position 6, the coin
candidate sets and the `side = self.face` frame failure, the
`RegisterHistory` 4 product states, and the `Once` refutation at
position 4.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-139 says an `only` with no or several matching objects is undefined "and the mapping that contains it is undetermined (FR-140)". This contradicts the owner ruling (undefined refutes with `UndefinedEvaluation`), ADR-020 RM-8 and RE-5, QSpec FR-376, FR-140's own Behavior bullet, and FR-139's own Dependencies line. State that the containing row evaluates undefined and the refinement settles `refuted`, `UndefinedEvaluation` (FR-140, FR-142). | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md:78-80 |
| FND-002 | medium | ADR-020 RC-1 makes an undefined mapping a counterexample whose `kind` is `UndefinedEvaluation{where, cause}` "in place of a `RefinementFailure`", and QS-6 asks QSpec for "six kinds". FR-141 (`StepVerdict::Undefined`) and FR-142 ("and no `RefinementFailure`") follow it. QSpec FR-379, which owns the wire, makes `Undefined{position, row}` the seventh `refinement_failure` kind, and FR-379-AC-2 requires exactly one of seven. The two specs would emit different counterexample wire. Align RC-1, QS-6, FR-141, FR-142 and FR-145 with FR-379, or change FR-379. | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:291,548; spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md:117-123 |
| FND-003 | medium | FR-135 refuses a unit that does not select the infinite-trace profile with `unsupported_construct`/`expression-form` at the profile's span (also tested in FR-135-AC-2). QSpec FR-375 refuses `unknown_profile`/`wrong-selection-role` at the alias. QSL implements QSpec's refusal, so use QSpec's code, subcode and locus. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:96-98 |
| FND-004 | medium | FR-136 refuses a wrong step-row argument count with `invalid_model_binding`/`malformed-declaration`, and FR-136-AC-3 tests it. QSpec FR-375 refuses `ill_typed`/`operator-ineligible`. Use QSpec's code. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:124-126 |
| FND-005 | medium | FR-137 refuses an `assume` row naming an abstract operation, or an `ensure` row naming a concrete one, with `invalid_model_binding`/`malformed-declaration` (FR-137-AC-2). QSpec FR-375 and FR-375-AC-6 refuse it with `missing_declaration`/`missing-name`. Use QSpec's code. | spec/functional/FR-137-check-a-refinement-s-fairness-rows.md:57-63 |
| FND-006 | medium | FR-146 (following ADR-020 "Claim kinds") writes RE-2 records only with no hidden field and no history field, and writes `Step` records for the non-`any` rows of a declaration that has `any` rows. QSpec FR-377 Engines and FR-377-AC-7 route per-step obligations for a declaration "with no hidden field and no `any` row" and do not exclude history fields. History is required: with history fields `map` is not a function of `(s, s')`. QSL is right here, but the two specs disagree on both conditions. Make QSpec FR-377 carry the history exclusion and agree on whether an `any` row suppresses only its own record or every record. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:79-84 |
| FND-007 | medium | FR-140 settles a field row whose "value lies outside the abstract field's declared type" `MappingUndetermined`. ADR-020 RM-7 and RE-4, and QSpec FR-377 and FR-379, give out-of-type only for history updates. For field rows they say only refused or incomplete. FR-140 adds a settlement rule neither owner states. Either add it to RE-4 and QSpec FR-379's `mapping-undetermined` definition, or drop it from FR-140. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:95-98 |
| FND-008 | low | FR-140's Description says "A row that does not evaluate makes the mapping undetermined at that state". An undefined row refutes (Behavior, RE-5). Say "a row that is refused or incomplete". | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:34-35 |
| FND-009 | medium | Coverage gaps (error-path rule). No AC or TC exercises a refused or incomplete field row or step-row argument settling `MappingUndetermined` (FR-140 Behavior, FR-141 argument bullet). That is the second half of the owner ruling, and QS-9 (g) asks for it. No AC exercises an undefined step-row argument either. Only an out-of-type history update is tested. TC-554's description claims "a refused, incomplete or out-of-type update stays `MappingUndetermined`", but its procedure tests only out-of-type. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:120-122; spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:95-98; spec/test-cases/TC-554-an-undefined-mapping-refutes-a-refinement.md:21 |
| FND-010 | medium | ADR-027 is cited throughout but does not exist on this branch or on main. It is in the ADR-020 relationships and status, in the amended RM-5, RM-6, CO-4, MC-1, RC-1 and RC-2, and in the relationships and Dependencies of FR-136, FR-137, FR-141, FR-142, FR-143, FR-145, FR-147 and FR-148. It is added by open PR #575 (spec/396-protocol-ts). Merging #564 first leaves dangling `ix://` targets and items such as PR-1, PA-3, TS-1 and PX-2 with no definition. Merge #575 first, or together. | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:22-23,61-63 |
| FND-011 | medium | FR-147 introduces an abstract protocol side, `abstract <A>::<protocol>`, and `internal <A>::<protocol>::<node>` rows, with "spelling QSpec's, ADR-020 QS-1 and QS-8". ADR-020 RM-1 makes `<A>` a model alias. QS-1 lists neither form. QSpec FR-375's grammar has neither (abstract is an `ident`, and there is no `internal` row). No QS item asks QSpec for them. Add the abstract protocol side and `internal` rows to ADR-020 §1 and §10 and to the QSpec counterpart. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:49-51 |
| FND-012 | low | FR-145 refuses `stale_dependency`/`revision-mismatch` when "the payload's fairness set differs" from the recompiled node (FR-145-AC-4). FR-137 puts `F_C` and `F_A` in the node identity, and FR-145 already refuses a node-identity mismatch and reads both sets from the recompiled node. So the payload copy of the fairness sets is never read except to cross-check data the identity already binds. It fails the value test. Delete the payload copy, its refusal and the AC-4 case. | spec/functional/FR-145-replay-a-refinement-counterexample.md:72-75 |
| FND-013 | low | Status still reads "Draft for plan-lead review of the key decisions", although §11 records the rulings on every question. This is leftover pending wording. | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:59 |
| FND-014 | low | §11 says the plan lead ruled on "the draft's five open questions" but has six rows (RU-6 added). The RU-6 row has five cells in a four-column table, so its "Where" column renders the rationale ("ADR-018 RU-5 applies ...") and drops the item list. | spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:556,566 |

## Verdict

Not mergeable as it stands. FND-001 contradicts the owner's
undefined-refutes ruling in normative text. FND-002 to FND-007 and FND-011
are disagreements with the QSpec counterpart that QSL implements, or with
the ADR. FND-009 leaves the refused/incomplete half of the ruling untested.
FND-010 is a merge-order dependency on #575. Everything else is consistent.
The design is sound: one `check_step` for engine and replay, safety on
phase 1 and liveness through `F_A` only (the RU-2 ruling), history and
hidden fields kept out of the concrete model, RE-2 never refuting, and the
abstraction relation kept separate. The ADR-011, ADR-012 and ADR-017
amendments are each marked in place.
