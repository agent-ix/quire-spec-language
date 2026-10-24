---
id: SR-9022
title: "Spec review of quire-spec-language PR #815: FR-360 QSpec citations and the criteria the code slice implements"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bddce547903993dcb29a7fe391d6a390a209a45b; spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md (cross-repo citation paragraph, items 1, 4, 7, 12, AC-3, AC-6), checked against FR-359, FR-056-AC-27..33, the code at this head, and the QSpec rows FR-322-AC-71, FR-141-AC-7, FR-141-AC-8, FR-208-AC-15, FR-208-AC-16, FR-208-AC-17, FR-144 enumeration key row, TC-939..TC-942 on agent-ix/quire-specification main"
review_set: subset
---
# Spec review of quire-spec-language PR #815

## Summary

Ticket: QSL-738 (QSL-748 AC2 depends on this citation). The PR's only spec edit is the paragraph after "Cross-repo requirements". It cites the merged QSpec rows by id. Each cited row was read on QSpec main to check that it says what FR-360 claims it says. The FR-360 criteria this code slice implements were then read against the code. Sub-analyses: base only. The edit adds no requirement statement, relationship edge or domain object, so ears, integrity and dependency do not apply. No QSpec text is quoted here. The public repo carries ids and paraphrase only.

Examined:
- the FR-360 citation paragraph (lines 243-253): "requirement 1: QSpec FR-322-AC-71 ... FR-141-AC-7 and FR-141-AC-8; requirement 2: QSpec FR-208-AC-16 ... FR-208-AC-17; the cross-repo test cases QSpec TC-939 to TC-942" (examined)
- QSpec FR-322-AC-71: the model-owned `variant_type` node, family `enum`, the step 4 member types, owner join, alias-independent key. It matches requirement 1. (examined)
- QSpec FR-141-AC-7 (member key over the variant node, alias-independent) and FR-141-AC-8 (any non-empty case name, distinct keys, UTF-8 byte Set order). Both match the requirement 1 identity half. (examined)
- QSpec FR-208-AC-15 (amended: a variant typeRef and a clause naming the variant type raise no refusal), FR-208-AC-16 (admission vectors), FR-208-AC-17 (case-literal typing and resolution, the refusals). (examined)
- QSpec TC-939 to TC-942 exist and cover admit, export key, name resolution, and non-identifier case names. (examined)
- FR-360 item 12 and its interim end condition (examined); FR-360-AC-3 (examined); FR-360 items 4 and 7 and AC-6 operator spelling (examined)
- FR-359 "An event type is a record value for this purpose" and the assembler's `EventType` arm (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-360-AC-3 sentence 2 contradicts FR-359 and the code. It says a field whose `typeRef` names an event type, state machine, process, persistence interface or namespace "keeps its refusal" `invalid_model_binding`/`malformed-declaration` at `Field.typeRef`. No such intake refusal exists for any of the five meanings, before or after this PR. An event-typed field is admitted and assembled as a composite, which FR-359 requires ("an event type is a record value for this purpose"; assemble.rs `RecordValueType \| EventType`). The other four reach assembly as `UnsupportedModelMember`. QSpec FR-208-AC-16, which this PR cites, repeats the same sentence. Fix: remove "event type" from FR-360-AC-3 (and from QSpec FR-208-AC-16 through QSL-748). For the remaining four, either state the assembly refusal that actually happens, or add the intake refusal at `Field.typeRef` (it mirrors `signature_type_refusal`) with a TC-1006 vector. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:287; qsl-semantics/src/check/assemble.rs:1205-1207; qsl-semantics/src/model/normalize.rs:2055-2070 |
| FND-002 | medium | The citations are mapped to the wrong cross-repo requirements. FR-208-AC-17 states case-literal resolution, which is cross-repo requirement 3, but it is cited under requirement 2. Requirement 3 gets no row. Requirement 2 also lifts the clause refusal, and that lift is in the amended FR-208-AC-15, which is not cited. Fix: requirement 2 cites FR-208-AC-15 and FR-208-AC-16; requirement 3 cites FR-208-AC-17 (and TC-941). | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:243-253 |
| FND-003 | medium | The paragraph says FR-360 "cites them by id rather than restating them", but the full restatement of requirements 1 to 3 stays directly above it, unchanged. QSL-748 AC2 asks for the citation "instead of restating". As written, the text contradicts itself and does not meet QSL-748 AC2, so QSL-748 should not close on it. Fix: replace each requirement's restated rule with its row ids and keep one sentence of intent each, or drop "rather than restating them". | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:222-253 |
| FND-004 | medium | Item 12's end condition is met on paper but not in fact. The interim emitter refusal lasts "until QSL-748 requirement 1 holds", and "this obligation ends when cross-repo requirement 1 holds, and emission then succeeds". Requirement 1 is a QSpec spec change, now merged, so read literally the interim has ended and emission must succeed. The code still refuses, correctly, because the IR reader and the QSV enumeration do not yet support the node; that work is tracked by QSL-752. Item 12 also names a ticket id twice in spec text. Fix: tie the end condition to the implemented reader ("until the IR reader admits a `model`/`variant_type` node and the runtime enumeration holds any non-empty case name"), with no ticket id. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:200-221 |
| FND-005 | low | A clause context, or an operation-body target, that names a variant type (`invariant .. on Config::Tier`, `body b on Config::Tier::size()`) now refuses at assembly as an unresolved type name (`missing_declaration`/`missing-name`), because a variant type is not an object type. FR-360 does not state this. Item 1 says only that the old clause refusal is removed, and AC-4 speaks of a clause naming the type "in a type position". Fix: add a sentence to item 4 giving this refusal and its cause, with a TC-1007 vector (the two TC-997 tests already pin it). | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:59-77; qsl-semantics/tests/it/variant_type.rs:552-571, 1130-1146 |
| FND-006 | low | Items 7 and 12 and AC-6 write the equality operator as `==`. QSL source spells it `=` (qsl-cst token `Equal => "="`). Two implementers could disagree about whether `==` is source syntax. Fix: write `=` where source is meant, or say once that `==` denotes the equality operator. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:130-141, 291 |

## Verdict

**BLOCKED** on FND-001 (high): a spec criterion that this PR claims to implement contradicts FR-359 and the code. The coder correctly invented no refusal. The fix is a spec edit in this PR, plus optionally the four-meaning intake refusal. The matching QSpec FR-208-AC-16 sentence goes to QSL-748.

Citation check, row by row. These read as claimed:
- FR-322-AC-71: model-owned node, `enum` family, step 4 types, alias-independent key.
- FR-141-AC-7: member key over the variant node, equal across aliases.
- FR-141-AC-8: any non-empty case name, byte-order Set.
- FR-208-AC-16: admission vectors.
- TC-939 to TC-942: they exist and match their titles.

FR-144's enumeration key row (case-name UTF-8 bytes for a `variant_type` node) is already cited in item 7's prose, which is enough. The mis-mapping is FND-002. The public-repo rule holds: the edit names `agent-ix/quire-specification` and row ids only. It quotes no QSpec text and has no local path, branch or commit.

Item 7's Set order (ascending UTF-8 bytes of case names) matches QSpec FR-144 and FR-141-AC-8. The code decides it once, at intake (`cases.sort_by` on `&str`), and both later consumers depend on that one order (SR-9020 FND-003). The new spec text adds no ticket id; the existing ones are FND-004.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | FR-360-AC-5, items 4 to 6, and TC-1007 step 3 still require check-stage `ill_typed`/`type-mismatch` refusals for a Text literal, `none`, and a `case` expression at a variant slot. At this ref none of the three has a form (qsl-forms `ExprNode`), so none ever reaches the checker: each is refused before check, at S1 or S2. The spec states an outcome the code cannot produce, and the trace tags claim it. Fix: state the actual refusal (stage, code and cause) for these spellings at this ref, or move the three rows to the requirement that adds the forms. Then key the evidence test (SR-9021 FND-006) to that statement. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:274 |
| FND-008 | low | AC-8 keeps the old end condition, "once cross-repo requirement 1 holds". Requirement 1 is now a citation of merged QSpec rows, so the condition is already met. Item 12 was fixed to "until the IR reader admits a `model`/`variant_type` node and the runtime enumeration holds any non-empty case name", and AC-8 was not. Fix: reuse item 12's condition in AC-8. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:277 |

## Dispositions

Round 1, reviewed at 101dbdf0809546eae22d2047c429a58f805ff612.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 101dbdf08 |
| FND-002 | fixed | 101dbdf08 |
| FND-003 | fixed | 101dbdf08 |
| FND-004 | fixed | 101dbdf08 |
| FND-005 | fixed | 101dbdf08 |
| FND-006 | fixed | 101dbdf08 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | FR-062 attributes 'a maximum fixed by the language is not a caller work budget' to the catalog's `stage_limit_exceeded` row. On QSpec main the sentence is on the `resource_exhausted` row only; the `stage_limit_exceeded` row instead says its limits are caller-configured FR-461 settings, distinct from the work-budget meter. QSpec's own review flagged the same misattribution and corrected it. Read with the new wording, FR-062 implies that stage limits are maxima fixed by the language, which is false. ADR-014 and FR-096 attribute the sentence to `resource_exhausted` and are fine. Fix: say the `stage_limit_exceeded` row gives stage limits as caller-configured settings, distinct from the `resource_exhausted` work-budget meter. | spec/functional/FR-062-implement-checked-family-contract.md:183-187 |

## Dispositions (round 2)

Round 2, reviewed at d764d9277feeebb94c79e96e4740c92e0c94df67.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-007 | fixed | d760c7210 |
| FND-008 | fixed | d760c7210 |
