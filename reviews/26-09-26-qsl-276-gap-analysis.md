---
id: SR-723
title: "S2 state clause forms and model operations delivery gaps"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@49b3398ab012c0635fc1b2306d536312e319de11; FR-102-AC-1..6; FR-103-AC-1..5; TC-456; TC-457; TC-458; qsl-forms/src/protocol_clause.rs; qsl-forms/src/dispatch.rs; qsl-forms/src/value.rs; qsl-forms/tests/it/protocol_clause_forms.rs; qsl-forms/tests/it/value_forms.rs; qsl-semantics/src/model/intake.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/value/declaration.rs; qsl-semantics/tests/it/model_operations.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: references
---
## Summary

Ticket: QSL-276. PR: quire-spec-language#486 at 49b3398a. This review checks
each FR-102 and FR-103 AC against the tagged tests and the code, and settles
the coder's four spec questions.

Delivered and traced:

- FR-102-AC-1/2/3: TC-456, three tests in `protocol_clause_forms.rs` plus cases in `value_forms.rs`. The AC-1 span coverage is partial (SR-722 FND-007).
- FR-102-AC-4: `no_dispatch_entry_refuses_...` moved to `temporal` (TC-457). The thin-call check covers the new arms but is not tagged (FND-005).
- FR-102-AC-5: TC-457. The depth `d` build and the `d + 1` limit are both asserted.
- FR-103-AC-2 relationship case, and the AC-4 record value type: TC-458.

The coder's spec questions:

1. The `value-type/v1` reader: **(a) deferred, with no owner yet.** The coder's claim that "no AC asks for it" is wrong. FR-103-AC-1 and AC-3 both name `VersionNumber` bound to `Int[0, 1000]`. The reader itself belongs to FR-056, whose export table requires "one `scalar`" for `value-type/v1`. The wire shape really is unsettled: FCD's own fixtures at 033e228 bind `value-type/v1` to record-shaped constructs (`value_object`, `event`). No Linear ticket owns it. FND-001.
2. The FR-108 fixture is owned by QSL-279: **(a) deferred to QSL-279.** That is correct: QSL-279's scope is the FR-108 corpus, and it comes after this ticket in the chain. FND-002.
3. The FCD validator pre-empts `missing_declaration`: **(b) a finding in this PR.** The FCD-to-refusal mapping is QSL code (`validate_with_semantic_ir`). See SR-722 FND-003.
4. Native `Text` and the intake-versus-assembly stage: **split verdict.** The mismatch between `Text` and `String` is **(a)**, a cross-repo vocabulary gap between FCD's `NATIVE_SCALARS` and QSL's `NativeValueType`, and needs a ticket (FND-004). On the stage, **(c)**: the coder's claim is right and the spec is wrong. FR-103 says the refusal is made "as a field of that type does", and a field typed `Decimal` or `Text` refuses at I1 with `unsupported_construct`/`declaration-form`. So AC-3's code, cause and stage contradict the rule it cites. Amend the spec in this PR (FND-003).

## Verdict

**Changes requested.** One medium spec-to-test gap needs a spec amendment in
this PR, and one medium needs a follow-up ticket. The rest are low or
deferred.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-103-AC-1 (`versionNumber: Int[0, 1000]`) and AC-3's second half (`delta: VersionNumber` typed `Int[0, 1000]`) are not verified. The tests type both as native `Integer` and still carry `#[trace("TC-458", "FR-103-AC-1")]`/`"FR-103-AC-3"`, so the trace claims coverage that the assertions do not give. The cause is real: `read_type_node` folds `quire.meaning.model.value-type/v1` into the known-but-unsupported arm, and FCD's fixtures do not settle a scalar wire shape. Fix in this PR: record in TC-458 and the test docs that the bound-scalar half of AC-1/AC-3 is unverified and pending the reader. Follow-up: a ticket for FR-056's `value-type/v1` scalar reader. None exists; the lead should file it. | qsl-semantics/src/model/intake.rs:2058-2063; qsl-semantics/tests/it/model_operations.rs:381-457; qsl-semantics/tests/it/model_operations.rs:694-736 |
| FND-002 | low | TC-458 names `examples/config-version/model.semantic-ir.json` (FR-108) as its fixture, and it does not exist. The tests use a hand-built document. Deferred to QSL-279, which owns FR-108: when that fixture lands, QSL-279 should repoint the TC-458 tests at it. | qsl-semantics/tests/it/model_operations.rs:11-32 |
| FND-003 | medium | FR-103-AC-3 and TC-458 step 3 expect `note: Text` to refuse `unknown_required_feature`/`unsupported-feature` at the `model` declaration (assembly). FR-103's own Behavior says the refusal is made "as a field of that type does", and a field of `Text`/`Decimal`/`Rational` refuses at I1 through `unsupported_native_parameters`. `a_decimal_typed_parameter_refuses_at_intake_not_assembly` knowingly pins the I1 refusal against the AC's wording. The spec is the defect, not the code. Fix in this PR: amend FR-103-AC-3 and TC-458 step 3 to the I1 refusal (`unsupported_construct`/`declaration-form` at the parameter) with a type both vocabularies accept (`Decimal`). If an assembly-stage case is still wanted, name a type that passes I1 but that the environment cannot hold. | spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md; spec/test-cases/TC-458-spine-admits-model-operations-and-frames.md; qsl-semantics/tests/it/model_operations.rs:636-692 |
| FND-004 | low | FCD's native scalar vocabulary (`String`) and QSL's `NativeValueType` (`Text`) disagree, so no `ix://quire/native/Text` or `.../String` typeRef can admit through the pipeline. The gap is cross-repo (FCD's vocabulary versus FR-056's native rule) and outside this PR. Deferred: it needs a ticket, and none exists. | qsl-semantics/src/model/domain_package.rs:62-88 |
| FND-005 | low | FR-102-AC-6 (TC-457 step 4) has no traced test. Its wording, that the S2 seam list "now includes the `protocol_clause` production entry", cannot hold by design: `state_clause` has no match over a probed enum, so no new E0004 location arises, and the PR says the seam probe passes unchanged. `dispatch_entry_is_a_single_thin_call` also lacks a `TC-457`/`FR-102-AC-4` tag. Fix: amend FR-102-AC-6 and TC-457 step 4 to "the checked-in list is unchanged", and tag the thin-call test. | spec/functional/FR-102-build-state-clause-forms.md; spec/test-cases/TC-457-s2-state-clause-dispatch-limits-and-seam.md; qsl-forms/src/dispatch.rs:630-631 |
| FND-006 | low | TC-457 step 3 prescribes `d` nested parentheses, but a bare `(e)` is transparent and adds no depth. The test correctly uses `not` instead. Fix: amend TC-457 step 3 to nested `not`. | spec/test-cases/TC-457-s2-state-clause-dispatch-limits-and-seam.md; qsl-forms/tests/it/protocol_clause_forms.rs:199-229 |
| FND-007 | low | FR-103 Behavior says that "the operation is visible on a subtype through FR-081's effective view, as a field is". `ObjectTypeDeclaration::operations` returns only the type's own operations, and no effective-view code or test covers inherited operations. No AC names this, and nothing reads operations yet. Deferred to QSL-277 (FR-104 is the first reader of `M::T::op`) unless it is added here. | qsl-semantics/src/value/declaration.rs:485-490 |

## Dispositions

Round 2, reviewed at 960490173a2601bf8fd9e76b9ccf344242b18002.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3b94b129: FR-103-AC-1/AC-3 and TC-458 mark the bound-scalar half **Unverified**, citing QSL-289 (Backlog, "FR-056: read quire.meaning.model.value-type/v1 bound scalars"). The test module doc cites QSL-289. |
| FND-002 | deferred | QSL-279 owns the FR-108 fixture, and should repoint the TC-458 tests at it when it lands. |
| FND-003 | fixed | 3b94b129: FR-103-AC-3 and TC-458 step 3 are amended to `note: Decimal` refusing `unsupported_construct`/`declaration-form` at the parameter, at I1. The test matches. |
| FND-004 | deferred | QSL-290 (Backlog): `ix://quire/native/Text` has no FCD name. |
| FND-005 | still-open | FR-102-AC-6 and TC-457 step 4 are unchanged at 96049017, and `dispatch_entry_is_a_single_thin_call` (qsl-forms/src/dispatch.rs:630-631) is still untagged. |
| FND-006 | still-open | TC-457 step 3 still prescribes nested parentheses at 96049017. |
| FND-007 | deferred | QSL-277 (FR-104 is the first reader of operations through the effective view). |

## Round 3 findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The amended TC-457 step 3 is off by one against its own expected result and the test. The procedure builds "`d` nested `not`s around `true`, then `d + 1`", and the expected result says "depth `d` builds". But the root counts at depth 1, so `d` nots plus the leaf is depth `d + 1`, which refuses. The test builds `d - 1` nots, which pass, and `d` nots, which refuse (protocol_clause_forms.rs:261-264). Fix: "`d - 1` nested `not`s around `true` (depth `d`), then `d`". | spec/test-cases/TC-457-s2-state-clause-dispatch-limits-and-seam.md:25; qsl-forms/tests/it/protocol_clause_forms.rs:261-264 |

## Dispositions (round 3)

Round 3, reviewed at fe9224fcffc69700dd7467cabab243e76206f399.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3b94b129 (bef8bb35): Unverified, QSL-289 |
| FND-002 | deferred | QSL-279 |
| FND-003 | fixed | 3b94b129 (bef8bb35) |
| FND-004 | deferred | QSL-290 |
| FND-005 | fixed | fe9224fc: FR-102-AC-6 and TC-457 step 4 now say the list is "unchanged", and `dispatch_entry_is_a_single_thin_call` is tagged `TC-457`/`FR-102-AC-4` (dispatch.rs:630) |
| FND-006 | fixed | fe9224fc: TC-457 step 3 now uses nested `not`s. It introduced the off-by-one in FND-008. |
| FND-007 | deferred | QSL-277 |
| FND-008 | still-open | new in round 3, low |
