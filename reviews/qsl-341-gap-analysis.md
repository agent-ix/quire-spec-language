---
id: SR-913
title: "QSL-341 gap analysis of PR 543 (FR-106-AC-3, FR-106-AC-8, check 6.5 conformance, QSL-341 ruling)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d0a4b6efe5007b0892ff8a01ba85ba2f22adf815; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/test-cases/TC-464-snapshots-and-invocations-admit.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md (read, context); qsl-semantics/tests/it/state_clauses.rs; qsl-semantics/src/model/observation/document.rs; qsl-semantics/src/value/declaration.rs; ticket QSL-341 ruling comment"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-464
    type: reviews
---
## Summary

Ticket: QSL-341. PR: quire-spec-language#543 at d0a4b6ef. This review checks
the QSL-341 ruling, FR-106 check 6.5's new sentence, FR-106-AC-8's new
sentence and FR-106-AC-3 against the tests.

The ruling's three required tests exist, carry `#[trace("TC-464", "FR-106-AC-8")]`
and pass. The reviewer killed each one with a mutant (SR-912):

- `a_subtype_object_binds_a_supertype_parameter`: a Sub object binds a
  ConfigVersion parameter.
- `an_unrelated_type_object_refuses_a_parameter_with_wrong_value_kind`: an
  unrelated type refuses.
- `a_supertype_object_refuses_a_subtype_parameter_with_wrong_value_kind`: a
  supertype object bound to a Sub parameter refuses.

The test that asserted the bug was replaced:
`a_reference_to_an_admitted_object_of_another_type_refuses_wrong_value_kind`
became `a_field_reference_to_an_admitted_subtype_object_admits`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers check 6.5's refusal half for an object field. FR-106 check 6.5 now says a `reference` value fits `Reference<T>` exactly when its target's type conforms to `T`, otherwise `wrong-value-kind`. The replaced test was the only field-level reference `wrong-value-kind` case, and its replacement asserts admission. Reviewer mutant M2 (`resolve` always admits), run over the full `qsl-semantics --test it` binary, fails only the two parameter tests (242 pass). So a regression on the field path is unobserved: it would surface as an internal fault from `ObjectEnvironment`, not `wrong-value-kind`. Add a field case, for example `child.parent` naming an object of an unrelated type (or a ConfigVersion object in a `Reference<Sub>` field), refusing `invalid_runtime_input`/`wrong-value-kind` at `child`/`parent`. | qsl-semantics/tests/it/state_clauses.rs:4031-4060, spec/functional/FR-106-admit-snapshots-and-invocations.md:225-233 |
| FND-002 | low | `a_field_reference_to_an_admitted_subtype_object_admits` is traced to FR-106-AC-3. AC-3 lists cases that each "fail only" one check and return its code. This test admits. No AC states field-level subtype admission: AC-8's new sentence covers only the `target` parameter. Extend an AC (AC-8, or AC-3 for the refusal case in FND-001) to cover field references, and retag the test. | qsl-semantics/tests/it/state_clauses.rs:4037-4039, spec/functional/FR-106-admit-snapshots-and-invocations.md:308 |
| FND-003 | low | TC-464 was not updated. Its step 5 procedure and expected results still end at the incomplete-snapshot case. They omit the three cases AC-8 now names: a Sub object of a complete `archive` admits, an unrelated type refuses, and a ConfigVersion object for a Sub parameter refuses. The new tests carry TC-464 tags for cases TC-464 does not list. | spec/test-cases/TC-464-snapshots-and-invocations-admit.md:31-61 |

## Verdict

The ruling is implemented and its three required tests have real oracles:
each mutant of the fix is caught. The gaps are around the edges. The field
path's refusal is untested (FND-001, which should get a test in this PR). The
admit test's trace (FND-002) and TC-464's text (FND-003) lag the spec.
