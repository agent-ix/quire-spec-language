---
id: SR-510
title: "integrity review of PR #354 (FR-091 OQ rulings)"
type: SpecReview
analysis: integrity
scope: "git diff origin/main...HEAD at a0e00cf9: ADR-011, ADR-012, ADR-013, FR-062, FR-091, spec.md, TC-259, TC-392 to TC-396, TC-400, TC-401, TC-405, TC-406, TC-412, tests.md"
review_set: subset
evaluated_revision: "a0e00cf90cadc3cb5bd5cccec787aaa557105932"
review_date: "2026-09-22"
---

## Summary

Integrity analysis of the PR #354 diff. It covers consistency between the
amended ADR-011, ADR-012 and ADR-013 text and the other ADR and FR text, and
completeness against QSpec at `2449ceb`, which is cited by reference and was
read with `git show`. These claims were checked and hold:

- The catalog codes and causes exist in `native-diagnostics.md` revision
  `1-draft.6`: `missing-name` and `missing-selection` under
  `missing_declaration`; `forbidden-pre-read` under `wrong_snapshot`;
  `type-mismatch` under `ill_typed`; `unsupported-feature` under
  `unknown_required_feature`; `definition-cycle` under `invalid_package`;
  `nesting-depth-exceeded` under `stage_limit_exceeded`.
- FR-145 makes `map`/`collect` one operation.
- FR-148 reads an omitted rounding spelling as `exact`.
- FR-322 makes types authoritative for the rounding mode, and its fixtures tag
  enum, dimension and unit as `scalar_type`.
- `shared-grammar.md` has an optional `[rounding-mode]`, and `using` has no
  default.
- The code claims hold at the tree: `DEFAULT_PACKAGE_IDENTITY`
  (`src/check/family.rs:96`), `ValueType::Float(IeeeWidth)` in both crates, the
  hard-coded `RoundingMode::Exact`, the required `[mode]` in `qsl-cst`, no
  `stage_limit_exceeded` in `Code`, `CheckCause::DefinitionCycle`, and the
  `forms`-core `Deref`/`Pre`/`AllInstances` variants.

The main defect is the owner-scope reading that the diff attributes to QSpec.

## Verdict

**Changes needed.** FND-001 is high. It records an inaccurate claim about QSpec
as the reason for a ruling, and it leaves an unrecorded QSpec dependency.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The diff says QSpec's `proposals/checked-package-v2/README.md` "publishes" owner scope for record, tuple and function nodes. It does not. The README's owner sentence covers only "the nominal scalar node forms covered by the node-preimage schema" (README:71-81), and the schema puts `owner` on `EnumDeclaration`, `Dimension` and `Unit` only. Record/tuple (`composite_type`) and `function` nodes use the `quire.application-node/v1` preimage `{version, node_tag, semantic_form, semantic_type, declaration: {qualified_name}, recursion, body}`, which has no owner (README:201-220). Under QSpec as published, AC-18's (`a`, `w`) run gives the same keys, not different ones. The ruling stands. What is wrong is its recorded basis: QSL's owner-scoped composite/function key needs a QSpec application-node preimage extension, and QC-18 is still an open QSpec contract item. Fix: in ADR-013 O-04, QC-18, ADR-012 §2, the ADR-011 E3 cell, FR-091 (Description, Node key owner, OQ-3 Reason) and the PR body, say that QSpec publishes the owner subject for nominal enum/dimension/unit nodes. Say that the owner member on `composite_type`/`function` application-node preimages is a proposed QSpec extension. Add a Dependencies bullet and proposed QSpec text, as was done for the alias-cycle row. | ADR-013:178, 992; ADR-012:210; ADR-011:356; FR-091:87-88, 334-339, 446, 514; QSpec checked-package-v2/README.md:71-81, 201-220; node-identity-preimage.schema.json:5-11, 60-64 |
| FND-002 | medium | ADR-013 O-04 and QC-18 say "a source- or definition-owned node's owner is `SourceOwner{authority, identity}`". QSpec has two distinct owner kinds: `SourceOwner` (`kind: "source"`) and `DefinitionOwner` (`kind: "definition"`). The `kind` member is part of the preimage, so equal authority and identity under the two kinds give different ids. Fix: write "`SourceOwner{kind: source, authority, identity}` for a source-owned node and `DefinitionOwner{kind: definition, authority, identity}` for a definition-owned node" in O-04, QC-18 and ADR-012 §2. | ADR-013:178, 992; ADR-012:210; node-identity-preimage.schema.json:18-21 |
| FND-003 | medium | The amendment to package scope is incomplete. Untouched text still states the old rule, and it now contradicts O-04: ADR-013:202 ("`NodeKey`, unique across packages (O-04)"); ADR-013:326 ("equal type node ids in two packages mean the same type declaration (O-04 package scope)"); ADR-013:808 T-3 ("Under the O-04 package scope a bare `NodeKey` is unique across packages"); ADR-013:1060 TK-08 ("preimage package scope (QC-18)"); FR-088-AC-7 ("per the O-04 package-scoped preimage"); TC-259's title, heading and description (lines 3, 9, 15); and tests.md:145. TC-259 step 1 also does not require the two packages to have different owners, which the new step-3 reasoning depends on. Fix: reword each to owner scope ("unique across owners"). In TC-259 step 1, author the two packages under two distinct `SourceOwner`s. See failure-domain FND-001 for the uniqueness property that these rows assert. | ADR-013:202, 326, 808, 1060; FR-088:196; TC-259:3, 9, 15, 29-31; tests.md:145 |
| FND-004 | medium | FR-091-CON-1 and Inputs add "the unit's `SourceOwner`" as an E3 input, and say it is among the inputs that ADR-011 §2.1's E3 row admits. That row (ADR-011:255) was not amended. It admits parsed forms, I1 domain packages, I2 import views and the library lock only. Nothing in E2's output carries a source identity either. Fix: amend the ADR-011 §2.1 E3 "Admitted input" cell to include the unit's source owner and name its producer, or have S2 carry it on the parsed unit and amend the E2 row to match. | ADR-011:254-255; FR-091:101-103, 422 |
| FND-005 | medium | There are two syntax authorities. FR-091:93-94 keeps "`qsl-cst/src/grammar.rs` is the authority for the syntax this requirement maps". FR-091:197 and AC-19 say `qsl-cst` accepts `Float32`/`Float64` with no mode, but grammar.rs requires `[mode]` (`qsl-cst/src/grammar.rs:368-373`). The PR body lists this change as having no ticket. Fix: name QSpec `shared-grammar.md` as the authority that grammar.rs transcribes. Then state the bare-float admission once, as a grammar fact with its owner, and ticket the `qsl-cst` change. | FR-091:93-94, 197-200, 447; qsl-cst/src/grammar.rs:368-373 |
| FND-006 | medium | The floating-type refusal maps to `unknown_required_feature`/`unsupported-feature`. The catalog says that this cause must "retain the feature and selected consumer/profile scope", but the FR's error names only the rounding mode and the type form's span. AC-19 and TC-405 also do not check the code or cause, and AC-21 checks the code only. Fix: add the retained feature and the consumer/profile scope to the floating-type error in FR-091:348-352. Assert `unknown_required_feature`/`unsupported-feature` and those payload members in AC-19 and TC-405. | FR-091:348-352, 400, 447, 449; native-diagnostics.md:78 |
| FND-007 | low | The Catalog codes intro says the codes "come from the catalog revision `1-draft.6`". The FR's own Dependencies bullet says that `1-draft.6` scopes `definition-cycle` to definition dependencies and FR-151 dispatch cycles, and has no row for a type alias. The table row gives no sign of this. Fix: mark the alias-cycle row as depending on the proposed catalog sentence, with a pointer to the Dependencies bullet. | FR-091:388-390, 402, 478-482 |
