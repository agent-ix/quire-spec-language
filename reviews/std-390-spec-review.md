---
id: SR-16211
title: "spec-review of the QSL side of a role on a whole model (quire-spec-language PR 1337)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2190c2da; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md; spec/test-cases/TC-1061-s4-emits-each-protocol-construct-as-its-v2-protocol-node.md; spec/test-cases/TC-1065-a-channel-s-loss-selector-and-the-admission-rules-emission-needs.md; spec/test-cases/TC-650-protocol-subject-and-state-key.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: references
---

# Spec review of the QSL side of a role on a whole model (quire-spec-language PR 1337)

## Summary

Ticket: STD-390 (spec gap for QSL-914), the QSL side of QSpec #389 (STD-331).
One spec-review pass over `git diff origin/main...2190c2da`. Oracles: QSpec
#389 at 57313733 (FR-465 role on a whole model, FR-465-AC-9, FR-425-AC-13,
TC-375 PK-17 and PK-18), QSpec FR-361, and QSL main's FR-218 "Admission for
emission" rule.

Gates: `make check-spec-validation`, run with `QSPEC_DIR` at QSpec origin/main
merged with #389 (57313733) and #390 (15f862d6), exits 0. It reports no
warning on any file this PR touches. origin/main is an ancestor of the head.

Examined, clean:

- Consistent with QSpec #389. S3 admits `role w on M` for any `over` type and
  beside roles on other aliases. S4 writes the `model_selections` identity
  text with no model node and an empty `replication` (FR-465-AC-9). FR-205
  binds the role by construction from the whole-package model subject, and
  its `roles` entry is QSpec's `model-binding` shape. A supplied binding
  refuses `wrong-role-mapping`. Nothing is narrowed: no `over` type and no
  second alias is refused.
- FR-205-AC-1 and TC-650 step 1 now use `role v on M::Cell` for the unbound
  case, which resolves QSL #1261's FND-002 contradiction. FR-205-AC-9 and TC-650
  step 9 cover the two-alias subject.
- FR-383-AC-23 and TC-1061 row 10 cover one alias, two aliases and the key
  distinction from `role v on M::Cell`. FR-218-AC-24 and TC-1065 row 8 cover
  every `over` kind.
- Ids. FR-383-AC-23, FR-205-AC-9 and TC-1061 row 10 are not added by any
  other open QSL PR. #1307 takes FR-383-AC-22 and row 9.
- No follow-up, deferral or narrowing is left. Lead defaults are labelled.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [major] The widened role rule also admits a replicated role on a model alias, which S4 cannot emit. FR-218's role rule now accepts a role "whose model is ... a model alias", and that covers `role r each M from pop max n lifetime L`. The `pop` rule then compares the population with "the domain package that declares `T`" and its element type with `T`, and neither is defined when `T` is an alias. QSpec FR-465 step 1 refuses a text `model` with a non-empty `replication` (`invalid_model_binding`/`malformed-declaration`), and "Admission for emission" says each construct the QSpec contract refuses is refused before S4. So the construct either checks at S3 and is then refused by QSL's own I2 reader, or it fails in a way the spec does not state. Fix: in "Role on a model alias", say that a replicated role on a model alias refuses `invalid_model_binding`/`malformed-declaration` at the alias, and add it to FR-218-AC-24 and TC-1065 row 8. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:165-168, :176-185, :395-400 | missing-requirement |

## Verdict

Not mergeable yet. Medium findings block. The PR matches QSpec #389 and
narrows nothing. FND-001, the missing S3 refusal for a replicated role on a
model alias, is a one-sentence rule plus one AC and TC row. Merge it after
QSpec #389.
