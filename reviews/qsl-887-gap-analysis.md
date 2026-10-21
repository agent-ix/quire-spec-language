---
id: SR-12262
title: "Gap analysis of quire-spec-language PR #1148 (QSL-887) against FR-147, FR-135 and FR-310"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@07c53b5365c12c2a537acfac7b603b20ecce7459; PR #1148 diff origin/main...HEAD; FR-147-AC-1 to AC-13 (TC-552), FR-135-AC-5 and AC-9 (TC-540), FR-310-AC-2 and AC-5 (TC-889); QSpec FR-375 and FR-432 at qspec main 8998f318 as context"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
---
# Gap analysis of quire-spec-language PR #1148 (QSL-887)

## Summary

Ticket: QSL-887. Ticket and PR text were read as data, and every claimed AC
was re-measured against the tests and the code. Plan completion was not
assessed.

- **FR-147-AC-1 (S3 half):** covered. The test checks `AbstractSide::Protocol`,
  the alias-free `[Twice::Done]` and the `node Spec::Twice::a1(self)` row.
  The requirement-record half is not built, but the test carries the full AC
  tag (FND-001).
- **AC-2:** covered. The four vectors are present: `Thrice` is `missing-name`
  at `Thrice`; the list entry `zz` is `missing-name` at its pointer; list plus
  node row is `conflicting-binding` naming the row and the entry; a node row
  under a model subject is `malformed` in both builds.
- **AC-4, AC-5, AC-11, AC-12:** covered with exact-equality oracles.
- **AC-6:** covered strongly. Each vector asserts that it is the only
  refusal, its code and cause, its pointer, the `first` pointer for a
  repetition, and that it has no source region.
- **AC-7:** covered. The list refusal is single and located at `Spec`, and
  the protocol side in a build that omits both features gives two refusals.
- **AC-8:** 9 of 13 classes are exercised. `timeout`, `cattempt`, `cend` and
  a positive `activate` are not (FND-001).
- **AC-13:** covered, with weak oracles (FND-003).
- **FR-135-AC-5:** covered from source. The hand-built test is gone. The
  FR-341 classification is not asserted (FND-002).
- **FR-135-AC-9:** claimed, but no test carries the tag (FND-002).
- **FR-310-AC-2 (protocol half), FR-310-AC-5:** covered in qsl-forms. QSL
  parser vectors for QSpec FR-375-AC-11 are also in qsl-cst.
- **Declared remainder.** These are not gaps of this PR, and the PR body and
  the ticket comment both list them: FR-147-AC-3, AC-9, AC-10, the
  requirement record, dependency-package clauses, binder typing (STD-249),
  FR-136 concrete protocol subject, and FR-137-AC-3. The code-review findings
  SR-12261 FND-003 and FND-004 cover how the build reports two of these.

No stub was found. No code in the diff lacks an owning requirement.

Test run at the reviewed sha: `cargo test -p qsl-semantics --all-features
--test it refinement` gave 58 passed, 0 failed.

## Verdict

No major gap. Two medium tag-honesty findings and three low findings. Fix
them in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two partial ACs carry their full criterion tags. `FR-147-AC-1` is on a test that does not cover the requirement record (S4, unbuilt). `FR-147-AC-8` is on a test that exercises 9 of its 13 classes, with no positive `timeout`, `cattempt`, `cend` or `activate`. The repository's convention for partial coverage is a TC-only tag with a "Part of FR-x-AC-y" doc line, as in `refinement_forms.rs`. Fix: tag both TC-552 only, with "Part of" docs, or add the missing fixtures. `activate` needs only an `on each` clause. | qsl-semantics/tests/it/refinement_protocol.rs:224; qsl-semantics/tests/it/refinement_protocol.rs:818 |
| FND-002 | medium | FR-135-AC-9 is claimed as covered in the PR body, the ticket comment and the module doc, but no test carries the `FR-135-AC-9` tag. Its "no `subject` member" and "classification `unsupported` (FR-341-AC-5)" clauses are not asserted for the list refusal. FR-135-AC-5's and FR-147-AC-7's classification is not asserted either. Fix: tag the list-refusal assertions with `FR-135-AC-9`, assert `subject == None` on the `refinement-internal-list` refusal, and assert the FR-341 `unsupported` classification for both features. | qsl-semantics/tests/it/refinement_protocol.rs:2; qsl-semantics/tests/it/refinement_protocol.rs:570-613; qsl-semantics/tests/it/refinement_protocol.rs:619-663 |
| FND-003 | low | The AC-13 oracles are weak. The order half compares only `abstract_side`, `abstract_internal` and `steps().len()`, not the CheckedRefinement's rows and step targets. The refused-clause half asserts only `!found.is_empty()` and no `Unresolved`. That would also pass if the refinement were never checked, or if it added a refusal of another kind. Fix: compare the steps' targets and the rows. For the refused clause, assert that the refusals equal those of the broken clause checked alone (an independent oracle), and that none is a refinement refusal. | qsl-semantics/tests/it/refinement_protocol.rs:751-782 |
| FND-004 | low | The fixtures drift from FR-147's text. The AC fixture names the finish node `finish` (`Spec::Twice::finish`, `[Twice::finish]`) and puts `Twice` in the abstract package `example/counter-spec`. The tests name it `Done` and declare `Twice` in the unit. Fix: align FR-147's fixture paragraph with what is built (a unit clause importing both models, finish node `Done`), and keep the dependency-package fixture as remaining work. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:244-259; qsl-semantics/tests/it/refinement_protocol.rs:29-44 |
| FND-005 | low | This FR-147 "Abstract side" SHALL has no test and no dedicated code path: "a row naming a protocol binder, queue or node as a field SHALL refuse `invalid_model_binding`/`malformed-declaration`". The diff adds no check for it. By reading, a field row naming `r1` or `ch` falls to the existing field-resolution refusal (not verified by a run). Fix: add a vector (`object Spec::Counter from Impl::Counter { r1 = ... }` under `CasTwice`) and the refusal, or record it as remaining work in FR-147. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:91-94 |

## Dispositions

| FND | Outcome | Disposition |
| --- | --- | --- |
| FND-001 | fixed | The FR-147-AC-1 and AC-8 tests are tagged TC-552 only with "Part of" docs. The `timeout` and `activate` positives are added (`the_timeout_and_activate_classes_resolve`); `cattempt` and `cend` need a record type the counter models do not declare and stay unfixtured. |
| FND-002 | fixed | `the_list_refusal_names_the_feature_the_refinement_and_no_subject` carries FR-135-AC-9 and asserts the profile and no `subject`. The FR-341 classification is asserted in qsl-replay (`an_unbuilt_refinement_feature_is_unsupported_and_a_list_defect_is_refused`) for all three features. |
| FND-003 | fixed | AC-13 compares the steps, objects and populations, and compares the refused clause's refusals with the clause checked alone. |
| FND-004 | fixed | FR-147 and TC-552 name the finish node `Done` and describe the fixture as a unit clause over the imported models; the dependency-package fixture is Remaining work. |
| FND-005 | fixed | A field row naming a protocol binder, node or queue is `malformed-declaration`. Test: `a_field_row_naming_a_protocol_binder_is_refused`. |
