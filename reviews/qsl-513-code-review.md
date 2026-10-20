---
id: SR-12219
title: "Code review of quire-spec-language PR #1090: abstraction declaration S1/S2 parse, FR-361 gate and S3 relation from source"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@877ba613d4159d5abb2306dc4a73ca6f651010f4; PR #1090: git diff origin/main...HEAD over qsl-cst (grammar, token, tests), qsl-forms (abstraction_declaration, dispatch, value, tests), qsl-semantics/src/check (abstraction, assemble, check, lowering/abstraction, mod, model_gate, node_key/abstraction, refusal, region), qsl-package (checked, emit, emit/tests/abstraction, export/tests), qsl-replay (spine, model_gate tests), spec/functional/FR-304"
review_set: subset
---
# Code review of quire-spec-language PR #1090

## Summary

Ticket: QSL-513 (AR2). Code review with the rust-review lane folded in.
Measured against QSL FR-304, FR-361 and QSpec FR-450/FR-451 and the shared
grammar's `abstraction-decl` production (cited by id only).

What is right: the S1 grammar matches the shared-grammar production
(`abstraction` reserved, `population`/`operation`/`receiver` contextual,
the four Rust positions quoted strings). S2 dispatch is one arm in `dispatch`
and one spelling in `from_spelling`, calling
`abstraction_declaration::declaration`; it is a thin call. The S2 form keeps
names spelled and spanned and each Rust spelling as the bytes between its
quotes. The S3 source path reuses PR #938's `check_abstraction` for key
uniqueness, missing operations and entry checks, so it does not duplicate the
#938 core. The FR-451 node body, its own preimage, canonical order and
`dependencies` are typed (`AbstractionTerm`), and the generic FR-092 `node_key`
refuses an abstraction body. The receiver read is the one `#[string_edge]`.
The Panel fixture change is justified: the fixture's `interface` construct is
built by `tc235_k2::construct` with no declared members (the `_ => json!({})`
arm), so a `featureOrder` member on the declaration is an undeclared member
under `unknownPolicy: reject`; removing it keeps Panel admissible as a systems
interface for FR-304-AC-9.

What is wrong: the source path gives two refusals the wrong cause against
QSpec FR-450's rules 2 and 4, which FR-304 says it SHALL apply with FR-450's
codes and causes.

FND-001 to FND-003 were reproduced with a throwaway probe test in the
qsl-replay model-gate harness (not committed): a field named twice gives
`invalid_model_binding/malformed-declaration` `DuplicateEntry`; `population
Config::ConfigVersion` and `type Config::config_history` each give
`missing_declaration/missing-name` `UnresolvedName`; a conflicting object
binding in declaration `B` is located at `A`. The PR's own model-gate,
emit-abstraction and FR-304-AC-9 export tests pass.

## Verdict

Changes requested: two high findings (FND-001, FND-002). The mediums and lows
do not block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A field named twice in one object binding refuses `invalid_model_binding`/`malformed-declaration` (`MalformedEntry::DuplicateEntry`). QSpec FR-450 rule 4 (and FR-450-AC-6) requires `invalid_model_binding`/`conflicting-binding`, naming both field bindings and the field name; FR-304 "Refusals" adopts FR-450's codes and causes. The parameter-named-twice case is correctly `malformed-declaration`. No test covers either duplicate. | qsl-semantics/src/check/abstraction.rs:490-505; qsl-semantics/src/check/abstraction.rs:290-295 |
| FND-002 | high | A source name that resolves to a declaration of another kind refuses `missing_declaration`/`missing-name`, not `invalid_model_binding`/`malformed-declaration` as FR-450 rule 2 requires. `authored_binding` resolves `type M::T` only against `model_objects` (object types) and `population M::P` only against `model_populations`, so `population Config::ConfigVersion` or `type M::<a population or value type>` become `UnresolvedName` before `check_binding`'s `NotAnObjectType`/`NotAPopulation` arms can run. The in-process path (export test `a_key_naming_a_declaration_of_another_kind_refuses_malformed_declaration`) is correct; the source path is not. | qsl-semantics/src/check/abstraction.rs:468-530 |
| FND-003 | medium | Every refusal except `UnresolvedName` is located at the first declaration's name (`abstraction_location`), so a conflicting or malformed binding in a second declaration points at the first. FR-450 "Outputs" asks for a located refusal per inadmissible binding. Declared as a remainder in the PR body. | qsl-semantics/src/check/abstraction.rs:411-419; qsl-semantics/src/check/mod.rs:3077-3089 |
| FND-004 | medium | Refusals are not in source order of the bindings, as FR-304 "Refusals" requires: every name-resolution refusal (`authored_binding`) is listed first, then every `check_abstraction` refusal, so a malformed binding in declaration 1 is reported after an unresolved name in declaration 2. Not declared in the PR body. | qsl-semantics/src/check/abstraction.rs:436-465 |
| FND-005 | medium | `UnresolvedName` carries only the name and span. FR-450 rule 1 (FR-450-AC-4) names "the name and its owning domain package identity"; the alias `M` resolves to a selected package, whose identity is available but not carried. | qsl-semantics/src/check/abstraction.rs:334-342 |
| FND-006 | low | `declaration_origin` encodes the declaration kind into a string, `Origin::TypeDeclaration { name: "abstraction {name}" }`, relying on the space so it never names a declared type. It works, but it is a stringly-typed origin; a typed origin (or a typed key in `declared_type_spans`) would not depend on a spelling convention. | qsl-semantics/src/check/abstraction.rs:373-377; qsl-semantics/src/check/assemble.rs:3346-3352 |
| FND-007 | low | `authored_binding`'s `object` closure repeats the `model_objects` lookup `CheckedGraph::resolve_operation` already does, and the population key is found by a linear inverse scan of `population_keys` for an equal `PopulationDomain`. | qsl-semantics/src/check/abstraction.rs:478-483; qsl-semantics/src/check/abstraction.rs:515-525 |
| FND-008 | low | Stale or broken doc text: `emit/tests/abstraction.rs` says "no source compile reads an `abstraction` declaration yet"; `mod.rs:500`, `assemble.rs:713` and `check.rs:450` cite `declared_abstraction`, which does not exist (the method is `check_declared_abstraction`); the FR-361 module doc in `model_gate.rs` has an unwrapped overlong line. | qsl-package/src/emit/tests/abstraction.rs:5; qsl-semantics/src/check/mod.rs:500; qsl-semantics/src/check/assemble.rs:713; qsl-semantics/src/check/check.rs:450; qsl-semantics/src/check/model_gate.rs:5-7 |
| FND-009 | low | The new `UndeclaredAlias` check for an `abstraction` declaration's `using` alias has no test. | qsl-semantics/src/check/assemble.rs:2614-2629 |

## Dispositions

Round 1, reviewed at `a84ed52d2535460338f22b07fd11f1f753e9523d` (fix commits
`033c819f6`, `a84ed52d2`); the branch head `4a59d1954` is a merge of main that
touches none of the PR's files.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 033c819f6 |
| FND-002 | fixed | 033c819f6 |
| FND-003 | fixed | 033c819f6 |
| FND-004 | fixed | 033c819f6 |
| FND-005 | fixed | 033c819f6 |
| FND-006 | accepted-no-change | `Origin` is the kernel's closed enum with no abstraction variant; the encoding is one documented function, and `lowering/protocol.rs` uses the same `"protocol {name}"` precedent. |
| FND-007 | fixed | 033c819f6 |
| FND-008 | fixed | 033c819f6 |
| FND-009 | fixed | 033c819f6 |

FND-001: `AbstractionRefusal::ConflictingField { key, field, first, first_span,
second, second_span }` maps to `invalid_model_binding`/`conflicting-binding`; a
parameter bound twice stays `MalformedDeclaration { DuplicateEntry }`. Tested
by `a_field_or_parameter_bound_twice_refuses`.

FND-002: names resolve against `model_names`, a per-alias table built from
`AdmittedModel::record_keys` (every top-level declaration, by artifact id), so
an other-kind name reaches `admit_binding` and refuses `NotAPopulation` /
`NotAnObjectType`. Tested by `a_name_of_another_kind_refuses_malformed_declaration`.

FND-003, FND-004: `check_resolved` takes every binding, resolved or not, in
source order, each tagged with its declaration index; each refusal is located
at its own declaration. Tested by `abstraction_refusals_come_in_source_order`
and the `located == "B"` assertion in `two_abstraction_declarations_make_one_relation`.
Location is per declaration, not per binding, except `UnresolvedName`.

FND-005: `UnresolvedName.package` carries the alias's selection identity;
asserted against `CONFIG_VERSION_PACKAGE_IDENTITY`.

FND-007: the inverse population scan is gone; one `export` lookup serves all
three binding forms.

FND-008: the stale header and the three `declared_abstraction` references are
fixed. The FR-361 module doc in `model_gate.rs` still has one line over 100
columns; cosmetic, not tracked.

FND-009: `an_abstraction_declaration_using_no_profile_refuses_at_the_alias`.
