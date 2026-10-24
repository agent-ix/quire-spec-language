---
id: SR-12142
title: "Gap analysis of quire-spec-language PR #964: FR-361 and TC-1013 against the model-profile gate"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ae6dacabdb6bab1f89a670238fa60dc98b41c53b; PR #964: FR-361 (items 1-16, AC-1 to AC-12), TC-1013, with FR-110, FR-304-AC-8 and FR-308 as context, against qsl-semantics/src/check/model_gate.rs, qsl-semantics/src/check/layer.rs, qsl-semantics/src/value/{definition,layer}.rs, qsl-package/src/emit.rs and qsl-replay/src/spine/clause/tests/model_gate.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #964

## Summary

Ticket: QSL-750. Plan completion: not assessed. Planless, read-only and
static, over the PR diff. Each requirement-to-test check was done by reading
FR-361, TC-1013 and the tagged tests.

Per item of FR-361:

- Item 1 (header only, no inference, no fallback): implemented. AC-7 is
  tested with three alias names.
- Items 2 to 4 (one resolution path, the lock, the closure): implemented.
  AC-8 compares the emitted identity set. AC-10 is a QSpec conformance
  check, and its 18 rows match QSpec's lock by id.
- Item 5 families: `operation-body`, `dispatch-call` and `lookup` are
  implemented. `abstraction` has no S2 form anywhere in the workspace, so
  AC-3 is correctly left unbound.
- Items 6 and 8 (the `model` declaration and protocol/temporal forms are not
  gated): the code matches FR-361 as merged. The planner ruling on the ticket
  differs, but FR-361 is the authority, so this is not a defect.
- Item 7 / AC-4: `Population<T>`, `allInstances` and navigation stay
  ungated (an untagged test). `hyper`, `relation` and state-graph clause
  forms do not exist, so AC-4 is correctly left unbound.
- Items 11 to 13: implemented for function, state-clause and operation-body
  sites. The `ExpressionForm` sites are not walked. That is recorded as
  SR-12141 FND-002 (item 13), not here.
- Item 14 (per unit): tested in both directions.
- Item 12 / AC-12: tested in both halves.
- The FR-110 value.complete `ObjectNavigation` exception is untouched, as
  STD-161 owns it. Its "removed when QSL-750 lands" wording is now stale,
  and that is STD-161's to revise.

Trace tags: none binds AC-3 or AC-4. AC-1 (TC-1012), AC-5 (TC-1016), AC-7,
AC-8, AC-9, AC-10 and AC-12 (TC-1017) are correct. AC-2, AC-6 and AC-11 are
partial; see below.

## Verdict

No high gaps against FR-361. FND-001 to FND-003 are mediums, recorded under
the review cap. FND-004 is a low. The PR's one blocking defect is SR-12141
FND-001 (the test suite goes red).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-361-AC-2 / TC-1013 expected result 3 says "the call resolves to the receiver's effective operation and the lookup reads the population binding". The test only checks that both units compile under the model profile. It asserts neither the resolved operation nor the lookup's population operand, and it never asserts the lookup refusal's `expression-form` code. | qsl-replay/src/spine/clause/tests/model_gate.rs:177-191 |
| FND-002 | medium | FR-361-AC-6 says "the importing unit's own forms written against the library's import alias are refused". `the_selection_belongs_to_the_unit` writes the importer's `body` against its own `Config` model, never against the import alias `l`, so that clause has no test. | qsl-replay/src/spine/clause/tests/model_gate.rs:271-308 |
| FND-003 | medium | `#[trace("FR-361-AC-11")]` and `#[trace("FR-361-AC-6")]` bind whole ACs that name all four families ("all four families", "every family of item 5"). The tests cover three, because `abstraction` has no form. This is the same partial coverage for which AC-3 was left unbound, so the two tags overclaim. | qsl-replay/src/spine/clause/tests/model_gate.rs:213; qsl-replay/src/spine/clause/tests/model_gate.rs:388 |
| FND-004 | low | The FR-361 Dependencies line "FR-102 and FR-302 record that the `operation-body` gate is not enforced" is stale. This PR deletes those sentences. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:220-221 |

## Dispositions

Round 1, reviewed at `3728ed91f0da4d8a0f53b600b81aeec0f2e2cade` (fix commits `dd535b3bd`, `3728ed91f`).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Medium, recorded under the review cap. It is partly addressed in dd535b3bd: the lookup refusal's `expression-form` code is now asserted, and so is the dispatch table under the model profile. The test still does not assert that the lookup reads `p` or which operation the call resolves to. |
| FND-002 | accepted-no-change | Medium, recorded under the review cap. In dd535b3bd FR-361-AC-6 is unbound, and the test comment states that the import-alias clause is untested, so nothing overclaims. The clause still has no test. |
| FND-003 | fixed | dd535b3bd: `every_construct_is_refused_once_in_source_order`, `the_selection_belongs_to_the_unit` and `the_refusals_are_s3s_not_s2s` are now tagged `TC-1017` only. FR-361-AC-6 and AC-11 are unbound, as AC-3 is. |
| FND-004 | fixed | dd535b3bd: the FR-361 Dependencies line now reads "FR-102 and FR-302 state the `operation-body` form and its dispatch linking, which this requirement gates." |

FR-361-AC-13 (new, from STD-161): `object_navigation_is_not_a_value_layer_form` carries `#[trace("FR-361-AC-13", "TC-1015")]` and covers every clause of the AC, so the binding is correct. Item 17 (a) to (c) is implemented. No new findings.
