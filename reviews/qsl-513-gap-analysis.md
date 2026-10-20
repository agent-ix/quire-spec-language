---
id: SR-12220
title: "Gap analysis of quire-spec-language PR #1090: FR-304 and FR-361 abstraction declaration from source"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@877ba613d4159d5abb2306dc4a73ca6f651010f4; PR #1090: every #[trace] added or changed in qsl-replay/src/spine/clause/tests/model_gate.rs and qsl-package/src/export/tests.rs, plus the untagged qsl-cst/tests/it/abstraction_declaration.rs, qsl-forms/tests/it/abstraction_declaration_forms.rs, qsl-package/src/emit/tests/abstraction.rs and qsl-semantics/src/check/node_key/abstraction/tests.rs, against FR-304-AC-3, AC-6, AC-7, AC-8, AC-9, FR-361-AC-3, FR-361-AC-6 and QSpec FR-450 rules 1-5 (shared-grammar abstraction-decl)"
review_set: subset
---

## Summary

Ticket: QSL-513 (AR2). Planless. Plan completion: not assessed. Scope is the
PR diff only. PR #938 already carries the in-process S3 core and its TC-797 to
TC-800 tests; this review does not re-audit them.

Trace bindings examined:

- `an_abstraction_declaration_needs_the_model_profile` (qsl-replay
  model_gate): FR-361-AC-3, FR-304-AC-8, TC-1014, TC-811. See FND-002. The
  value-profile refusal (once, `unsupported_construct`/`declaration-form`,
  family `abstraction`, at the declaration's span) is asserted correctly.
- `every_construct_is_refused_once_in_source_order`: FR-361-AC-6, TC-1017.
  See FND-001. The first clause (every family, once, in source order,
  retaining layers, an unused declaration refused) is now asserted for all
  four families, `abstraction` included.
- `two_abstraction_declarations_make_one_relation`: FR-304-AC-7, TC-810. See
  FND-003.
- `an_abstraction_binding_naming_nothing_refuses_at_the_name`: FR-304-AC-3.
  See FND-004.
- `a_systems_interface_bound_as_an_object_type_refuses_malformed_declaration`
  (qsl-package export): FR-304-AC-9. Correct for both clauses (object key and
  frame declaring type), asserting the key and the entry.
- qsl-cst `abstraction_declaration.rs` (no tag): S1 parse and the
  `invalid_syntax` refusals at the offending token (empty body, unquoted
  type path, population path and receiver). Correct; it carries FR-304-AC-8's
  third clause but no tag binds it.
- qsl-forms `abstraction_declaration_forms.rs` (no tag): S2 build. Correct.
- emit `abstraction.rs`: TC-804, FR-306-AC-1/AC-2. Correct; out of the
  named AC scope but examined.

FR-304-AC-6 (Rust-spelling syntax) is a declared remainder and carries no tag
in this PR, which is right.

## Verdict

Changes requested: one high finding (FND-001, a coverage tag on an AC whose
clause the repo itself says is untested). The code-review SR-12219 FND-001
and FND-002 also have no test; FND-005 records that.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `every_construct_is_refused_once_in_source_order` now carries `FR-361-AC-6`, the AC's only tag. AC-6 also requires that "the importing unit's own forms written against the library's import alias are refused". The sibling test `the_selection_belongs_to_the_unit` states that clause is not tested and that "FR-361-AC-6 is not bound". The new tag reports the AC as covered while a clause stays untested: coverage inflation. Either test the import-alias clause or drop the AC tag and keep TC-1017. | qsl-replay/src/spine/clause/tests/model_gate.rs:227-230; qsl-replay/src/spine/clause/tests/model_gate.rs:289-293 |
| FND-002 | medium | FR-304-AC-8 and FR-361-AC-3 require the compiled source to yield "the same `CheckedAbstractionRelation` FR-304-AC-1 gives". The test compiles a different relation (one field, `config_history`, a parameterless `attemptUpdate`) and asserts counts and spellings, not equality with a relation built by `bind_abstraction` from the same bindings. FR-304-AC-8's `invalid_syntax` clause and FR-361-AC-3's "no binding refuses `invalid_syntax` before this check under either profile" are only exercised by the untagged S1 test, under one profile. | qsl-replay/src/spine/clause/tests/model_gate.rs:501-555; qsl-cst/tests/it/abstraction_declaration.rs:220-253 |
| FND-003 | medium | FR-304-AC-7 requires a conflict "with an equal or a different value", "naming both bindings and the key". The test covers only a different value (`store::Other`) and asserts only the catalog pair, not the named bindings or key. | qsl-replay/src/spine/clause/tests/model_gate.rs:557-589 |
| FND-004 | low | The FR-304-AC-3 tag on the source test asserts the code and the location. AC-3 is about "naming the key and its owning `DomainPackageRef` identity", which `UnresolvedName` does not carry (SR-12219 FND-005), and the operation-not-declared clause is not exercised from source. AC-3 is still fully covered by the in-process TC-798 test, so the matrix is not inflated, but this binding claims more than it asserts. | qsl-replay/src/spine/clause/tests/model_gate.rs:591-614 |
| FND-005 | medium | No test drives FR-450 rule 2 (a name of another kind) or rule 4 (a field or parameter named twice) from source. Both paths are wrong (SR-12219 FND-001, FND-002) and no test failed. | qsl-semantics/src/check/abstraction.rs:468-570 |
| FND-006 | low | The new FR-304-AC-9 row's verification column is `Test (FR-304-AC-9)`, which names the AC itself, not a TC as every other row does. | spec/functional/FR-304-check-an-authored-abstraction-relation.md:115 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | Four new tests carry AC tags whose clauses they do not assert: `a_name_of_another_kind_refuses_malformed_declaration` and `a_field_or_parameter_bound_twice_refuses` are tagged FR-304-AC-5 (omitted/undeclared/shared parameters, undeclared field) but test FR-450 rules 2 and 4; `abstraction_refusals_come_in_source_order` is tagged FR-304-AC-7 (cross-declaration relation and conflict) but tests refusal order; `an_abstraction_declaration_using_no_profile_refuses_at_the_alias` is tagged FR-304-AC-8, which says nothing of the `using` alias. Each AC stays covered by its own tests, so nothing is inflated, but the bindings are wrong. FR-304 has no AC for rule 2, rule 4 or source order. | qsl-replay/src/spine/clause/tests/model_gate.rs |

## Dispositions

Round 1, reviewed at `a84ed52d2535460338f22b07fd11f1f753e9523d` (fix commits
`033c819f6`, `a84ed52d2`); the branch head `4a59d1954` is a merge of main that
touches none of the PR's files.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 033c819f6 |
| FND-002 | still-open | The new equality assertion is circular. `authored` is rebuilt from the compiled relation's own keys and values, and those are fed back into `check_abstraction`. A source path that resolved a name to the wrong `DeclarationKey` would still pass. No independently built expected relation (fixture `DeclarationKey`s, literal values) exists. The `invalid_syntax` clauses are still only in the untagged S1 test. Medium; does not block under the majors-only rule. |
| FND-003 | fixed | 033c819f6 |
| FND-004 | fixed | 033c819f6 |
| FND-005 | fixed | 033c819f6 |
| FND-006 | fixed | 033c819f6 |

FND-001: `every_construct_is_refused_once_in_source_order` carries `TC-1017`
only.

FND-003: equal and different values, both bindings' `rust_type` and keys,
the code and cause, and the location at `B` are all asserted.

FND-004: `UnresolvedName.package` and the undeclared-operation
`MissingDeclaration` from source are asserted.

FND-005: rule 2 and rule 4 source tests exist (see FND-007 for their tags).

FND-006: FR-304-AC-9 cites TC-1087. `spec/test-cases/TC-1087-a-systems-interface-is-no-abstraction-target.md`
follows TC-810's layout (frontmatter `verifies` FR-304, Description with
Scope, Test Procedure, Expected Results). TC-1087 is free on main, which ends
at TC-1086. The export test carries `TC-1087`.
