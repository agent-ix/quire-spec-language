---
id: SR-807
title: "EARS conformance review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: ears-conformance
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-807: EARS conformance review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted draft of ADR-017 on `spec/16-arch43-mapping` and its
amendment pointers in ADR-011 §1, ADR-012 §3 and §13.5, ADR-013 R-06, O-08
and O-25, and `spec/spec.md`. The engine check (`quire validate --summary`)
reports the ADR grammar-clean with 0 findings, because an ADR carries no
`shall` statements. So this review applies EARS by hand. It reads each
normative statement that #191, #192, #198 and TK-1 to TK-4 will turn into FR
statements. It judges them by the repo's plain-declarative ADR style (ADR-014,
ADR-015) and by the If/then refusal pattern of FR-109, FR-115 and FR-116.

What holds:

- Most decisions are precise declaratives that map to one EARS statement
  each. Examples: PF-2 package authority, RF-1, RF-3's case identity, RF-4's
  class table, AR-3's revision rule and AR-5.
- Most refusals name a catalog cause and what they report, and the three
  amended pointers match the decisions they cite.

What does not hold:

- Two findings are high. The AR-2 anchor key is not unique per bound element.
  The RF-2 reference outcome has no stated capture event, so #192 cannot
  build it.
- The medium findings are unwanted-behaviour cases with no If/then response,
  overlapping decision-table rows, and undefined terms that the FRs would
  inherit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The AR-2 key for FR-012 pre/post/result anchors is (`anchor` `NodeKey`, role) "for each bound parameter, result and framed field". Two parameters, or two framed fields, of one operation share the same (anchor, `Pre`) key. So AR-3's "one binding per key" makes a legal relation refuse `conflicting-binding`, and a #198 FR written from AR-2 would build a non-unique key. Fix: add the bound element to the key, for example (`anchor`, role, `AnchorSlot { Parameter(Identifier) \| Result \| Field(DeclarationKey, Identifier) }`), and say which ADR-013 owner keys the slot. | ADR-017 AR-2, AR-3; QSpec FR-353 |
| FND-002 | high | RF-2 says the reference outcome is "captured under the reference side and stored with the case", but no statement gives the event or the actor that captures it. RF-2 also says no build can compile the parent profile, because E3 fixes the non-`root` `DefinitionLock` rows. So #192 cannot state a `When …` requirement for producing a parent-refused record, and RF-7's "a new AD-003 edge adds a #192 pair" has no producer either. Fix: state the capture rule once. Either the recorded outcome is authored by hand as expected-result data when the case is added, or it is written by a named gate mode under a named build, for example "when `xtask refinement --record` runs on the build that selects edition E". Then say how a parent-profile outcome is obtained when no build can select that profile. | ADR-017 RF-2, RF-3, RF-7; QSpec AD-003 |
| FND-003 | medium | The RF-5 tables have overlapping rows and a first-match rule only for the gate verdict, not for the per-case result. For #191, R = not admitted with C = tool failure matches both "not applicable" and "tool failure". For #192, R = admitted, unsupported or incomplete with C = tool failure matches two rows as well. The FRs would inherit that ambiguity. Fix: state that the per-case rows are also first match, with the `any \| tool failure` row first. Also state whether a recorded reference class can be tool failure, or refuse such a record when the case is read. | ADR-017 RF-5 |
| FND-004 | medium | RF-4 does not decide a `StageFailure::Refused` with an empty cause list. The "every cause is `unsupported_construct`" row matches it vacuously, and the "at least one cause" row does not, so an empty refusal classifies as unsupported. RF-4 also says it classifies "by type", yet it splits refused from unsupported by `CatalogCode` value. Fix: add a row, for example "an empty cause list is tool failure (internal failure)". Reword the lead to "by result type and `CatalogCode` value, never by message text". | ADR-017 RF-4; ADR-011 FB-02 |
| FND-005 | medium | PF-4 and PF-3 give the E9 comparison with no complete trigger or response per selection kind. PF-4 says `replay_frame` "compares the recompiled frame node and occurrence with the envelope's members, and the anchor with the payload's", but it gives no response on mismatch and no order relative to the `package_id` check. PF-3 says "the resolved identities" are compared and a mismatch refuses "naming both", without saying which identities apply to a `Function` selection. No statement covers an envelope whose `ReplaySelection` variant disagrees with its payload type, for example `Function` over a `FrameCounterexample`. Fix: write one If/then per member, in FR-116 style. For example: "If the recompiled frame node differs from `clause_node`, then replay refuses `stale_dependency`/`revision-mismatch` naming both, before admission." Name the compared members for each variant, and add "If the selection variant does not match the payload type, then replay refuses …" with a catalog cause. | ADR-017 PF-3, PF-4; FR-116; ADR-013 O-25, O-26 |
| FND-006 | medium | TK-4 (G-4) "refuses a field the type does not declare at admission" but names no catalog cause, no stage and no report content. Every other refusal in the ADR names these. Fix: name the cause, for example `invalid_model_binding`/`missing-name` or the FR-106 reader's existing cause, the stage (`admit`), and the reported fields (declaring type `DeclarationKey`, field name). | ADR-017 PF-5 G-4, §6 TK-4; FR-106 |
| FND-007 | medium | PF-1 says "the three post-check name lookups that exist are entry selections". PF-3 lists four entries (FR-100, FR-109, FR-115, FR-116). PF-3 also says "no other post-check code resolves a name", yet TK-4 looks up a member by field name after check, and TK-1 looks up by operation `Identifier`. A drift gate built from PF-3 would flag both. Fix: correct the count. State that a member name inside the O-06 pair and an operation `Identifier` under a resolved `DeclarationKey` are declared keys, not names under R-06. | ADR-017 PF-1, PF-3, G-1, G-4; ADR-013 R-06, O-06 |
| FND-008 | medium | Two keys are given for one operation selection. PF-3 says a frame is selected by `OperationName` (`QualifiedName` plus `Identifier`). G-1 and TK-1 key the lookup by (`DomainPackageRef`, `DeclarationKey`, `Identifier`). No statement gives the step that converts one into the other, or its refusal when the alias or the object type does not resolve. Fix: state it as one rule. "The entry resolves `OperationName`'s model alias to its `DomainPackageRef` and its object path to a `DeclarationKey`. If either fails, it refuses `missing_declaration`/`missing-name`. It then looks up the operation by `Identifier`." | ADR-017 PF-3, G-1, TK-1; FR-115 |
| FND-009 | medium | AR-4 "Referenced elements" is not testable as written. "The object type of each model read in its claim" leaves "model read" undefined, and ", for a frame, precondition or postcondition record, its operation frame" does not parse. So #198 cannot derive which bindings an item needs. Fix: define referenced elements from checked data. For example: "the declaring `object_type` node of every model member in the record's checked node dependencies; the population of every extent domain (ADR-012 §15.7); and, when the record's node is a frame, precondition or postcondition, the `FrameBindingKey` of its operation." | ADR-017 AR-4 |
| FND-010 | medium | The AR-4 export states no refusal for a requested occurrence key that names no requirement record in the package. "No partial relation is emitted" can be read as whole-request failure, which contradicts "other items continue". Fix: add "If a requested key names no requirement record, then that item refuses `missing_declaration`/`missing-name`, naming the key." Replace "no partial relation" with "a refused item returns no bindings; a bound item returns every binding it references". | ADR-017 AR-4 |
| FND-011 | medium | AR-3 has no refusal for a `FrameBindingKey` whose parts disagree, for example a `frame` or `anchor` `NodeKey` that is not the named operation's `CheckedOperationFrame`. Only "model key resolves to no admitted declaration" is covered. Fix: add "If a `FrameBindingKey`'s frame or anchor is not the `CheckedOperationFrame` of its operation, then S3 refuses …" with a cause and the key reported. Also say what the S3 `missing-name` refusal names; AR-4 names the key and `DomainPackageRef`, but AR-3 does not. | ADR-017 AR-2, AR-3 |
| FND-012 | medium | AR-2 says "QSL checks only that each segment is a Rust identifier" without defining one. Raw identifiers (`r#type`), reserved keywords, non-ASCII identifiers and edition differences decide what a test accepts. A tuple-struct field (`0`) is not an identifier at all, yet "one further identifier" is the field key. Fix: cite the Rust Reference identifier grammar for a named edition. State whether keywords and raw identifiers are admitted. Admit or refuse tuple-field indices explicitly. | ADR-017 AR-2, AR-3 |
| FND-013 | low | PF-3 states the generalized entry rule with no unwanted-behaviour clause. The If/then refusal for a name that resolves to nothing lives only in FR-100, FR-109, FR-115 and FR-116. Fix: add "If the name resolves to no item of the selected kind, then the entry reports stage `select`, `missing_declaration`/`missing-name` (FR-109, FR-115)". That keeps the rule self-contained, which R-06 needs because it cites PF-3 as the exception. | ADR-017 PF-3; ADR-013 R-06 |
| FND-014 | low | The RF-5 verdict clause "any unresolved case is unsupported or incomplete (unsupported first)" is ambiguous. Fix: "else, if any case is unresolved (unsupported), the verdict is unsupported; else, if any case is unresolved (incomplete), it is incomplete". | ADR-017 RF-5 |
| FND-015 | low | The RF-6 tests lack measurable criteria. Determinism's "equal reports" gives no equality or case order. The incomplete control needs "a one-unit stage limit", but RF-3 does not say where a case's stage limits are stored. The #192 fault injection needs a compile seam that §5 does not name. Fix: say "byte-equal reports with cases ordered by `RawSourceRef`". Put the stage limits in the RF-3 case record. Name the injectable compile function in #191's interface in §5. | ADR-017 RF-3, RF-6, §5 |
| FND-016 | low | Several statements are predictions or vague outcomes, not verifiable decisions. PF-2 says "No QSL change is needed when they land". PF-1 says "The landed code conforms", which G-1 to G-4 contradict. RF-7 says "The case's source may need no change". The §4 test cell says "model admission unchanged". Fix: drop them or restate them as checks, for example "TC-510 to TC-515 pass unchanged" in place of "model admission unchanged". Scope PF-1's claim to "except G-1 to G-4". | ADR-017 PF-1, PF-2, RF-7, §4 |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001: the frame
binding keys on `OperationKey`, and parameters, result and framed state have
separate carriers. FND-002: moot; there is no recorded reference. FND-003:
per-case rows are first-match with tool failure first. FND-004:
`CompileRefusal` vectors are never empty; the lead is reworded. FND-005:
If/then statements for each compared member (PF-3, PF-4). FND-006: rejected
with reason: G-4 is not a defect, so there is no TK-4 refusal to name.
FND-007: four entries; declared keys are not names. FND-008: TK-1's one
resolver, refusing `missing-name`. FND-009: referenced elements defined from
checked data (AR-4). FND-010: unknown occurrence key refusal; per-item
returns stated. FND-011: `OperationKey` has no redundant parts; AR-3's
`missing-name` names the key and `DomainPackageRef`. FND-012: Rust Reference
2021 `IDENTIFIER`, raw identifiers and tuple indices. FND-013: PF-3 bullet
2. FND-014: RF-4 verdict table. FND-015: byte-equal reports ordered by
`RawSourceRef`s; limits in the case; the comparison control is a unit test
with no compile seam. FND-016: claims restated as checks.
