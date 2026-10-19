---
id: SR-12817
title: "Gap analysis of QSL-550 hyper and relation forms (PR #971)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6c7915ebf (qsl-550-e14fe-hyper, diff origin/main...HEAD); FR-171, FR-172, FR-173, FR-174 (S3), FR-180 (S3), FR-361-AC-4 against qsl-cst/tests/it/hyper_clause.rs, qsl-forms/tests/it/hyper_clause_forms.rs, qsl-semantics/tests/it/hyper_clauses.rs"
review_set: subset
---

## Summary

Ticket: QSL-550 (PR quire-spec-language#971). Planless gap analysis: the
listed FRs' acceptance criteria compared with the tests tagged to them in the
PR's diff, and the strength of each tagged test's oracle. Plan completion: not
assessed.

The author's declared scope limits are taken as legitimate and are not
defects in themselves: FR-173-AC-4 request writer, FR-174-AC-2..4 (EN-1),
HP-4 V-8 negotiation, the FR-172-AC-1 universe instance, FR-180-AC-1's S4
relation node, and FR-180-AC-2/AC-3 (replay). They depend on unbuilt stages.
The findings below are about tags that report those unbuilt parts as covered,
and about spec text the PR leaves stale.

## Verdict

PASS with one medium finding. Every AC this PR builds has a tagged test with
a real oracle. FR-171-AC-1..3 (CST and form shapes, `NonInterference`,
`SavesPower`, fairness, `align skip`, `Det`, no `UnrepresentedConstruct`),
FR-172-AC-2 (every refusal with code, tag and exact span), FR-172-AC-3,
FR-172-AC-4 (identity distinguishes swapped fairness sets and an extra
conjunct, and equal clauses have equal identities), FR-173-AC-1 and FR-174-AC-1
(both spellings of `=` are swappable; `<=`, one-sided fairness, two aliases
and a one-sided `match` are not) are clean. The problems are tags that
overclaim.

## Examined Scope

```yaml
scope:
  - {id: FR-171-AC-1, path: spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md, role: examined, excerpt: "ADR-023 §1's NonInterference builds a HyperBehavioursForm with one object parameter v: V::Vault, quantifiers forall a of V and forall b of V in order ..."}
  - {id: FR-171-AC-2, path: spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md, role: examined, excerpt: "forall trace a of V fair { weak V::Vault::step } builds a quantifier holding one fairness constraint form; §13's clause with align skip { V::Vault::mix } builds align_skip holding V::Vault::mix."}
  - {id: FR-171-AC-3, path: spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md, role: examined, excerpt: "relation Det ... builds a ModelRelationForm with executions x and y in order and the body's pre reads as written. None of AC-1 to AC-3's clauses refuses UnrepresentedConstruct."}
  - {id: FR-172-AC-1, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: examined, excerpt: "NonInterference checks with one alias V, two universal quantifiers over it, one instance for universe {v}, both mu conjuncts in mu_universal ..."}
  - {id: FR-172-AC-2, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: examined, excerpt: "Refusals at the named span: forall trace a with no of in a behaviours clause (unsupported_construct, the quantifier); ..."}
  - {id: FR-172-AC-3, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: examined, excerpt: "§13's clause with align skip { V::Vault::mix } checks with align_skip holding mix; align skip { }, ... each refuse unsupported_construct/expression-form."}
  - {id: FR-172-AC-4, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: examined, excerpt: "FR-171-AC-3's Det checks with two model execution bindings to step. NonInterference and the same clause with its quantifiers' fairness sets swapped ... have different node identities"}
  - {id: FR-173-AC-1, path: spec/functional/FR-173-classify-hyper-clauses-into-forms.md, role: examined, excerpt: "NonInterference classifies Universal; §8.2's Opaque ForallExistsSafety; Det StepRelation; §13's clause with align skip ProjectionAligned; exists trace b of V {...} SingleExistential."}
  - {id: FR-173-AC-2, path: spec/functional/FR-173-classify-hyper-clauses-into-forms.md, role: examined, excerpt: "Other, each naming its rule: ... Each settles V-8 unsupported-requested-capability in negotiation with no engine run."}
  - {id: FR-173-AC-3, path: spec/functional/FR-173-classify-hyper-clauses-into-forms.md, role: examined, excerpt: "SingleExistential with a match block refuses unsupported_construct/expression-form at the block. NonInterference under the event-position profile records no form and settles V-8."}
  - {id: FR-173-AC-4, path: spec/functional/FR-173-classify-hyper-clauses-into-forms.md, role: examined, excerpt: "A request with SavesPower supplies subjects for S and N and carries two deadlock-freedom items; a request with NonInterference and Opaque over V carries one."}
  - {id: FR-174-AC-1, path: spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md, role: examined, excerpt: "NonInterference has copy group [[a, b]]. The same clause with v.l @ a <= v.l @ b ..., with fair { weak V::Vault::step } on a only, or with SavesPower's two aliases has an empty copy group."}
  - {id: FR-180-AC-1, path: spec/functional/FR-180-hand-over-a-step-relation-code-claim.md, role: examined, excerpt: "Det over a vault whose step is bound to an implementation records a temporal-satisfaction model claim and an operation-contract code claim with different identities; the checked package's relation node holds ..."}
  - {id: FR-361-AC-4, path: spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, role: examined, excerpt: "For Population<T>, allInstances, field navigation and a hyper, relation or state-graph clause, the same unit with the model profile admits the form, and with the value profile the refusal, if any, is FR-110's layer rule ..."}
  - {id: FR-362-AC-7, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "During the migration sequence, a unit that holds a predicate and a relation clause is refused with unsupported_construct at the relation clause ... A hyper clause is refused the same way. Once a family has moved, its constructs are admitted and the test for that family is retired with it."}
bindings:
  - {test_id: the_adr_clauses_bind_their_aliases_parameters_and_match_conjuncts, ac_id: FR-172-AC-1, trace: correct}
  - {test_id: an_ill_formed_clause_refuses_at_the_named_span_with_the_named_code, ac_id: FR-172-AC-2, trace: correct}
  - {test_id: an_align_skip_list_checks_and_its_ill_formed_variants_refuse, ac_id: FR-172-AC-3, trace: correct}
  - {test_id: a_relation_binds_its_executions_and_a_clause_identity_binds_its_meaning, ac_id: FR-172-AC-4, trace: correct}
  - {test_id: the_adr_clauses_classify_into_their_forms, ac_id: FR-173-AC-1, trace: correct}
  - {test_id: a_clause_outside_the_admitted_forms_names_its_rule, ac_id: FR-173-AC-2, trace: correct}
  - {test_id: a_single_existential_clause_admits_no_match_and_another_profile_has_no_form, ac_id: FR-173-AC-3, trace: correct}
  - {test_id: a_clause_names_one_subject_alias_per_distinct_model, ac_id: FR-173-AC-4, trace: wrong}
  - {test_id: a_symmetric_clause_has_a_copy_group_and_an_asymmetric_one_has_none, ac_id: FR-174-AC-1, trace: correct}
  - {test_id: a_bound_step_relation_has_a_code_claim_beside_its_model_claim, ac_id: FR-180-AC-1, trace: correct}
  - {test_id: hyper_clauses_are_ungated_by_the_model_profile, ac_id: FR-361-AC-4, trace: correct}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_clause_names_one_subject_alias_per_distinct_model` is tagged `FR-173-AC-4`, but it only asserts `aliases()` lists. AC-4 is entirely about the request: the subjects supplied and the number of deadlock-freedom items. The author says that part is not built. The matrix reports FR-173-AC-4 as backed when no part of it is. Drop the tag, or tag the test only once the request writer is asserted. | qsl-semantics/tests/it/hyper_clauses.rs:652-668 |
| FND-002 | low | `a_bound_step_relation_has_a_code_claim_beside_its_model_claim` is tagged `FR-180-AC-1`. It asserts only that the two claims' capability kinds differ, not that their identities differ, and the relation node is S4's and unbuilt. The tag reports the AC as covered while most of it is not. | qsl-semantics/tests/it/hyper_clauses.rs:734-779 |
| FND-003 | low | The tags on FR-173-AC-2 and FR-173-AC-3 (V-8 settlement in negotiation, no engine run) and on FR-172-AC-1 ("one instance for universe {v}") cover only the S3 half. Those halves are not asserted, yet the matrix shows the full ACs as backed. | qsl-semantics/tests/it/hyper_clauses.rs:243-299, 568-650 |
| FND-004 | low | The FR-361-AC-4 oracle is vacuous for the relation, and largely so for the hyper body. S3 never examines the relation body, and checks only the trace binding of the hyper body, so neither can reach the layer rule. "No layer refusal" therefore passes whatever the gate does. It would still pass if a later body check gated `pre`/`self`/navigation wrongly. | qsl-semantics/tests/it/hyper_clauses.rs:781-807; qsl-semantics/src/check/hyper_clause.rs:1996-2049 |
| FND-005 | low | The PR deletes the TC-1026 step 5 test, as FR-362-AC-7's retirement clause allows. The AC text still says a `relation` or `hyper` clause is refused `unsupported_construct`, and TC-1026 step 5 still says "S2 has no dispatch entry" for `relation`. The spec now contradicts the code. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:67,96; spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md:21 |

## Coverage

Planless review over the PR diff; tags read from the diff's `#[trace]`
attributes. Plan completion: not assessed. A `quoin matrix` run was not
needed to judge the diff's own tags.
