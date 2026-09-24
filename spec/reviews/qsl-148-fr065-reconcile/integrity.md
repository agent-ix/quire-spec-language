---
id: SR-619
title: "Integrity review of the QSL-148 FR-065 reconciliation"
type: SpecReview
analysis: integrity
scope: "Working-tree change on spec/148-fr065-reconcile: FR-065 against ADR-012 §3 and §4.3, ADR-011 §7.3 M-6e, FR-062 and the delivered qsl-semantics checker, and consistency between FR-065's ACs, CONs, Behavior, Status and TC-163, TC-164, TC-376 and TC-377"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-163
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-164
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-376
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-377
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
---

## Summary

The amendment fixes the contradiction it was written for. ADR-012 §4.3 says
there is one `Expression` enum in the `forms` core, and each variant belongs
to the family whose hook its arm calls. The old FR-065-AC-5 required a
function-application variant to be removed from a "composed checker input
form-kind enum". No such enum exists: the composed checker
(`src/checking/composed/`) takes no `qsl-forms` input, and `Expression::Call`
is the `Value`-owned variant that §4.3 keeps. The new text says
`Expression::Call` is a `Value` variant of the one enum, which matches §4.3
and the `Value` row of §3 ("calls, ... function declarations"). FR-065-CON-3
restates §4.3's thin-seam rule and its definition of what is not semantic
logic. The old AC-4 shape test and the old AC-5 symbol-absence and `E0004`
tests are gone. This follows the 2026-09-22 testing-policy ruling.

FR-062 has no rule that a nested call must go through a `FamilyContract`
method, so checking a call through `Application` does not contradict it.

Termination: FR-065's normative text says nothing about it. The Behavior,
CON-2 and AC-7 text names typing and static definedness only. The one
mention is the Status sentence "Termination is a separate whole-package
pass ... after every declaration is checked" (line 288). That sentence
describes the delivered code and does not decide the open owner question.
TC-377 no longer argues that termination cannot move. The argument is still
in `check_declaration_body`'s doc comment (`family.rs:869-886`), which is
code and outside this review.

The findings are about places where the new text and the code disagree, and
about stale text left around the edited paragraphs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "A call therefore receives the same verdict from every entry point" is false for a call whose argument is a clause-sensitive form. The typer descends into each argument using the current clause kind. `Expression::Dispatch` is refused in `ClauseKind::Body` and admitted in `Precondition` (`check.rs:1551-1560`, TC-196 D07 in `dispatch_calls.rs:300-330`). `pre(...)` is legal only in `Postcondition` (`typing.rs:717`). So `f(self.size())` is refused as a declaration body and admitted as a precondition. AC-5's fixtures avoid this case, so the test passes while the Behavior sentence is wrong. Fix: "`Application`'s verdict on a call (callee resolution, arity and parameter types) is the same from every entry point. Each argument is checked under the entry point's clause kind (FR-151, FR-153)." | FR-065:99-106; check.rs:1551-1560; typing.rs:717 |
| FND-002 | medium | The application-check rule "SHALL refuse a name that resolves to no function and no type as `missing-name`" does not match `Application::resolve`. A name that is not a function or type but is a declared model operation is refused `ill_typed`/`operator-ineligible` (`family.rs:768-771`). A name that resolves to a non-tuple type, or to a composite that is not a tuple, is also `operator-ineligible` (`family.rs:747-752`, `:764-767`). A tuple constructor with the wrong arity is `type-mismatch` (`family.rs:753-758`). "It resolves a declared tuple type's constructor by the same rules" does not specify any of these. Fix: add SHALL statements for these cases. A model-operation name, a non-tuple type name and a non-tuple composite are refused `ill_typed`/`operator-ineligible`. A tuple constructor supplies its position types and builds a tuple node of that type. Limit `missing-name` to "a name that is no callable function, no type and no model operation". | FR-065:89-97; family.rs:714-777 |
| FND-003 | medium | FR-065-CON-1 still says "no `infer_form` arm other than those two forms'", meaning a function-declaration arm and a function-application arm. The amended Behavior section says "A function declaration is not an expression and has no `infer_form` arm", and "only the `Call` arm is in scope". Fix: CON-1: "... and no `infer_form` arm other than `Expression::Call`'s." | FR-065:198, :116-120 |
| FND-004 | medium | Status text next to the edited paragraph names code that no longer exists. `mint_declaration_identity` and `mint_call_identity` are not in the tree. `Self::call` (`Typer::call`) was deleted by QSL-148, as TC-164's old Status said. The note on "`infer_form`'s/`Self::call`'s doc in `check.rs`" points to a file that no longer holds `infer_form` (it is in `check/typing.rs`). The Status also quotes "check... exclusively through that contract" as unchanged target text, but the Description now says "exclusively through that contract and `Value`'s own family check code". Fix: rewrite the paragraph at lines 275 to 289. Name the current minting site, which is `check::lowering` keying each function and call after `PackageDeclarations::check` types them (`mod.rs:901-905`). Drop the `Self::call` and `check.rs` references. Quote the amended Description wording. | FR-065:275-289; mod.rs:901-905; FR-065:23-30 |
| FND-005 | medium | The code's doc comment on `infer_form` still describes the old criteria. It says "(FR-065-AC-4)" for the thin-arm rule, which is now FR-065-CON-3. It also says "`Expression::Call` itself is still present in `Expression` (FR-065-AC-5 remains unmet for this reason)" and quotes the old AC-5 text. After this amendment the doc contradicts the spec it cites, and a reader of the code will think AC-5 still asks for the variant to be removed. Fix: in the QSL-148 implementation PR, change the `infer_form` doc (`typing.rs:582-602`) to cite FR-065-CON-3 for the thin arm. Replace the AC-5 paragraph with one sentence: `Expression::Call` is `Value`'s variant of the one `Expression` enum (ADR-012 §4.3). | typing.rs:582-602; FR-065-AC-5, FR-065-CON-3 |
| FND-006 | low | FR-065 says M-6e deletes SEAM-2 "with the `Value` family tickets (#214, #120, #164, #170, #175), the last of which removes the composed checker module". ADR-011 §7.3 M-6e lists the tickets of every family, including `StateModel`, `SumCase`, `TemporalTrace`, `ProtocolClause` and `Relation`, and says "the last one deletes the remainder". The last `Value` ticket does not have to be the last one overall. Fix: "... deletes SEAM-2 one composed family at a time, each in the ticket that lands that family's S3 checker and S4 emission (`Value`: #214, #120, #164, #170, #175). The last family ticket deletes the remainder." Make the same change in the Dependencies bullet. | FR-065:161-167, :223-224; ADR-011:1036 |
| FND-007 | low | FR-065-OQ-1 relies on "the 2026-09-19 owner ruling removes nothing working early" but gives no link, and no other FR-065 text names that ruling. A reader cannot check what it covers. Fix: link the ruling's comment or record. If none is recorded, drop the clause and state only that the question is open. | FR-065:371-378 |

## Resolution

- FND-001: the Behavior section now states that `Application`'s own verdict
  (callee, arity, parameter types) is the same from every entry point, and
  that each argument is checked under the entry point's clause kind (FR-151,
  FR-153).
- FND-002: the Behavior section gives one SHALL per `Application::resolve`
  case, including tuple arity, non-tuple type and model operation
  (`operator-ineligible`), and limits `missing-name` to a name that is no
  callable function, no type and no model operation.
- FND-003: FR-065-CON-1 names `Expression::Call`'s arm.
- FND-004: the Status paragraph now names `check::lowering` keying as the
  minting site and drops the deleted symbols.
- FND-005: the `infer_form` doc comment (`typing.rs`), `check.rs`'s module
  doc and `Application`'s doc now cite FR-065-CON-3, the amended AC-4 and
  AC-5, and ADR-012 §4.3 (comment-only change).
- FND-006: the Behavior text and Dependencies bullet say SEAM-2 is deleted one
  family at a time and the last family ticket deletes the remainder.
- FND-007: FR-065-OQ-1 cites ADR-011 §7.3's M-6a row for the 2026-09-19
  ruling.
