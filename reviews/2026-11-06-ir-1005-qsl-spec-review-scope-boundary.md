---
id: SR-20261105101081
title: "IR-1005 QSL admission binding scope-boundary review"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; IR-1005; eight-file diff at 41ff656a0b481baab2aa53a8f710c96193832d16"
review_set: subset
---

## Summary

Contract model owns AdmissionBinding issuance and retains validation; QSL owns checked subject forwarding and sealed temporal classification. The owner contract is exercised by the genuine producer/read AC-26 test. Integration owns the subsequent project_tl foreign-package refusal; this PR makes no FR-350/TC-461 completion claim. FR-515 and FR-527 remain QSL core responsibilities.

## Verdict

**PASS** — No defects found in the selected scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

No applicable AssuranceProfile is present in this repository. Direct scoped review; no enforced review workflow is claimed. No build was rerun per dispatch. Existing log /tmp/IR-1005-qsl-binding-tests.log reports 16/16 classification tests pass; all-target compile and Clippy are dispatch-provided evidence. Handoff route, doctests and aggregate final gate remain the lead’s validation responsibility.

## Examined scope

```yaml
scope:
- id: FR-515
  path: spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md
  role: examined
  excerpt: The view's `admission_binding() -> AdmissionBinding` SHALL expose the opaque admitted-package binding
    issued by `CheckedPackageV2`'s owner, without replacing it with a QSL carrier.
- id: FR-527-AC-12
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: Changing `surrounding` changes the target of FR-045's future rows and nothing else.
- id: FR-527-AC-13
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: Every result's `selection` holds the package identity, the clause, and the definition `authority` and
    `identity` (the `/vN` suffix being the revision) equal to the values the test reads from the lock's `profile_selections`
    row, and holds no clock parameter map.
- id: FR-527-AC-14
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: A compile-fail test that constructs `AuthenticatedSelection` outside the classifier fails to compile.
- id: FR-527-AC-15
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: For each temporal clause of each of **R**'s three packages, the reader applied to the unit emitter's
    bytes and the reader applied to the handoff tree's bytes produce equal support, retained subject, package name,
    clause and definition artifact. Each classification belongs only to its own admission and clones retaining that
    admission; their independently issued bindings are unequal.
- id: FR-527-AC-25
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: The signature of `mapping_support` takes no budget, limit or receipt parameter and returns none; `MappingRefusal`
    holds no work-limit variant; over a view built at the FR-515 work-unit default exactly, it returns the same
    classification on a second call, and a view one expansion past that default is refused at view construction
    (`ViewRefusal::FormulaTooLarge`), never by `mapping_support`.
- id: FR-527-AC-26
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: 'For a supported and an unsupported clause, classify the same producer bytes admitted twice independently:
    package names, content, disposition and retained clause facts are equal, but each selection''s owner binding
    accepts its original admission and its unchanged clone, refuses the other admission, and makes the classifications
    unequal. Moving the original package preserves binding membership. Dropping the source package and subject borrow
    does not prevent an owned selection from retaining and comparing its owner binding.'
- id: TC-1322
  path: spec/test-cases/TC-1322-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: Verify FR-527-AC-1 to FR-527-AC-26.
- id: FR-515-AC-31
  path: spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md
  role: context_only
  excerpt: The signatures of `temporal_subject`, `arena_of`, `occurrence_of` and `mapping_support` take no budget,
    limit or receipt parameter and return none.
```
