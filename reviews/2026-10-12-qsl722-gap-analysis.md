---
id: SR-7860
title: "QSL-722 PR 790 gap analysis: TC-235 M04, M06, M08, M09"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@dcc72d4cb537b0d3b2b07c353d913e7b60015830; TC-235 M04, M06, M08, M09; qsl-semantics/tests/it/tc235_business_meanings.rs, qsl-semantics/tests/it/tc235_state_machine.rs"
review_set: subset
---

## Summary

Plan completion: not assessed

Ticket: QSL-722. This analysis checks each run of rows M04, M06, M08 and M09
against the test that backs it, at the PR head. The spec was read at the
quire-specification main branch; the TC-235 rows are unchanged on the current
`origin/main`.

## Verdict

CONDITIONAL. Most runs are backed by typed assertions: M04 runs 1, 2, 4 and 5
(typed node and charge count); M04 run 3 (the exact typed cause); M08 run 5
and M09 run 5 (typed node, pointer and absent node); and M09 runs 3 and 4
(typed node). Two kinds of gap remain. Several refusal runs never assert the
charge count that the row states. M06 run 4's binding claim is asserted
through records only.

Status of the three not-built items:

- **M06 run 7: `self.placed` bound as the postcondition of `place`.** This
  is tracked by QSL-726 ("Model operations: intake keeps pre/post clauses
  and body", state Specced). The TC row conflicts with FR-103 (domain-package
  `pre`/`post` text "SHALL NOT be kept"), so the spec has to settle that
  first. It is legitimately out of this PR's reach and is not a finding here.
  The test does correctly assert admission and "no added declaration" (21
  charges).
- **Read-time charge counts.** The TC rows state these, and this PR can meet
  them now without new production code. See FND-001.
- **A typed member-path field on `IntakeMalformedDeclaration`.** FR-154
  requires a refusal to carry the IR node identity and `origin`. The member
  path appears in the TC wording and in FR-154's member-path ordering rule,
  but no FR clause requires it as a typed field on the refusal. Adding one is
  not a spec requirement for this PR. Asserting the locus somehow is required,
  though; see SR-7859 FND-002.

## Examined Scope

```yaml
scope:
  - {id: TC-235-M04, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 3, 21 charges: refused { code: missing_declaration, cause: missing-name } naming ix://test/orders/OrderPlaced, member path TypeDefinition.occurrenceField, absent node ix://test/orders/OrderPlaced/at"}
  - {id: TC-235-M06, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 4: admitted after 22 charges, with Extra bound as an invariant and NotYetPlaced bound as the guard of [0] and as no invariant; ... run 7: admitted after 21 charges, with self.placed bound as the postcondition of place and no added declaration"}
  - {id: TC-235-M08, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 1, 22 charges ... run 2, 19 charges: malformed-declaration naming ix://test/orders/OrderStore at TypeDefinition.operations; run 3, 21 charges ... run 6, 21 charges: unsupported_construct/declaration-form naming OrderStore/save at Operation.pre; run 7, 22 charges"}
  - {id: TC-235-M09, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 1, 22 charges: malformed-declaration naming OrderManagement/name at TypeDefinition.fields[0]; ... run 5, 21 charges: missing_declaration/missing-name naming OrderManagement at TypeDefinition.members[3], absent node Ghost; run 6, 22 charges"}
  - {id: FR-208-AC-4, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: context_only, excerpt: "an occurrence field naming no field is missing_declaration/missing-name."}
  - {id: FR-208-AC-7, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "A field, no operations, an empty persists or a persists entry that is not an object type is invalid_model_binding/malformed-declaration; an entry naming nothing is missing_declaration/missing-name; an operation contract or a quire clause is unsupported_construct/declaration-form."}
  - {id: FR-208-AC-8, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "A field, an operation, a namespace member, or a member already in an earlier namespace is invalid_model_binding/malformed-declaration; a member naming nothing is missing_declaration/missing-name; a quire clause is unsupported_construct/declaration-form."}
  - {id: FR-208-AC-5, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "A type definition whose construct names quire.meaning.model.state-machine/v1 binds its states and transitions and no firing, current or initial state."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Several refusal runs never assert the charge count their row states: M04 run 3, M08 runs 1-7 and M09 runs 1-6 (for example M08 run 2 says "19 charges"). `read_refusals` goes through `read_records(...).expect_err` with no meter. `normalized_refusals` (`read_k2` plus `normalize_with_meter`) already returns retained intake refusals together with a charge count; M06 run 1 uses it for intake-phase refusals at 19 charges. Switching the M08/M09 helpers and M04 run 3 to `normalized_refusals` would back every stated count. | qsl-semantics/tests/it/tc235_business_meanings.rs:548-566; qsl-semantics/tests/it/tc235_business_meanings.rs:1292-1436; qsl-semantics/tests/it/tc235_business_meanings.rs:1493-1508 |
| FND-002 | low | M06 run 4 requires `Extra` bound as an invariant and `NotYetPlaced` bound as no invariant. The test only checks that a `Clause` record for `Extra` exists with owner `OrderLifecycle` and that `transitions[0].guard == "NotYetPlaced"`. The invariant-versus-guard binding the row names is not asserted. | qsl-semantics/tests/it/tc235_state_machine.rs:148-173 |
