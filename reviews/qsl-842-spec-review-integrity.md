---
id: SR-12097
title: "integrity review of PR 944 (FR-308, FR-218, FR-381 protocol S2/S3 spec)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@8fdb69ac17e5f7c148c7317db892d127b3299ec7; spec/functional/FR-218-*.md, spec/functional/FR-308-*.md, spec/functional/FR-381-*.md, spec/test-cases/TC-1055..TC-1058, spec/spec.md; context FR-113, FR-326, FR-086, FR-092-AC-18, FR-215, ADR-027 CP/AE/FO-1, QSpec native-diagnostics catalog (by id)"
review_set: subset
---

## Summary

Ticket: QSL-842 (SP-A under QSL-839). Cross-file consistency of the amended FR-308/FR-218, the new FR-381 and
their TCs against FR-113, FR-326, FR-086, FR-092-AC-18, FR-215, ADR-027 and the QSpec diagnostic catalog.
Three high findings: the protocol `over` input is given two different refusal codes for the same read in a
`choice` guard; FR-113's Description still says every construct this PR checks is refused
`not-yet-implemented`; and FR-381 refuses an inverted `within` interval (and `attempts 0`) with a catalog
pair whose meaning does not fit and that differs from FR-326's code for the same interval form.

## Scope

Examined: FR-308-AC-7..AC-12 and Behavior; FR-218 Description, Outputs, Channels, Relationships,
Requirements, Choice and repeat decisions, AC-10..AC-19; FR-381 all sections and AC-1..AC-12; TC-1055..TC-1058;
spec/spec.md rows. Context only: FR-113 (Description, Refusals, AC-5, AC-7), FR-326 interval rule, FR-086
wrong-export, FR-092-AC-18, FR-215, ADR-027 CP-1..CP-5, AE-1, FO-1, PS-6; QSpec FR-052/053/054/055/056/057/058/172/173
and the native diagnostic catalog's `unsupported_construct`, `wrong_snapshot`, `invalid_model_binding`,
`undefined_expression` rows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A `choice` guard that reads the protocol `over` input gets two contradictory refusals: FR-218 Behavior and AC-2 (unchanged; `k` is `Fill`'s `over (k: M::Cell)`) refuse it `ill_typed`/`operator-ineligible` as a model-state read, while the new eligibility table, AC-17 and TC-1056 row 34 refuse the same read `wrong_snapshot`/`wrong-observation` reason `ineligible-kind`. The same name is also model state in a FIFO key (TC-1056 row 12) and in the protocol activation guard (FR-381-AC-4, TC-1057 row 18) but a plain stage-scope `missing-name` read in a retry relation (TC-1057 row 6), with no stated precedence between the model-state rule and the stage table; FR-381 also calls the protocol `activation on each ... when` an "activation guard", which is the table's name for the template's `activate first` stage. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:106-108,230-250,314,329; spec/test-cases/TC-1056:row 34; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:105-119; TC-1057 rows 6, 18 |
| FND-002 | high | FR-113's Description (a dependency of FR-218 and FR-381) still states that every node kind except `sequence`/`attempt`, and channels, relationships, `requires temporal`, captures, `activation on each`, replicated roles, `related by` and `compensate`, are refused `unsupported_construct`/`not-yet-implemented` "because no checker reads it yet". FR-218-AC-1 and AC-10..19 and FR-381-AC-1..12 now require these to check. The spec contradicts itself; the paragraph is also interim history, and FR-113 Dependencies carries roadmap prose ("M-6d deletes that checker once ..."). Partly pre-existing for node kinds; this PR extends the contradiction to relationships, requirements, `related by`, captures and activation. | spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md:31-53,148-151 |
| FND-003 | high | FR-381 refuses `within [31,30]`, `within [0,*]`, `attempts 0` (and `commit-precedes-effect`) with `unsupported_construct`/`declaration-form`. The catalog defines that pair as a recognized form the selected profile prohibits, retaining "the exact selected profile that prohibits it", which none of these cases has. For the same written interval form QSL already refuses `a > b` with `ill_typed`/`type-mismatch` (FR-326; FR-333 also uses `ill_typed`/`type-mismatch` for `a > b`), so one source interval `[31,30]` refuses with different codes in a temporal clause and in a compensation. The commit case has a QSpec precedent (FR-052 uses `declaration-form` for a structural no-progress `repeat`), so the commit pair is the weakest of the three; the interval and zero-count pairs are not supported. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:180-192,202-210,235-236,239; spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md:99-100 |
| FND-004 | medium | FR-218-AC-19 and TC-1056 rows 37-39 give `undefined_expression`/`unproved-exhaustiveness` a `failed` payload of `gap`/`overlap`. The catalog's closed failed-obligation set for that cause is `duplicate-arm`, `unknown-member`, `arm-arity`, `missing-arm`, and it retains a `union-decl` identity; a choice partition fits neither. The code use predates the PR (AC-2); the new payload values are QSL-invented. Needs a QSpec catalog row or a mapping (`missing-arm` for gap, `duplicate-arm` for overlap). | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:255-260,331 |
| FND-005 | low | FR-218 Relationships section cites bare "(FR-056, FR-086)" for the domain package's relationship; FR-056 is QSL's domain-package FR (not QSpec FR-056, which FR-381 cites for compensation) but is not in FR-218's front matter, so the bare id reads ambiguously beside FR-381's "QSpec FR-056". | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:174-175 |

## Verdict

FIX. FND-001..003 are self-contradictions or misuse of catalog meaning that an implementer of QSL-558 cannot
satisfy as written. Clean: FR-218 relationships and requirements resolution rules, `invalid_model_binding`/
`wrong-export` for a non-relationship target (matches FR-086 and the catalog's systems-model-kind payload),
`wrong_snapshot`/`wrong-observation` for an unobservable decision fact (the catalog's "exact prohibited read"
supports it), FR-308 vs QSpec shared-grammar productions (capture, relationship, protocol-requirement,
related, compensation, complete channel), ADR-027 CP-1/CP-4 vs FR-381 commit semantics, FR-092-AC-18 and
FR-215 (no conflict: the checked clause stays in-process), spec.md contains edge and index row for FR-381,
AC numbering continuing each file, TC ids continuing from TC-1054.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The `over` input is model state. A guard, `visible` entry, FIFO key, protocol activation guard, trigger guard, retry relation or recovery predicate that reads it refuses `ill_typed`/`operator-ineligible`; this takes precedence over the stage table, whose `missing-name` applies to binders and captures. The template stage is renamed `trigger-guard`. FR-218-AC-17, FR-381-AC-1/4, TC-1056 row 34 and TC-1057 row 6 agree. |
| FND-002 | fixed | FR-113's Description states that FR-218 and FR-381 check every construct, and its roadmap dependency bullet is deleted. |
| FND-003 | fixed | `attempts 0` and an inverted or open `within` refuse `ill_typed`/`type-mismatch`, as FR-326 refuses `[31,30]`. The commit case keeps `unsupported_construct`/`declaration-form`, retaining the selected protocol profile, on QSpec FR-052's precedent of a structurally prohibited form. |
| FND-004 | fixed | `failed` takes the catalog's `missing-arm` (gap) and `duplicate-arm` (overlap), and the choice's node identity stands in for the `union-decl`. |
| FND-005 | fixed | FR-056 is a front-matter relationship of FR-218. |
